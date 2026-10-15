//! The dust on the GPU: the particles live in a ring buffer and are moved by compute passes (`shaders/dust_gpu.wgsl`, the reference is
//! `game/dust_ref.rs`); the result is written in the layout of `DustInst`, so the render buffer is the vertex buffer of the dust sprites.
use bytemuck::{Pod, Zeroable};
use crate::game::dust::{phys, MAX_EMITS};
use crate::game::dust_ref::{self, WINDOW};
use crate::game::terrain::Terrain;
use crate::game::world::DustGpuIn;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct S { a: [f32; 4], b: [f32; 4], c: [f32; 4], d: [f32; 4] }
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DUni { n: [u32; 4], g: [f32; 4], wind: [f32; 4], ph: [f32; 4], ph2: [f32; 4], car: [f32; 4], car_vel: [f32; 4], bx: [f32; 4], gw: [i32; 4], cp: [f32; 4], sun: [f32; 4], body: [f32; 4] }

/// The buffer holds this many particles; the level's cap is how many the ring uses.
pub const CAPACITY: usize = 65536;
/// The most cells of the airflow's box the per-cell dust field has room for.
const MAX_CELLS: usize = 262144;
const DUST_INST_BYTES: u64 = 48;

pub struct DustGpu {
    ubuf: wgpu::Buffer, emit_buf: wgpu::Buffer, state: wgpu::Buffer, mass_buf: wgpu::Buffer, fld_buf: wgpu::Buffer, render: wgpu::Buffer,
    bg: wgpu::BindGroup, bgl_grid: wgpu::BindGroupLayout, dummy: wgpu::TextureView,
    pl_emit: wgpu::ComputePipeline, pl_step: wgpu::ComputePipeline, pl_clear: wgpu::ComputePipeline, pl_splat: wgpu::ComputePipeline, pl_fmass: wgpu::ComputePipeline, pl_flight: wgpu::ComputePipeline, dummy_buf: wgpu::Buffer,
    head: usize, cap: usize, last_time: f32,
    /// The terrain heights of the ground window (cell mod 64) and the cell each slot holds now.
    ground_tex: wgpu::Texture, heights: Vec<f32>, cells: Vec<[i32; 2]>,
}

impl DustGpu {
    pub fn new(dev: &wgpu::Device) -> DustGpu {
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_u"), size: std::mem::size_of::<DUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let emit_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_emits"), size: (MAX_EMITS * std::mem::size_of::<S>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mass_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_mass"), size: (MAX_CELLS * 4) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let fld_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_fld"), size: (MAX_CELLS * 16) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let state = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_state"), size: (CAPACITY * std::mem::size_of::<S>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let render = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_render"), size: CAPACITY as u64 * DUST_INST_BYTES, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let entry = |binding: u32, ty: wgpu::BufferBindingType| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None }, count: None };
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_bgl"), entries: &[
            entry(0, wgpu::BufferBindingType::Uniform), entry(1, wgpu::BufferBindingType::Storage { read_only: false }), entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
            entry(4, wgpu::BufferBindingType::Storage { read_only: true }),
            entry(5, wgpu::BufferBindingType::Storage { read_only: false }), entry(6, wgpu::BufferBindingType::Storage { read_only: false }),
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None }] });
        let ground_tex = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_ground"), size: wgpu::Extent3d { width: WINDOW as u32, height: WINDOW as u32, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::R32Float, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        let ground_view = ground_tex.create_view(&Default::default());
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_bg"), layout: &bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: state.as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: render.as_entire_binding() }, wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&ground_view) }, wgpu::BindGroupEntry { binding: 4, resource: emit_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: mass_buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 6, resource: fld_buf.as_entire_binding() }] });
        let tex = |binding: u32| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None };
        let bgl_grid = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_grid_bgl"), entries: &[tex(0), tex(1), entry(2, wgpu::BufferBindingType::Storage { read_only: false })] });
        let dummy_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_dummy_buf"), size: 16, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let dummy = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_dummy"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D3, format: wgpu::TextureFormat::Rgba16Float, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }).create_view(&Default::default());
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dust_gpu"), source: wgpu::ShaderSource::Wgsl(crate::shaders::dust_gpu().into()) });
        let lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl, &bgl_grid], push_constant_ranges: &[] });
        let cp = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some(entry), layout: Some(&lay), module: &module, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
        let (pl_emit, pl_step, pl_clear, pl_splat) = (cp("emit"), cp("dust_step"), cp("dust_clear"), cp("dust_splat"));
        let (pl_fmass, pl_flight) = (cp("dust_field_mass"), cp("dust_field_light"));
        log::info!("dust gpu: {CAPACITY} particles");
        DustGpu { ubuf, emit_buf, state, mass_buf, fld_buf, render, bg, bgl_grid, dummy, pl_emit, pl_step, pl_clear, pl_splat, pl_fmass, pl_flight, dummy_buf, head: 0, cap: 0, last_time: -1.0, ground_tex, heights: vec![0.0; (WINDOW * WINDOW) as usize], cells: vec![[i32::MIN; 2]; (WINDOW * WINDOW) as usize] }
    }

    pub fn render_buffer(&self) -> &wgpu::Buffer { &self.render }
    /// How many instances to draw: the ring's size.
    pub fn count(&self) -> u32 { self.cap as u32 }

    /// Emits the wheels' new particles and moves all of them to `frame.time`.
    /// `grid`: the airflow's velocity and solid textures and its box (see `AirflowGpu::fields`), or None (the wind only).
    pub fn update(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, terrain: &Terrain, frame: &DustGpuIn, grid: Option<(&wgpu::TextureView, &wgpu::TextureView, [f32; 4], &wgpu::Buffer)>) {
        let cap = frame.cap.clamp(1, CAPACITY);
        if cap != self.cap { queue.write_buffer(&self.state, 0, &vec![0u8; CAPACITY * std::mem::size_of::<S>()]); queue.write_buffer(&self.render, 0, &vec![0u8; CAPACITY * DUST_INST_BYTES as usize]); self.head = 0; self.cap = cap; }
        let dt = if self.last_time < 0.0 { 0.0 } else { (frame.time - self.last_time).clamp(0.0, 0.1) };
        self.last_time = frame.time;
        // the terrain heights of the window around the car: only the cells that came into it are measured
        let base = dust_ref::window_base([frame.car_pos.x.floor() as i32, frame.car_pos.z.floor() as i32]);
        let mut changed = false;
        for tz in 0..WINDOW { for tx in 0..WINDOW {
            let cell = [base[0] + (tx - base[0]).rem_euclid(WINDOW), base[1] + (tz - base[1]).rem_euclid(WINDOW)];
            let i = dust_ref::slot(cell);
            if self.cells[i] != cell { self.cells[i] = cell; self.heights[i] = terrain.height(cell[0] as f32 + 0.5, cell[1] as f32 + 0.5); changed = true; }
        } }
        if changed {
            queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &self.ground_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                bytemuck::cast_slice(&self.heights), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(WINDOW as u32 * 4), rows_per_image: Some(WINDOW as u32) },
                wgpu::Extent3d { width: WINDOW as u32, height: WINDOW as u32, depth_or_array_layers: 1 });
        }
        let ph = phys();
        let mut u = DUni { n: [frame.emits.len().min(MAX_EMITS) as u32, self.head as u32, cap as u32, 0], g: [dt, frame.time, frame.ground_y, 0.0],
            wind: [frame.wind.x, frame.wind.y, frame.wind.z, 0.0], ph: [ph.tau_fine, ph.tau_coarse, ph.settle_fine, ph.settle_coarse], ph2: [ph.turbulence * 1.5, (frame.car_vel.x * frame.car_vel.x + frame.car_vel.z * frame.car_vel.z).sqrt(), ph.plume_lift, ph.plume_height],
            car: [frame.car_pos.x, frame.car_pos.y, frame.car_pos.z, frame.yaw], car_vel: [frame.car_vel.x, frame.car_vel.y, frame.car_vel.z, 0.0], bx: grid.map_or([0.0; 4], |g| g.2), gw: [base[0], base[1], 0, 0], cp: { let c = dust_ref::coupling(); [c.push, c.mass_fine, c.mass_coarse, 0.0] }, sun: [frame.sun.x, frame.sun.y, frame.sun.z, dust_ref::coupling().shadow_k], body: [frame.body[0], frame.body[1], frame.body[2], ph.vortex] };
        let emits: Vec<S> = frame.emits.iter().take(MAX_EMITS).map(|p| S { a: [p.pos.x, p.pos.y, p.pos.z, p.size0], b: [p.vel.x, p.vel.y, p.vel.z, p.age], c: [p.color[0], p.color[1], p.color[2], p.life], d: [if p.coarse { 1.0 } else { 0.0 }, p.seed, 1.0, 0.0] }).collect();
        if !emits.is_empty() { queue.write_buffer(&self.emit_buf, 0, bytemuck::cast_slice(&emits)); }
        self.head = (self.head + u.n[0] as usize) % cap;
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_gpu"), timestamp_writes: None });
        let (vel, solid) = grid.map_or((&self.dummy, &self.dummy), |g| (g.0, g.1));
        let dens = grid.map_or(&self.dummy_buf, |g| g.3);
        let gbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_grid_bg"), layout: &self.bgl_grid, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(vel) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(solid) }, wgpu::BindGroupEntry { binding: 2, resource: dens.as_entire_binding() }] });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        cp.set_pipeline(&self.pl_emit);
        cp.dispatch_workgroups(((u.n[0] + 63) / 64).max(1), 1, 1);
        cp.set_pipeline(&self.pl_step);
        cp.dispatch_workgroups(((cap as u32) + 63) / 64, 1, 1);
        // the dust field of this frame (read by the next frame's step and by the airflow solver): clear, then every particle adds itself
        if let Some(g) = grid {
            let cells = (g.2[0] / g.2[3]).round() as u32 * (g.2[1] / g.2[3]).round() as u32 * (g.2[2] / g.2[3]).round() as u32;
            cp.set_pipeline(&self.pl_clear);
            cp.dispatch_workgroups((cells * 4 + 63) / 64, 1, 1);
            cp.set_pipeline(&self.pl_splat);
            cp.dispatch_workgroups(((cap as u32) + 63) / 64, 1, 1);
            let groups = ((cells.min(MAX_CELLS as u32)) + 63) / 64;
            cp.set_pipeline(&self.pl_fmass);
            cp.dispatch_workgroups(groups, 1, 1);
            cp.set_pipeline(&self.pl_flight);
            cp.dispatch_workgroups(groups, 1, 1);
        }
    }
}


/// The GPU dust drawn at a fraction of the screen: the particles go into a small premultiplied target (hidden behind and soft against the scene, whose depth
/// is sampled) that is then laid over the finished frame (bilinear). The clouds are soft, so a coarse target hardly shows and costs a sixteenth of the pixels.
pub struct DustRender {
    pl_low: wgpu::RenderPipeline, bgl_depth: wgpu::BindGroupLayout,
    pl_comp: wgpu::RenderPipeline, bgl_comp: wgpu::BindGroupLayout, smp: wgpu::Sampler,
    low: Option<((u32, u32), wgpu::TextureView)>,
}

const LOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The dust target is 1/DUST_DIV of the screen in each direction (= DUST_DIV in raster.wgsl).
const DUST_DIV: u32 = 6;

impl DustRender {
    pub fn new(dev: &wgpu::Device, bgl_main: &wgpu::BindGroupLayout, raster: &wgpu::ShaderModule, surface_format: wgpu::TextureFormat) -> DustRender {
        let premul = wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha, operation: wgpu::BlendOperation::Add } };
        let f = wgpu::ShaderStages::FRAGMENT;
        let bgl_depth = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_depth"), entries: &[wgpu::BindGroupLayoutEntry { binding: 3, visibility: f,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None }] });
        let bgl_comp = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_comp"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: f, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: f, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None }] });
        let smp = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("dust_comp_smp"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let layout = wgpu::VertexBufferLayout { array_stride: DUST_INST_BYTES, step_mode: wgpu::VertexStepMode::Instance, attributes: &ATTRS };
        let lay_low = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[bgl_main, &bgl_depth], push_constant_ranges: &[] });
        let pl_low = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor { label: Some("dust_low"), layout: Some(&lay_low),
            vertex: wgpu::VertexState { module: raster, entry_point: Some("vs_dust"), compilation_options: Default::default(), buffers: &[layout] },
            fragment: Some(wgpu::FragmentState { module: raster, entry_point: Some("fs_dust_lo"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: LOW_FORMAT, blend: Some(premul), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        let lay_comp = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[bgl_main, &bgl_comp], push_constant_ranges: &[] });
        let pl_comp = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor { label: Some("dust_comp"), layout: Some(&lay_comp),
            vertex: wgpu::VertexState { module: raster, entry_point: Some("vs_vcomp"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: raster, entry_point: Some("fs_vcomp"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: surface_format, blend: Some(premul), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        DustRender { pl_low, bgl_depth, pl_comp, bgl_comp, smp, low: None }
    }

    /// Draws `count` particles of `buf` into the small target and lays it over `color`; `depth` is the main pass's depth buffer (it must have been stored).
    pub fn draw(&mut self, dev: &wgpu::Device, enc: &mut wgpu::CommandEncoder, color: &wgpu::TextureView, depth: &wgpu::TextureView, bg_main: &wgpu::BindGroup, buf: &wgpu::Buffer, count: u32, size: (u32, u32)) {
        let ls = ((size.0 + DUST_DIV - 1) / DUST_DIV, (size.1 + DUST_DIV - 1) / DUST_DIV);
        if self.low.as_ref().map_or(true, |h| h.0 != ls) {
            let t = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_low"), size: wgpu::Extent3d { width: ls.0, height: ls.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
                dimension: wgpu::TextureDimension::D2, format: LOW_FORMAT, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
            self.low = Some((ls, t.create_view(&Default::default())));
        }
        let low = &self.low.as_ref().unwrap().1;
        let dbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_depth_bg"), layout: &self.bgl_depth, entries: &[wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(depth) }] });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("dust_low"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: low, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&self.pl_low);
            rp.set_bind_group(0, bg_main, &[]);
            rp.set_bind_group(1, &dbg, &[]);
            rp.set_vertex_buffer(0, buf.slice(..));
            rp.draw(0..6, 0..count);
        }
        let cbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_comp_bg"), layout: &self.bgl_comp, entries: &[
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(low) }, wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.smp) }] });
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("dust_comp"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: color, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
        rp.set_pipeline(&self.pl_comp);
        rp.set_bind_group(0, bg_main, &[]);
        rp.set_bind_group(1, &cbg, &[]);
        rp.draw(0..3, 0..1);
    }
}
