//! The fluid dust on the GPU (the experiment): compute passes on a wrap-around 3D grid and the volume render.
//! The algorithm is `game/fluid.rs` (host-tested); the shaders are `shaders/fluid.wgsl` and the end of `shaders/raster.wgsl`.
use bytemuck::{Pod, Zeroable};
use crate::game::fluid::{self, FluidCfg, Packed, Window};
use crate::game::terrain::Terrain;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FUni {
    start: [i32; 4], prev: [i32; 4], g: [f32; 4], wind: [f32; 4], car: [f32; 4], car_half: [f32; 4], car_vel: [f32; 4],
    wpos: [[f32; 4]; 4], wvel: [[f32; 4]; 4], wcol: [[f32; 4]; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct VUni { start: [i32; 4], dims: [i32; 4], g: [f32; 4], scr: [f32; 4] }

const STEP: f32 = 1.0 / 30.0;
const TEX_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::from_bits_truncate(wgpu::TextureUsages::TEXTURE_BINDING.bits() | wgpu::TextureUsages::STORAGE_BINDING.bits());

pub struct FluidGpu {
    cfg: FluidCfg,
    n: [u32; 3],
    solid: [wgpu::TextureView; 2],
    vel: [wgpu::TextureView; 2], dens: [wgpu::TextureView; 2], pres: [wgpu::TextureView; 2], div: [wgpu::TextureView; 2],
    height_tex: wgpu::Texture, height_view: wgpu::TextureView,
    smp_clamp: wgpu::Sampler, smp_repeat: wgpu::Sampler,
    fbuf: wgpu::Buffer, vbuf: wgpu::Buffer,
    c_bgl: wgpu::BindGroupLayout,
    pl_solids: wgpu::ComputePipeline, pl_advect_vel: wgpu::ComputePipeline, pl_div: wgpu::ComputePipeline, pl_jacobi: wgpu::ComputePipeline, pl_project: wgpu::ComputePipeline, pl_dens: wgpu::ComputePipeline,
    v_bgl: wgpu::BindGroupLayout, pl_volume: wgpu::RenderPipeline,
    comp_bgl: wgpu::BindGroupLayout, pl_comp: wgpu::RenderPipeline, smp_lin: wgpu::Sampler,
    half: Option<((u32, u32), wgpu::TextureView)>,
    window: Window, have_window: bool,
    /// The ground height of every column of the window (a torus like the grid) and the cell column each texel holds.
    heights: Vec<f32>, height_cells: Vec<[i32; 2]>,
    dp: usize,
    acc: f32, last_time: f32, time: f32,
}

fn tex3d(dev: &wgpu::Device, label: &str, n: [u32; 3], format: wgpu::TextureFormat) -> wgpu::TextureView {
    dev.create_texture(&wgpu::TextureDescriptor { label: Some(label), size: wgpu::Extent3d { width: n[0], height: n[1], depth_or_array_layers: n[2] }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D3, format, usage: TEX_USAGE, view_formats: &[] }).create_view(&Default::default())
}

impl FluidGpu {
    /// `None` when the device cannot do it (the formats must be usable as storage textures).
    pub fn new(adapter: &wgpu::Adapter, dev: &wgpu::Device, queue: &wgpu::Queue, bgl_main: &wgpu::BindGroupLayout, raster: &wgpu::ShaderModule, surface_format: wgpu::TextureFormat) -> Option<FluidGpu> {
        let cfg = fluid::cfg();
        for f in [wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::R32Float] {
            if !adapter.get_texture_format_features(f).allowed_usages.contains(wgpu::TextureUsages::STORAGE_BINDING) { log::warn!("fluid dust: {f:?} cannot be a storage texture here"); return None; }
        }
        let n = [cfg.n[0] as u32, cfg.n[1] as u32, cfg.n[2] as u32];
        let vel = [tex3d(dev, "fvel0", n, wgpu::TextureFormat::Rgba16Float), tex3d(dev, "fvel1", n, wgpu::TextureFormat::Rgba16Float)];
        let dens = [tex3d(dev, "fdens0", n, wgpu::TextureFormat::Rgba16Float), tex3d(dev, "fdens1", n, wgpu::TextureFormat::Rgba16Float)];
        let pres = [tex3d(dev, "fpres0", n, wgpu::TextureFormat::R32Float), tex3d(dev, "fpres1", n, wgpu::TextureFormat::R32Float)];
        let solid = [tex3d(dev, "fsolid0", n, wgpu::TextureFormat::Rgba16Float), tex3d(dev, "fsolid1", n, wgpu::TextureFormat::Rgba16Float)];
        let div = [tex3d(dev, "fdiv0", n, wgpu::TextureFormat::R32Float), tex3d(dev, "fdiv1", n, wgpu::TextureFormat::R32Float)];
        let height_tex = dev.create_texture(&wgpu::TextureDescriptor { label: Some("fheight"), size: wgpu::Extent3d { width: n[0], height: n[2], depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::R32Float, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        let height_view = height_tex.create_view(&Default::default());
        let smp_clamp = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("fsmp"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat, address_mode_w: wgpu::AddressMode::Repeat, ..Default::default() });
        let smp_repeat = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("vsmp"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat, address_mode_w: wgpu::AddressMode::Repeat, ..Default::default() });
        let fbuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("fparams"), size: std::mem::size_of::<FUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let vbuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("vparams"), size: std::mem::size_of::<VUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let tex = |binding: u32, filterable: bool| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None };
        let store = |binding: u32, format: wgpu::TextureFormat| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format, view_dimension: wgpu::TextureViewDimension::D3 }, count: None };
        let c_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("fluid_c"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            tex(1, true), store(2, wgpu::TextureFormat::Rgba16Float), tex(3, true), store(4, wgpu::TextureFormat::Rgba16Float),
            tex(5, false), store(6, wgpu::TextureFormat::R32Float), tex(7, false), store(8, wgpu::TextureFormat::R32Float),
            wgpu::BindGroupLayoutEntry { binding: 9, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 10, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            tex(11, true), store(12, wgpu::TextureFormat::Rgba16Float),
        ] });
        let cmod = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fluid"), source: wgpu::ShaderSource::Wgsl(crate::shaders::fluid().into()) });
        let c_lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&c_bgl], push_constant_ranges: &[] });
        let cp = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some(entry), layout: Some(&c_lay), module: &cmod, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
        let (pl_solids, pl_advect_vel, pl_div, pl_jacobi, pl_project, pl_dens) = (cp("solids"), cp("advect_vel"), cp("divergence"), cp("jacobi"), cp("project"), cp("advect_dens"));

        let v_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("fluid_v"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
        ] });
        let comp_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("fluid_comp"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ] });
        let smp_lin = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("fcomp_smp"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let r_lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[bgl_main, &v_bgl], push_constant_ranges: &[] });
        let premul = wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add } };
        let targets = [Some(wgpu::ColorTargetState { format: surface_format, blend: Some(premul), write_mask: wgpu::ColorWrites::ALL })];
        let half_targets = [Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })];
        let comp_lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[bgl_main, &comp_bgl], push_constant_ranges: &[] });
        let pl_comp = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vcomp"), layout: Some(&comp_lay),
            vertex: wgpu::VertexState { module: raster, entry_point: Some("vs_vcomp"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: raster, entry_point: Some("fs_vcomp"), compilation_options: Default::default(), targets: &targets }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        let pl_volume = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volume"), layout: Some(&r_lay),
            vertex: wgpu::VertexState { module: raster, entry_point: Some("vs_volume"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: raster, entry_point: Some("fs_volume"), compilation_options: Default::default(), targets: &half_targets }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        let _ = queue;
        log::info!("fluid dust: {}x{}x{} cells of {} m", n[0], n[1], n[2], cfg.cell);
        let cells = (n[0] * n[2]) as usize;
        Some(FluidGpu { cfg, n, solid, vel, dens, pres, div, height_tex, height_view, smp_clamp, smp_repeat, fbuf, vbuf, c_bgl, pl_solids, pl_advect_vel, pl_div, pl_jacobi, pl_project, pl_dens, v_bgl, pl_volume, comp_bgl, pl_comp, smp_lin, half: None,
            window: Window { start: [0; 3], n: cfg.n }, have_window: false, heights: vec![0.0; cells], height_cells: vec![[i32::MIN; 2]; cells], dp: 0, acc: 0.0, last_time: 0.0, time: 0.0 })
    }

    fn bind(&self, dev: &wgpu::Device, vel: [usize; 2], dens: [usize; 2], pres: [usize; 2], div: [usize; 2], solid: [usize; 2]) -> wgpu::BindGroup {
        fn e<'a>(binding: u32, v: &'a wgpu::TextureView) -> wgpu::BindGroupEntry<'a> { wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(v) } }
        dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("fluid_bg"), layout: &self.c_bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: self.fbuf.as_entire_binding() },
            e(1, &self.vel[vel[0]]), e(2, &self.vel[vel[1]]), e(3, &self.dens[dens[0]]), e(4, &self.dens[dens[1]]),
            e(5, &self.pres[pres[0]]), e(6, &self.pres[pres[1]]), e(7, &self.div[div[0]]), e(8, &self.div[div[1]]),
            e(9, &self.height_view), wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::Sampler(&self.smp_clamp) },
            e(11, &self.solid[solid[0]]), e(12, &self.solid[solid[1]]),
        ] })
    }

    /// Advances the simulation to `time` (30 Hz steps, at most two per frame) around the car and records the compute passes.
    pub fn update(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, terrain: &Terrain, car_pos: [f32; 3], wind: [f32; 3], packed: &Packed, time: f32) {
        let dt_frame = (time - self.last_time).clamp(0.0, 0.1);
        self.last_time = time; self.time = time;
        self.acc += dt_frame;
        let steps = ((self.acc / STEP) as u32).min(2);
        self.acc -= steps as f32 * STEP;
        if self.acc > STEP { self.acc = 0.0; }
        if steps == 0 && self.have_window { return; }
        let dx = self.cfg.cell;
        let now = Window::around(car_pos, dx, self.cfg.n, 3.0);
        let prev = if self.have_window { self.window } else { now };
        // the ground under every column of the window (only the columns that changed hands are measured)
        let n = self.n;
        for tz in 0..n[2] as i32 { for tx in 0..n[0] as i32 {
            let cell = now.cell_of([tx, 0, tz]);
            let i = (tz as usize) * n[0] as usize + tx as usize;
            if self.height_cells[i] != [cell[0], cell[2]] {
                self.height_cells[i] = [cell[0], cell[2]];
                self.heights[i] = terrain.height((cell[0] as f32 + 0.5) * dx, (cell[2] as f32 + 0.5) * dx);
            }
        } }
        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &self.height_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&self.heights), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(n[0] * 4), rows_per_image: Some(n[2]) },
            wgpu::Extent3d { width: n[0], height: n[2], depth_or_array_layers: 1 });
        let moved = self.have_window && now != prev;
        let u = FUni { start: [now.start[0], now.start[1], now.start[2], moved as i32], prev: [prev.start[0], prev.start[1], prev.start[2], 0],
            g: [dx, STEP, self.cfg.settle, self.cfg.decay], wind: [wind[0], wind[1], wind[2], self.cfg.drag],
            car: packed.car, car_half: packed.car_half, car_vel: packed.car_vel, wpos: packed.wpos, wvel: packed.wvel, wcol: packed.wcol };
        queue.write_buffer(&self.fbuf, 0, bytemuck::bytes_of(&u));
        self.window = now; self.have_window = true;
        if steps == 0 { return; }
        let (gx, gy, gz) = ((n[0] + 3) / 4, (n[1] + 3) / 4, (n[2] + 3) / 4);
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("fluid"), timestamp_writes: None });
        for s in 0..steps {
            let dp = self.dp;
            // velocity: advect (+ forces) 0 -> 1, divergence of 1, pressure iterations, project 1 -> 0
            cp.set_pipeline(&self.pl_solids);
            cp.set_bind_group(0, &self.bind(dev, [0, 1], [dp, 1 - dp], [0, 1], [1, 0], [1, 0]), &[]);
            cp.dispatch_workgroups(gx, gy, gz);
            cp.set_pipeline(&self.pl_advect_vel);
            cp.set_bind_group(0, &self.bind(dev, [0, 1], [dp, 1 - dp], [0, 1], [1, 0], [0, 1]), &[]);
            cp.dispatch_workgroups(gx, gy, gz);
            cp.set_pipeline(&self.pl_div);
            cp.set_bind_group(0, &self.bind(dev, [1, 0], [dp, 1 - dp], [0, 1], [1, 0], [0, 1]), &[]);
            cp.dispatch_workgroups(gx, gy, gz);
            cp.set_pipeline(&self.pl_jacobi);
            let (j0, j1) = (self.bind(dev, [1, 0], [dp, 1 - dp], [0, 1], [0, 1], [0, 1]), self.bind(dev, [1, 0], [dp, 1 - dp], [1, 0], [0, 1], [0, 1]));
            for k in 0..self.cfg.iterations { cp.set_bind_group(0, if k % 2 == 0 { &j0 } else { &j1 }, &[]); cp.dispatch_workgroups(gx, gy, gz); }
            cp.set_pipeline(&self.pl_project);
            cp.set_bind_group(0, &self.bind(dev, [1, 0], [dp, 1 - dp], [0, 1], [0, 1], [0, 1]), &[]);
            cp.dispatch_workgroups(gx, gy, gz);
            // dust: advect with the final velocity 0, then swap
            cp.set_pipeline(&self.pl_dens);
            cp.set_bind_group(0, &self.bind(dev, [0, 1], [dp, 1 - dp], [0, 1], [0, 1], [0, 1]), &[]);
            cp.dispatch_workgroups(gx, gy, gz);
            self.dp = 1 - dp;
            let _ = s;
        }
    }

    /// Draws the dust volume at half resolution and lays it over the finished frame (`depth` is the main pass's depth buffer, sampled).
    pub fn draw(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, color: &wgpu::TextureView, depth: &wgpu::TextureView, bg_main: &wgpu::BindGroup, size: (u32, u32)) {
        if !self.have_window { return; }
        let hs = ((size.0 + 1) / 2, (size.1 + 1) / 2);
        if self.half.as_ref().map_or(true, |h| h.0 != hs) {
            let t = dev.create_texture(&wgpu::TextureDescriptor { label: Some("fluid_half"), size: wgpu::Extent3d { width: hs.0, height: hs.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
                dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
            self.half = Some((hs, t.create_view(&Default::default())));
        }
        let half = &self.half.as_ref().unwrap().1;
        let u = VUni { start: [self.window.start[0], self.window.start[1], self.window.start[2], 0], dims: [self.n[0] as i32, self.n[1] as i32, self.n[2] as i32, 0],
            g: [self.cfg.cell, self.cfg.absorb, self.cfg.steps as f32, self.time % 7.0], scr: [hs.0 as f32, hs.1 as f32, if self.cfg.debug { 1.0 } else { 0.0 }, 2.0] };
        queue.write_buffer(&self.vbuf, 0, bytemuck::bytes_of(&u));
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("volume_bg"), layout: &self.v_bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: self.vbuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.dens[self.dp]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.smp_repeat) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(depth) },
        ] });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("volume"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: half, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&self.pl_volume);
            rp.set_bind_group(0, bg_main, &[]);
            rp.set_bind_group(1, &bg, &[]);
            rp.draw(0..3, 0..1);
        }
        let cbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("vcomp_bg"), layout: &self.comp_bgl, entries: &[
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(half) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.smp_lin) },
        ] });
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("vcomp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: color, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
        rp.set_pipeline(&self.pl_comp);
        rp.set_bind_group(0, bg_main, &[]);
        rp.set_bind_group(1, &cbg, &[]);
        rp.draw(0..3, 0..1);
    }
}
