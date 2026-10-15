//! The airflow around the car on the GPU (aero step 2, a debug prototype): compute passes in a box that moves with the car and a side slice
//! drawn in a corner of the screen. The algorithm is `game/airflow.rs` (host-tested); the shaders are `shaders/airflow.wgsl`.
use bytemuck::{Pod, Zeroable};
use crate::game::airflow::{self, AirflowCfg, WAKE_LEN};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AUni { n: [i32; 4], g: [f32; 4], inflow: [f32; 4], dust: [f32; 4], shift: [f32; 4], boxes: [[f32; 4]; 16] }

/// The airflow steps at 30 Hz: the target of the game is 30 fps, and the semi-Lagrangian solver is stable at this step.
const STEP: f32 = 1.0 / 30.0;
const MAX_BOXES: usize = 8;
const TEX_USAGE: wgpu::TextureUsages = wgpu::TextureUsages::from_bits_truncate(wgpu::TextureUsages::TEXTURE_BINDING.bits() | wgpu::TextureUsages::STORAGE_BINDING.bits());

pub struct AirflowGpu {
    cfg: AirflowCfg,
    n: [u32; 3],
    /// Velocity ping-pong, two scratch fields (BFECC, vorticity), pressure ping-pong; the dummies fill the bindings a pass does not use.
    v: [wgpu::TextureView; 2], t1: wgpu::TextureView, t2: wgpu::TextureView, zv: wgpu::TextureView,
    p: [wgpu::TextureView; 2], div: wgpu::TextureView, solid: wgpu::TextureView, zs: wgpu::TextureView, zr: wgpu::TextureView,
    cur: usize,
    ubuf: wgpu::Buffer,
    /// The dust field (mass and momentum per cell, fixed point) the dust passes write and the solver reads.
    dens: wgpu::Buffer,
    c_bgl: wgpu::BindGroupLayout,
    pl_init: wgpu::ComputePipeline, pl_solids: wgpu::ComputePipeline, pl_fwd: wgpu::ComputePipeline, pl_corr: wgpu::ComputePipeline, pl_fin: wgpu::ComputePipeline, pl_vort: wgpu::ComputePipeline,
    pl_confine: wgpu::ComputePipeline, pl_div: wgpu::ComputePipeline, pl_jacobi: wgpu::ComputePipeline, pl_project: wgpu::ComputePipeline,
    s_bgl: wgpu::BindGroupLayout, pl_slice: wgpu::RenderPipeline,
    acc: f32, last_time: f32,
    /// The smoothed inflow the solver uses; `None` until the first frame (then the box is filled with it).
    inflow: Option<[f32; 3]>,
}

fn tex3d(dev: &wgpu::Device, label: &str, n: [u32; 3], format: wgpu::TextureFormat) -> wgpu::TextureView {
    dev.create_texture(&wgpu::TextureDescriptor { label: Some(label), size: wgpu::Extent3d { width: n[0], height: n[1], depth_or_array_layers: n[2] }, mip_level_count: 1, sample_count: 1,
        dimension: wgpu::TextureDimension::D3, format, usage: TEX_USAGE, view_formats: &[] }).create_view(&Default::default())
}

impl AirflowGpu {
    /// `None` when the device cannot do it (the formats must be usable as storage textures).
    pub fn new(adapter: &wgpu::Adapter, dev: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Option<AirflowGpu> {
        let cfg = airflow::cfg();
        for f in [wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::R32Float] {
            if !adapter.get_texture_format_features(f).allowed_usages.contains(wgpu::TextureUsages::STORAGE_BINDING) { log::warn!("airflow: {f:?} cannot be a storage texture here"); return None; }
        }
        let n = [cfg.n[0] as u32, cfg.n[1] as u32, cfg.n[2] as u32];
        let (h, r) = (wgpu::TextureFormat::Rgba16Float, wgpu::TextureFormat::R32Float);
        let v = [tex3d(dev, "av0", n, h), tex3d(dev, "av1", n, h)];
        let (t1, t2, zv) = (tex3d(dev, "at1", n, h), tex3d(dev, "at2", n, h), tex3d(dev, "azv", n, h));
        let p = [tex3d(dev, "ap0", n, r), tex3d(dev, "ap1", n, r)];
        let (div, solid, zs, zr) = (tex3d(dev, "adiv", n, r), tex3d(dev, "asolid", n, r), tex3d(dev, "azs", n, r), tex3d(dev, "azr", n, r));
        let dens = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("adens"), size: (n[0] * n[1] * n[2] * 4 * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("aparams"), size: std::mem::size_of::<AUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });

        let ub = |vis: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry { binding: 0, visibility: vis, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None };
        let tex = |binding: u32, vis: wgpu::ShaderStages| wgpu::BindGroupLayoutEntry { binding, visibility: vis,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None };
        let store = |binding: u32, format: wgpu::TextureFormat| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format, view_dimension: wgpu::TextureViewDimension::D3 }, count: None };
        let c = wgpu::ShaderStages::COMPUTE;
        let c_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("airflow_c"), entries: &[
            ub(c | wgpu::ShaderStages::FRAGMENT), tex(1, c | wgpu::ShaderStages::FRAGMENT), tex(2, c), tex(3, c), tex(4, c), tex(5, c | wgpu::ShaderStages::FRAGMENT), store(6, h), store(7, r),
            wgpu::BindGroupLayoutEntry { binding: 8, visibility: c, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ] });
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("airflow"), source: wgpu::ShaderSource::Wgsl(crate::shaders::airflow().into()) });
        let c_lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&c_bgl], push_constant_ranges: &[] });
        let cp = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some(entry), layout: Some(&c_lay), module: &module, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
        let pl_init = cp("init");
        let (pl_solids, pl_fwd, pl_corr, pl_fin, pl_vort) = (cp("solids"), cp("adv_fwd"), cp("adv_corr"), cp("adv_fin"), cp("vort"));
        let (pl_confine, pl_div, pl_jacobi, pl_project) = (cp("confine"), cp("divergence"), cp("jacobi"), cp("project"));

        // the slice view reads the same uniform, the velocity (binding 1) and the solid field (binding 5)
        let f = wgpu::ShaderStages::FRAGMENT;
        let s_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("airflow_s"), entries: &[ub(f), tex(1, f), tex(5, f)] });
        let s_lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&s_bgl], push_constant_ranges: &[] });
        let pl_slice = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("airflow_slice"), layout: Some(&s_lay),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_slice"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_slice"), compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: surface_format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        log::info!("airflow: {}x{}x{} cells of {} m", n[0], n[1], n[2], cfg.cell);
        Some(AirflowGpu { cfg, n, dens, v, t1, t2, zv, p, div, solid, zs, zr, cur: 0, ubuf, c_bgl, pl_init, pl_solids, pl_fwd, pl_corr, pl_fin, pl_vort, pl_confine, pl_div, pl_jacobi, pl_project,
            s_bgl, pl_slice, acc: 0.0, last_time: 0.0, inflow: None })
    }

    /// One bind group: the reads (`vel`, `t`, `pres`, `div`, `solid`) and the writes (`dv` to a vec4 field, `ds` to a scalar field).
    fn bind(&self, dev: &wgpu::Device, vel: &wgpu::TextureView, t: &wgpu::TextureView, pres: &wgpu::TextureView, div: &wgpu::TextureView, solid: &wgpu::TextureView, dv: &wgpu::TextureView, ds: &wgpu::TextureView) -> wgpu::BindGroup {
        fn e<'a>(binding: u32, v: &'a wgpu::TextureView) -> wgpu::BindGroupEntry<'a> { wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(v) } }
        dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("airflow_bg"), layout: &self.c_bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: self.ubuf.as_entire_binding() }, e(1, vel), e(2, t), e(3, pres), e(4, div), e(5, solid), e(6, dv), e(7, ds), wgpu::BindGroupEntry { binding: 8, resource: self.dens.as_entire_binding() },
        ] })
    }

    /// Advances the simulation to `time` (30 Hz steps, at most `substeps` per frame) and records the compute passes. `inflow` is the air's velocity
    /// relative to the car in the car's frame, `boxes` the car (see `airflow::thick_boxes`).
    pub fn update(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, inflow: [f32; 3], boxes: &[([f32; 3], [f32; 3])], time: f32) {
        let dt_frame = (time - self.last_time).clamp(0.0, 0.1);
        self.last_time = time;
        self.acc += dt_frame;
        let steps = ((self.acc / STEP) as u32).min(self.cfg.substeps);
        self.acc -= steps as f32 * STEP;
        if self.acc > STEP { self.acc = 0.0; }
        let first = self.inflow.is_none();
        let prev = self.inflow.unwrap_or(inflow);
        // the inflow only moves on frames that step (the shift below is applied by the steps)
        let inflow = if first { inflow } else if steps > 0 { airflow::smooth_inflow(prev, inflow, steps as f32 * STEP) } else { prev };
        self.inflow = Some(inflow);
        // the whole air follows the car's change of speed; split over the steps of this frame
        let per = 1.0 / steps.max(1) as f32;
        let shift = [0, 1, 2].map(|a| (inflow[a] - prev[a]) * per);
        let mut u = AUni { n: [self.n[0] as i32, self.n[1] as i32, self.n[2] as i32, boxes.len().min(MAX_BOXES) as i32], g: [self.cfg.cell, STEP, self.cfg.confinement, (WAKE_LEN / self.cfg.cell).max(1.0).floor()],
            inflow: [inflow[0], inflow[1], inflow[2], 0.0], dust: { let c = crate::game::dust_ref::coupling(); [c.drag, c.drag_max, 0.0, 0.0] }, shift: [shift[0], shift[1], shift[2], 0.0], boxes: [[0.0; 4]; 16] };
        for (i, (lo, hi)) in boxes.iter().take(MAX_BOXES).enumerate() { u.boxes[2 * i] = [lo[0], lo[1], lo[2], 0.0]; u.boxes[2 * i + 1] = [hi[0], hi[1], hi[2], 0.0]; }
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        if steps == 0 && !first { return; }
        let (gx, gy, gz) = ((self.n[0] + 3) / 4, (self.n[1] + 3) / 4, (self.n[2] + 3) / 4);
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("airflow"), timestamp_writes: None });
        if first {
            for t in [&self.v[0], &self.v[1]] { cp.set_pipeline(&self.pl_init); cp.set_bind_group(0, &self.bind(dev, &self.t1, &self.t2, &self.p[0], &self.div, &self.solid, t, &self.zs), &[]); cp.dispatch_workgroups(gx, gy, gz); }
        }
        let (zv, zs, zr) = (&self.zv, &self.zs, &self.zr);   // zv, zs: unused write targets; zr: an unused scalar to read
        for _ in 0..steps {
            let (a, b) = (&self.v[self.cur], &self.v[1 - self.cur]);
            let run = |cp: &mut wgpu::ComputePass, pl: &wgpu::ComputePipeline, bg: &wgpu::BindGroup| { cp.set_pipeline(pl); cp.set_bind_group(0, bg, &[]); cp.dispatch_workgroups(gx, gy, gz); };
            // the solid field of the car's boxes
            run(&mut cp, &self.pl_solids, &self.bind(dev, a, &self.t1, &self.p[0], zr, zr, zv, &self.solid));
            // BFECC: forward a -> t1, correction (a, t1) -> t2, final (a, t2) -> b
            run(&mut cp, &self.pl_fwd, &self.bind(dev, a, &self.t2, &self.p[0], zr, &self.solid, &self.t1, zs));
            run(&mut cp, &self.pl_corr, &self.bind(dev, a, &self.t1, &self.p[0], zr, &self.solid, &self.t2, zs));
            run(&mut cp, &self.pl_fin, &self.bind(dev, a, &self.t2, &self.p[0], zr, &self.solid, b, zs));
            // vorticity b -> t1, confinement (b, t1) -> a
            run(&mut cp, &self.pl_vort, &self.bind(dev, b, &self.t2, &self.p[0], zr, &self.solid, &self.t1, zs));
            run(&mut cp, &self.pl_confine, &self.bind(dev, b, &self.t1, &self.p[0], zr, &self.solid, a, zs));
            // projection of a: divergence, pressure iterations, gradient subtraction a -> b
            run(&mut cp, &self.pl_div, &self.bind(dev, a, &self.t1, &self.p[0], zr, &self.solid, zv, &self.div));
            let (j0, j1) = (self.bind(dev, &self.t1, &self.t2, &self.p[0], &self.div, &self.solid, zv, &self.p[1]), self.bind(dev, &self.t1, &self.t2, &self.p[1], &self.div, &self.solid, zv, &self.p[0]));
            for k in 0..self.cfg.iterations { run(&mut cp, &self.pl_jacobi, if k % 2 == 0 { &j0 } else { &j1 }); }
            let pres = &self.p[(self.cfg.iterations % 2) as usize];
            run(&mut cp, &self.pl_project, &self.bind(dev, a, &self.t2, pres, zr, &self.solid, b, zs));
            self.cur = 1 - self.cur;
        }
    }

    /// The velocity field of the latest step (air relative to the car, in the car's frame) and the car's solid cells, for the dust.
    pub fn fields(&self) -> (&wgpu::TextureView, &wgpu::TextureView) { (&self.v[self.cur], &self.solid) }
    /// The dust field buffer (see `dens`).
    pub fn dens_buffer(&self) -> &wgpu::Buffer { &self.dens }
    /// The box in metres (x, y, z) and its cell.
    pub fn box_size(&self) -> [f32; 4] { [self.n[0] as f32 * self.cfg.cell, self.n[1] as f32 * self.cfg.cell, self.n[2] as f32 * self.cfg.cell, self.cfg.cell] }

    /// Draws the slice at the left, above the touch controls (half of the screen's width).
    pub fn draw(&self, dev: &wgpu::Device, enc: &mut wgpu::CommandEncoder, color: &wgpu::TextureView, size: (u32, u32)) {
        let w = (size.0 as f32 * 0.5).max(64.0);
        let h = (w * self.n[1] as f32 / self.n[2] as f32).max(16.0);
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("airflow_slice_bg"), layout: &self.s_bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: self.ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.v[self.cur]) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&self.solid) },
        ] });
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("airflow_slice"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: color, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
        rp.set_viewport(8.0, size.1 as f32 * 0.22, w, h, 0.0, 1.0);
        rp.set_pipeline(&self.pl_slice);
        rp.set_bind_group(0, &bg, &[]);
        rp.draw(0..3, 0..1);
    }
}
