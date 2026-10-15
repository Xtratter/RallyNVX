//! The dust on the GPU: the particles live in a ring buffer and are moved by compute passes (`shaders/dust_gpu.wgsl`, the reference is
//! `game/dust_ref.rs`); the result is written in the layout of `DustInst`, so the render buffer is the vertex buffer of the dust sprites.
use bytemuck::{Pod, Zeroable};
use crate::game::dust::{phys, MAX_EMITS};
use crate::game::dust_ref::{self, WINDOW, MAX_COLLIDERS};
use crate::game::props::Prop;
use crate::game::terrain::Terrain;
use crate::game::world::DustGpuIn;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
struct S { a: [f32; 4], b: [f32; 4], c: [f32; 4], d: [f32; 4] }
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DUni { n: [u32; 4], g: [f32; 4], wind: [f32; 4], ph: [f32; 4], ph2: [f32; 4], car: [f32; 4], car_vel: [f32; 4], bx: [f32; 4], gw: [i32; 4], cp: [f32; 4], sun: [f32; 4], body: [f32; 4],
    /// The body mesh's origin and axes in the world, the signed distance field's corner and cell, its size and whether there is one.
    bpos: [f32; 4], bax0: [f32; 4], bax1: [f32; 4], bax2: [f32; 4], sdf_o: [f32; 4], sdf_n: [f32; 4],
    /// The reserved ring (start, size), the frame number, 0.
    rs: [u32; 4],
    /// The particle pressure: the kernel radius, the rest density, the relaxation, the viscosity.
    pb: [f32; 4] }

/// The buffer holds this many particles; the level's cap is how many the ring uses.
pub const CAPACITY: usize = 65536;
/// The particles the settled layer lifts again live in this many extra slots after the main ring.
pub const RS_N: usize = 4096;
/// Real fine dust settles at millimetres to centimetres a second (Stokes: 6 um grains about 1 mm/s, 50 um about 0.1 m/s): the GPU dust uses a fraction of the CPU path's speeds.
const SETTLE_FINE_SCALE: f32 = 0.3;
const SETTLE_COARSE_SCALE: f32 = 0.35;
/// The most cells of the airflow's box the per-cell dust field has room for.
const MAX_CELLS: usize = 262144;
/// Every this-many-th particle takes part in the pressure.
pub const PBD_STRIDE: usize = 8;
const DUST_INST_BYTES: u64 = 48;

pub struct DustGpu {
    sdf: Option<(wgpu::TextureView, [f32; 4], [f32; 4])>, sdf_serial: u32,
    ubuf: wgpu::Buffer, coll_buf: wgpu::Buffer, cell_buf: wgpu::Buffer, emit_buf: wgpu::Buffer, state: wgpu::Buffer, mass_buf: wgpu::Buffer, fld_buf: wgpu::Buffer, render: wgpu::Buffer,
    bg: wgpu::BindGroup, bgl_grid: wgpu::BindGroupLayout, dummy: wgpu::TextureView,
    pl_emit: wgpu::ComputePipeline, pl_step: wgpu::ComputePipeline, pl_clear: wgpu::ComputePipeline, pl_splat: wgpu::ComputePipeline, pl_fmass: wgpu::ComputePipeline, pl_flight: wgpu::ComputePipeline, dummy_buf: wgpu::Buffer,
    head: usize, cap: usize, last_time: f32, frame: u32,
    /// The settled layer: counters per cell of the ground window (and one for the reserved ring's head), each slot's cell, and the texture the ground shaders read.
    dep_buf: wgpu::Buffer, dep_cells: wgpu::Buffer, pl_dep: wgpu::ComputePipeline,
    /// The particle pressure: the cell lists (counts then 8 slots per cell), the per-particle corrections, and the passes.
    pbd_grid: wgpu::Buffer, pbd_dat: wgpu::Buffer, pl_pbd: [wgpu::ComputePipeline; 5],
    /// GPU timings of the four compute passes (the extra `gpuprof=1`), read back every 90 frames.
    qs: Option<(wgpu::QuerySet, wgpu::Buffer, wgpu::Buffer, f32)>, prof_frame: u32,
    /// The terrain heights of the ground window (cell mod 64) and the cell each slot holds now.
    ground_tex: wgpu::Texture, heights: Vec<f32>, cells: Vec<[i32; 2]>,
}

impl DustGpu {
    pub fn new(dev: &wgpu::Device, queue: &wgpu::Queue, dep_tex: &wgpu::Texture) -> DustGpu {
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_u"), size: std::mem::size_of::<DUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let dep_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_dep"), size: ((WINDOW * WINDOW + 1) * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let dep_cells = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_dep_cells"), size: (WINDOW * WINDOW * 8) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let dep_view = dep_tex.create_view(&Default::default());
        let pbd_grid = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_pbd_grid"), size: (MAX_CELLS * 9 * 4) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let pbd_dat = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_pbd_dat"), size: (((CAPACITY + RS_N) / PBD_STRIDE + 1) * 32) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let coll_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_colliders"), size: (MAX_COLLIDERS * 32) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let cell_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_ccells"), size: (WINDOW * WINDOW * 16) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let emit_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_emits"), size: (MAX_EMITS * std::mem::size_of::<S>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let mass_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_mass"), size: (MAX_CELLS * 4) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let fld_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_fld"), size: (MAX_CELLS * 16) as u64, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let state = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_state"), size: ((CAPACITY + RS_N) * std::mem::size_of::<S>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let render = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_render"), size: (CAPACITY + RS_N) as u64 * DUST_INST_BYTES, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let entry = |binding: u32, ty: wgpu::BufferBindingType| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None }, count: None };
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_bgl"), entries: &[
            entry(0, wgpu::BufferBindingType::Uniform), entry(1, wgpu::BufferBindingType::Storage { read_only: false }), entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
            entry(4, wgpu::BufferBindingType::Storage { read_only: true }),
            entry(5, wgpu::BufferBindingType::Storage { read_only: false }), entry(6, wgpu::BufferBindingType::Storage { read_only: false }),
            entry(7, wgpu::BufferBindingType::Storage { read_only: true }), entry(8, wgpu::BufferBindingType::Storage { read_only: true }),
            entry(9, wgpu::BufferBindingType::Storage { read_only: false }), entry(10, wgpu::BufferBindingType::Storage { read_only: false }),
            entry(12, wgpu::BufferBindingType::Storage { read_only: false }), entry(13, wgpu::BufferBindingType::Storage { read_only: false }),
            wgpu::BindGroupLayoutEntry { binding: 11, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None }] });
        let ground_tex = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_ground"), size: wgpu::Extent3d { width: WINDOW as u32, height: WINDOW as u32, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::R32Float, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        let ground_view = ground_tex.create_view(&Default::default());
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_bg"), layout: &bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: state.as_entire_binding() }, wgpu::BindGroupEntry { binding: 2, resource: render.as_entire_binding() }, wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&ground_view) }, wgpu::BindGroupEntry { binding: 4, resource: emit_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: mass_buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 6, resource: fld_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 7, resource: coll_buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 8, resource: cell_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 9, resource: dep_buf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 10, resource: dep_cells.as_entire_binding() }, wgpu::BindGroupEntry { binding: 11, resource: wgpu::BindingResource::TextureView(&dep_view) },
            wgpu::BindGroupEntry { binding: 12, resource: pbd_grid.as_entire_binding() }, wgpu::BindGroupEntry { binding: 13, resource: pbd_dat.as_entire_binding() }] });
        let tex = |binding: u32| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false }, count: None };
        let bgl_grid = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("dust_grid_bgl"), entries: &[tex(0), tex(1), entry(2, wgpu::BufferBindingType::Storage { read_only: false }), tex(3)] });
        let dummy_buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_dummy_buf"), size: 16, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
        let dummy = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_dummy"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D3, format: wgpu::TextureFormat::Rgba16Float, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }).create_view(&Default::default());
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("dust_gpu"), source: wgpu::ShaderSource::Wgsl(crate::shaders::dust_gpu().into()) });
        let lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl, &bgl_grid], push_constant_ranges: &[] });
        let cp = |entry: &str| dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some(entry), layout: Some(&lay), module: &module, entry_point: Some(entry), compilation_options: Default::default(), cache: None });
        let (pl_emit, pl_step, pl_clear, pl_splat) = (cp("emit"), cp("dust_step"), cp("dust_clear"), cp("dust_splat"));
        let (pl_fmass, pl_flight) = (cp("dust_field_mass"), cp("dust_field_light"));
        let pl_dep = cp("dust_deposit");
        let pl_pbd = [cp("pbd_clear"), cp("pbd_bin"), cp("pbd_lambda"), cp("pbd_delta"), cp("pbd_apply")];
        let qs = if dev.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            Some((dev.create_query_set(&wgpu::QuerySetDescriptor { label: Some("dust_prof"), ty: wgpu::QueryType::Timestamp, count: 8 }),
                dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_prof_r"), size: 64, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false }),
                dev.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_prof_m"), size: 64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }), queue.get_timestamp_period()))
        } else { None };
        log::info!("dust gpu: {CAPACITY} particles, timestamps {}", qs.is_some());
        DustGpu { sdf: None, sdf_serial: 0, ubuf, coll_buf, cell_buf, emit_buf, state, mass_buf, fld_buf, render, bg, bgl_grid, dummy, pl_emit, pl_step, pl_clear, pl_splat, pl_fmass, pl_flight, dummy_buf, head: 0, cap: 0, last_time: -1.0, frame: 0, dep_buf, dep_cells, pl_dep, pbd_grid, pbd_dat, pl_pbd, qs: None, prof_frame: 0, ground_tex, heights: vec![0.0; (WINDOW * WINDOW) as usize], cells: vec![[i32::MIN; 2]; (WINDOW * WINDOW) as usize] }
    }

    pub fn sdf_serial(&self) -> u32 { self.sdf_serial }
    /// Uploads the body's signed distance field (R32Float, one texel per voxel).
    pub fn set_sdf(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, s: &crate::game::sdf::Sdf, serial: u32) {
        let t = dev.create_texture(&wgpu::TextureDescriptor { label: Some("dust_sdf"), size: wgpu::Extent3d { width: s.n[0] as u32, height: s.n[1] as u32, depth_or_array_layers: s.n[2] as u32 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D3, format: wgpu::TextureFormat::R32Float, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &t, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, bytemuck::cast_slice(&s.d),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(s.n[0] as u32 * 4), rows_per_image: Some(s.n[1] as u32) },
            wgpu::Extent3d { width: s.n[0] as u32, height: s.n[1] as u32, depth_or_array_layers: s.n[2] as u32 });
        self.sdf = Some((t.create_view(&Default::default()), [s.origin[0], s.origin[1], s.origin[2], s.cell], [s.n[0] as f32, s.n[1] as f32, s.n[2] as f32, 1.0]));
        self.sdf_serial = serial;
    }

    pub fn render_buffer(&self) -> &wgpu::Buffer { &self.render }
    /// How many instances to draw: the ring's size.
    pub fn count(&self) -> u32 { (self.cap + RS_N) as u32 }

    /// Emits the wheels' new particles and moves all of them to `frame.time`.
    /// `grid`: the airflow's velocity and solid textures and its box (see `AirflowGpu::fields`), or None (the wind only).
    pub fn update(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, terrain: &Terrain, frame: &DustGpuIn, props: &[&Prop], grid: Option<(&wgpu::TextureView, &wgpu::TextureView, [f32; 4], &wgpu::Buffer)>) {
        let cap = frame.cap.clamp(1, CAPACITY);
        if cap != self.cap { queue.write_buffer(&self.state, 0, &vec![0u8; (CAPACITY + RS_N) * std::mem::size_of::<S>()]); queue.write_buffer(&self.render, 0, &vec![0u8; (CAPACITY + RS_N) * DUST_INST_BYTES as usize]); self.head = 0; self.cap = cap; }
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
        // the trees and rocks near the car as colliders, binned into the ground window's cells
        if frame.opts & crate::game::dust::OPT_WORLD != 0 {
            let near: Vec<&Prop> = props.iter().copied().filter(|p| (p.pos.x - frame.car_pos.x).abs() < 34.0 && (p.pos.z - frame.car_pos.z).abs() < 34.0).collect();
            let cs = dust_ref::colliders_from_props(&near);
            let mut raw: Vec<f32> = Vec::with_capacity(cs.len() * 8);
            for c in &cs { raw.extend_from_slice(&[c.a.x, c.a.y, c.a.z, c.r, c.b.x, c.b.y, c.b.z, 0.0]); }
            if !raw.is_empty() { queue.write_buffer(&self.coll_buf, 0, bytemuck::cast_slice(&raw)); }
            let cells = dust_ref::bin_colliders(&cs, base, 0.5);
            queue.write_buffer(&self.cell_buf, 0, bytemuck::cast_slice(&cells));
        }
        let ph = phys();
        let mut u = DUni { n: [frame.emits.len().min(MAX_EMITS) as u32, self.head as u32, cap as u32, frame.opts], g: [dt, frame.time, frame.ground_y, 0.0],
            wind: [frame.wind.x, frame.wind.y, frame.wind.z, 0.0], ph: [ph.tau_fine, ph.tau_coarse, ph.settle_fine * SETTLE_FINE_SCALE, ph.settle_coarse * SETTLE_COARSE_SCALE], ph2: [ph.turbulence * 1.5, (frame.car_vel.x * frame.car_vel.x + frame.car_vel.z * frame.car_vel.z).sqrt(), ph.plume_lift, ph.plume_height],
            car: [frame.car_pos.x, frame.car_pos.y, frame.car_pos.z, frame.yaw], car_vel: [frame.car_vel.x, frame.car_vel.y, frame.car_vel.z, 0.0], bx: grid.map_or([0.0; 4], |g| g.2), bpos: [frame.body_pose.0.x, frame.body_pose.0.y, frame.body_pose.0.z, 0.0],
            bax0: [frame.body_pose.1[0].x, frame.body_pose.1[0].y, frame.body_pose.1[0].z, 0.0], bax1: [frame.body_pose.1[1].x, frame.body_pose.1[1].y, frame.body_pose.1[1].z, 0.0], bax2: [frame.body_pose.1[2].x, frame.body_pose.1[2].y, frame.body_pose.1[2].z, 0.0],
            sdf_o: self.sdf.as_ref().map_or([0.0; 4], |s| s.1), sdf_n: self.sdf.as_ref().map_or([0.0; 4], |s| s.2), rs: [cap as u32, RS_N as u32, self.frame, 0], pb: { let c = dust_ref::coupling(); [c.pbd_h, dust_ref::rest_density(c.pbd_spacing, c.pbd_h), 100.0, c.pbd_visc] }, gw: [base[0], base[1], 0, 0], cp: { let c = dust_ref::coupling(); [c.push, c.mass_fine, c.mass_coarse, ph.street] }, sun: [frame.sun.x, frame.sun.y, frame.sun.z, dust_ref::coupling().shadow_k], body: [frame.body[0], frame.body[1], frame.body[2], ph.vortex * frame.vortex_scale] };
        let emits: Vec<S> = frame.emits.iter().take(MAX_EMITS).map(|p| S { a: [p.pos.x, p.pos.y, p.pos.z, p.size0], b: [p.vel.x, p.vel.y, p.vel.z, p.age], c: [p.color[0], p.color[1], p.color[2], p.life], d: [if p.smoke { 3.0 } else if p.speck { 2.0 } else if p.coarse { 1.0 } else { 0.0 }, p.seed, 1.0, 0.0] }).collect();
        if !emits.is_empty() { queue.write_buffer(&self.emit_buf, 0, bytemuck::cast_slice(&emits)); }
        self.head = (self.head + u.n[0] as usize) % cap;
        self.frame = self.frame.wrapping_add(1);
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let prof = self.qs.is_some() && crate::raster::prof_on();
        let tw = |k: u32| -> Option<wgpu::ComputePassTimestampWrites> { self.qs.as_ref().filter(|_| prof).map(|q| wgpu::ComputePassTimestampWrites { query_set: &q.0, beginning_of_pass_write_index: Some(2 * k), end_of_pass_write_index: Some(2 * k + 1) }) };
        let (vel, solid) = grid.map_or((&self.dummy, &self.dummy), |g| (g.0, g.1));
        let sdf_view = self.sdf.as_ref().map_or(&self.dummy, |s| &s.0);
        let dens = grid.map_or(&self.dummy_buf, |g| g.3);
        let gbg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("dust_grid_bg"), layout: &self.bgl_grid, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(vel) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(solid) }, wgpu::BindGroupEntry { binding: 2, resource: dens.as_entire_binding() }, wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(sdf_view) }] });
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_emit_pbd"), timestamp_writes: tw(0) });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        cp.set_pipeline(&self.pl_emit);
        cp.dispatch_workgroups(((u.n[0] + 63) / 64).max(1), 1, 1);
        // the particle pressure: the near particles (every 8th) are binned into the box's cells and pushed apart by the density constraint, a few iterations
        if frame.opts & crate::game::dust::OPT_PBD != 0 && grid.is_some() {
            let g = grid.as_ref().unwrap();
            let cells = ((g.2[0] / g.2[3]).round() as u32 * (g.2[1] / g.2[3]).round() as u32 * (g.2[2] / g.2[3]).round() as u32).min(MAX_CELLS as u32);
            let parts = (((cap + RS_N) / PBD_STRIDE + 1) as u32 + 63) / 64;
            for _ in 0..dust_ref::coupling().pbd_iters {
                cp.set_pipeline(&self.pl_pbd[0]); cp.dispatch_workgroups((cells + 63) / 64, 1, 1);
                cp.set_pipeline(&self.pl_pbd[1]); cp.dispatch_workgroups(parts, 1, 1);
                cp.set_pipeline(&self.pl_pbd[2]); cp.dispatch_workgroups(parts, 1, 1);
                cp.set_pipeline(&self.pl_pbd[3]); cp.dispatch_workgroups(parts, 1, 1);
                cp.set_pipeline(&self.pl_pbd[4]); cp.dispatch_workgroups(parts, 1, 1);
            }
        }
        drop(cp);
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_step"), timestamp_writes: tw(1) });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        cp.set_pipeline(&self.pl_step);
        cp.dispatch_workgroups((((cap + RS_N) as u32) + 63) / 64, 1, 1);
        drop(cp);
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_deposit"), timestamp_writes: tw(2) });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        // the dust field of this frame (read by the next frame's step and by the airflow solver): clear, then every particle adds itself
        // the settled layer: the cells decay, hand over, show on the ground and give back to the wake
        if frame.opts & crate::game::dust::OPT_DEPOSIT != 0 {
            cp.set_pipeline(&self.pl_dep);
            cp.dispatch_workgroups((WINDOW * WINDOW / 64) as u32, 1, 1);
        }
        drop(cp);
        let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("dust_field"), timestamp_writes: tw(3) });
        cp.set_bind_group(0, &self.bg, &[]);
        cp.set_bind_group(1, &gbg, &[]);
        if let Some(g) = grid {
            let cells = (g.2[0] / g.2[3]).round() as u32 * (g.2[1] / g.2[3]).round() as u32 * (g.2[2] / g.2[3]).round() as u32;
            cp.set_pipeline(&self.pl_clear);
            cp.dispatch_workgroups((cells * 4 + 63) / 64, 1, 1);
            cp.set_pipeline(&self.pl_splat);
            cp.dispatch_workgroups((((cap + RS_N) as u32) + 63) / 64, 1, 1);
            let groups = ((cells.min(MAX_CELLS as u32)) + 63) / 64;
            cp.set_pipeline(&self.pl_fmass);
            cp.dispatch_workgroups(groups, 1, 1);
            cp.set_pipeline(&self.pl_flight);
            cp.dispatch_workgroups(groups, 1, 1);
        }
        drop(cp);
        if let (true, Some(q)) = (prof, self.qs.as_ref()) { enc.resolve_query_set(&q.0, 0..8, &q.1, 0); enc.copy_buffer_to_buffer(&q.1, 0, &q.2, 0, 64); }
    }

    /// After the submit: every 90th frame wait and log the four passes' GPU times.
    pub fn prof_report(&mut self, dev: &wgpu::Device) {
        let Some(q) = self.qs.as_ref() else { return };
        if !crate::raster::prof_on() { return; }
        self.prof_frame += 1;
        if self.prof_frame % 90 != 0 { return; }
        let slice = q.2.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        dev.poll(wgpu::Maintain::Wait);
        let ts: Vec<u64> = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();
        q.2.unmap();
        let ms = |k: usize| ts[2 * k + 1].wrapping_sub(ts[2 * k]) as f32 * q.3 / 1.0e6;
        log::info!("dustprof ms: emit+pbd {:.2}, step {:.2}, deposit {:.2}, field {:.2}", ms(0), ms(1), ms(2), ms(3));
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
