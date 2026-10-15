//! The dust on the GPU: the particles live in a ring buffer and are moved by compute passes (`shaders/dust_gpu.wgsl`, the reference is
//! `game/dust_ref.rs`); the result is written in the layout of `DustInst`, so the render buffer is the vertex buffer of the dust sprites.
use bytemuck::{Pod, Zeroable};
use crate::game::dust::{phys, MAX_EMITS};
use crate::game::world::DustGpuIn;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct S { a: [f32; 4], b: [f32; 4], c: [f32; 4], d: [f32; 4] }
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DUni { n: [u32; 4], g: [f32; 4], wind: [f32; 4], ph: [f32; 4], ph2: [f32; 4], car: [f32; 4], car_vel: [f32; 4], bx: [f32; 4], emits: [S; MAX_EMITS] }

/// The buffer holds this many particles; the level's cap is how many the ring uses.
pub const CAPACITY: usize = 4096;
const DUST_INST_BYTES: u64 = 48;

pub struct DustGpu {
    ubuf: wgpu::Buffer, state: wgpu::Buffer, render: wgpu::Buffer,
    bg: wgpu::BindGroup, bgl_grid: wgpu::BindGroupLayout, dummy: wgpu::TextureView,
    pl_emit: wgpu::ComputePipeline, pl_step: wgpu::ComputePipeline,
    head: usize, cap: usize, last_time: f32,
}

impl DustGpu {
    pub fn new(dev: &wgpu::Device) -> DustGpu {
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_u"), size: std::mem::size_of::<DUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let state = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_state"), size: (CAPACITY * std::mem::size_of::<S>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let render = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_render"), size: CAPACITY as u64 * DUST_INST_BYTES, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let entry = |binding: u32, ty: wgpu::BufferBindingType| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None }, count: None };
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_bgl"), entries: &[
            entry(0, wgpu::BufferBindingType::Uniform), entry(1, wgpu::BufferBindingType::Storage { read_only: false }), entry(2, wgpu::BufferBindingType::Storage { read_only: false })] });
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_bg"), layout: &bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: state.as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: render.as_entire_binding() }] });
        let tex = |binding: u32| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None };
        let bgl_grid = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_grid_bgl"), entries: &[tex(0), tex(1)] });
        let dummy = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_dummy"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D3, format: wgpu::TextureFormat::Rgba16Float, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }).create_view(&Default::default());
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dust_gpu"), source: wgpu::ShaderSource::Wgsl(crate::shaders::dust_gpu().into()) });
        let lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl, &bgl_grid], push_constant_ranges: &[] });
        let cp = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some(entry), layout: Some(&lay), module: &module, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
        let (pl_emit, pl_step) = (cp("emit"), cp("dust_step"));
        log::info!("dust gpu: {CAPACITY} particles");
        DustGpu { ubuf, state, render, bg, bgl_grid, dummy, pl_emit, pl_step, head: 0, cap: 0, last_time: -1.0 }
    }

    pub fn render_buffer(&self) -> &wgpu::Buffer { &self.render }
    /// How many instances to draw: the ring's size.
    pub fn count(&self) -> u32 { self.cap as u32 }

    /// Emits the wheels' new particles and moves all of them to `frame.time`.
    /// `grid`: the airflow's velocity and solid textures and its box (see `AirflowGpu::fields`), or None (the wind only).
    pub fn update(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, frame: &DustGpuIn, grid: Option<(&wgpu::TextureView, &wgpu::TextureView, [f32; 4])>) {
        let cap = frame.cap.clamp(1, CAPACITY);
        if cap != self.cap { queue.write_buffer(&self.state, 0, &vec![0u8; CAPACITY * std::mem::size_of::<S>()]); queue.write_buffer(&self.render, 0, &vec![0u8; CAPACITY * DUST_INST_BYTES as usize]); self.head = 0; self.cap = cap; }
        let dt = if self.last_time < 0.0 { 0.0 } else { (frame.time - self.last_time).clamp(0.0, 0.1) };
        self.last_time = frame.time;
        let ph = phys();
        let mut u = DUni { n: [frame.emits.len().min(MAX_EMITS) as u32, self.head as u32, cap as u32, 0], g: [dt, frame.time, frame.ground_y, 0.0],
            wind: [frame.wind.x, frame.wind.y, frame.wind.z, 0.0], ph: [ph.tau_fine, ph.tau_coarse, ph.settle_fine, ph.settle_coarse], ph2: [ph.turbulence, (frame.car_vel.x * frame.car_vel.x + frame.car_vel.z * frame.car_vel.z).sqrt(), 0.0, 0.0],
            car: [frame.car_pos.x, frame.car_pos.y, frame.car_pos.z, frame.yaw], car_vel: [frame.car_vel.x, frame.car_vel.y, frame.car_vel.z, 0.0], bx: grid.map_or([0.0; 4], |g| g.2), emits: [S::default(); MAX_EMITS] };
        for (i, p) in frame.emits.iter().take(MAX_EMITS).enumerate() {
            u.emits[i] = S { a: [p.pos.x, p.pos.y, p.pos.z, p.size0], b: [p.vel.x, p.vel.y, p.vel.z, p.age], c: [p.color[0], p.color[1], p.color[2], p.life], d: [if p.coarse { 1.0 } else { 0.0 }, p.seed, 1.0, 0.0] };
        }
        self.head = (self.head + u.n[0] as usize) % cap;
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_gpu"), timestamp_writes: None });
        let (vel, solid) = grid.map_or((&self.dummy, &self.dummy), |g| (g.0, g.1));
        let gbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_grid_bg"), layout: &self.bgl_grid, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(vel) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(solid) }] });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        cp.set_pipeline(&self.pl_emit);
        cp.dispatch_workgroups(((u.n[0] + 63) / 64).max(1), 1, 1);
        cp.set_pipeline(&self.pl_step);
        cp.dispatch_workgroups(((cap as u32) + 63) / 64, 1, 1);
    }
}
