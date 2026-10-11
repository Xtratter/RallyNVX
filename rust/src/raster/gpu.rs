//! wgpu side of the raster renderer.
use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[cfg(target_os = "android")]
use ndk::native_window::NativeWindow;
#[cfg(target_os = "android")]
use raw_window_handle::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::meshes;
use crate::game::math::*;
use crate::game::props::PropKind;
use crate::game::terrain::{Mesh, TILE_SIZE};
use crate::game::world::{tile_of, World};

const SHADOW_SIZE: u32 = 2048;
const DRAW_TILES: i32 = 3;
const MAX_INST: usize = 4096;
const TILE_BUDGET: usize = 3;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uni {
    vp: [f32; 16], light_vp: [f32; 16], inv_vp: [f32; 16],
    cam: [f32; 4], sun: [f32; 4], fog: [f32; 4], sky_top: [f32; 4], sky_hor: [f32; 4],
}
const _: () = assert!(std::mem::size_of::<Uni>() == 272);

/// One instance: model matrix, per-axis scale (for normals), colour. Matches the instance vertex layout (96 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Inst { m: [f32; 16], scale: [f32; 4], color: [f32; 4] }

fn inst(rot: Q, pos: V3, s: V3, color: [f32; 3]) -> Inst {
    let (x, y, z) = (rot.rotate(v3(1., 0., 0.)) * s.x, rot.rotate(v3(0., 1., 0.)) * s.y, rot.rotate(v3(0., 0., 1.)) * s.z);
    Inst { m: [x.x, x.y, x.z, 0., y.x, y.y, y.z, 0., z.x, z.y, z.z, 0., pos.x, pos.y, pos.z, 1.],
        scale: [s.x, s.y, s.z, 0.], color: [color[0], color[1], color[2], 1.] }
}

struct MeshBuf { vb: wgpu::Buffer, ib: wgpu::Buffer, n: u32 }
fn mesh_buf(dev: &wgpu::Device, m: &Mesh) -> MeshBuf {
    MeshBuf {
        vb: dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("mesh_v"), contents: bytemuck::cast_slice(&m.verts), usage: wgpu::BufferUsages::VERTEX }),
        ib: dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("mesh_i"), contents: bytemuck::cast_slice(&m.idx), usage: wgpu::BufferUsages::INDEX }),
        n: m.idx.len() as u32,
    }
}

const MESH_ATTRS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
const INST_ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];

fn depth_tex(dev: &wgpu::Device, w: u32, h: u32, sampled: bool) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = dev.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"), size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | if sampled { wgpu::TextureUsages::TEXTURE_BINDING } else { wgpu::TextureUsages::empty() },
        view_formats: &[] });
    let view = tex.create_view(&Default::default());
    (tex, view)
}

pub struct Raster {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: (wgpu::Texture, wgpu::TextureView),
    shadow: (wgpu::Texture, wgpu::TextureView),
    ubuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    bg_main: wgpu::BindGroup,
    bg_shadow: wgpu::BindGroup,
    pl_terrain: wgpu::RenderPipeline,
    pl_terrain_sh: wgpu::RenderPipeline,
    pl_inst: wgpu::RenderPipeline,
    pl_inst_sh: wgpu::RenderPipeline,
    pl_sky: wgpu::RenderPipeline,
    meshes: [MeshBuf; 4], // cube, cylinder, cone, sphere
    tiles: HashMap<(i32, i32, u32), MeshBuf>,
    _keep: Box<dyn std::any::Any>, // native window / winit window; dropped after `surface` (field order)
}

impl Raster {
    #[cfg(target_os = "android")]
    pub fn new(window: NativeWindow, w: u32, h: u32) -> Raster {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN, ..Default::default() });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Android(AndroidDisplayHandle::new()),
                raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(window.ptr().cast())),
            })
        }.expect("create_surface");
        Raster::from_surface(instance, surface, Box::new(window), w, h)
    }

    pub fn from_surface(instance: wgpu::Instance, surface: wgpu::Surface<'static>, keep: Box<dyn std::any::Any>, w: u32, h: u32) -> Raster {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("no GPU adapter");
        let info = adapter.get_info();
        log::info!("adapter: {} ({:?}) {}", info.name, info.backend, info.driver_info);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("game_device"), required_features: wgpu::Features::empty(), required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        }, None)).expect("request_device");
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: w.max(1), height: h.max(1),
            present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0], view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth = depth_tex(&device, config.width, config.height, false);
        let shadow = depth_tex(&device, SHADOW_SIZE, SHADOW_SIZE, true);
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("uni"), size: std::mem::size_of::<Uni>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("inst"), size: (MAX_INST * std::mem::size_of::<Inst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let smp = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("shadow_smp"), mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear, compare: Some(wgpu::CompareFunction::LessEqual), ..Default::default() });

        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let ubo = |vis| wgpu::BindGroupLayoutEntry { binding: 0, visibility: vis, count: None,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None } };
        let bgl_main = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("raster_main"), entries: &[
            ubo(vf),
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison) },
        ] });
        let bgl_shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("raster_shadow"), entries: &[ubo(wgpu::ShaderStages::VERTEX)] });
        let bg_main = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("raster_main"), layout: &bgl_main, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow.1) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&smp) },
        ] });
        let bg_shadow = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("raster_shadow"), layout: &bgl_shadow,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }] });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("raster"), source: wgpu::ShaderSource::Wgsl(crate::shaders::raster().into()) });
        let lay_main = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl_main], push_constant_ranges: &[] });
        let lay_shadow = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl_shadow], push_constant_ranges: &[] });
        let mesh_layout = wgpu::VertexBufferLayout { array_stride: 24, step_mode: wgpu::VertexStepMode::Vertex, attributes: &MESH_ATTRS };
        let inst_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Inst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &INST_ATTRS };
        let depth_state = |write: bool, cmp: wgpu::CompareFunction, bias: wgpu::DepthBiasState| Some(wgpu::DepthStencilState {
            format: DEPTH, depth_write_enabled: write, depth_compare: cmp, stencil: Default::default(), bias });
        let prim = wgpu::PrimitiveState { cull_mode: None, ..Default::default() };
        let target = [Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })];
        let no_bias = wgpu::DepthBiasState::default();
        let sh_bias = wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 };
        let mk = |label: &str, lay: &wgpu::PipelineLayout, vs: &str, fs: Option<&str>, bufs: &[wgpu::VertexBufferLayout], ds| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label), layout: Some(lay),
                vertex: wgpu::VertexState { module: &module, entry_point: Some(vs), compilation_options: Default::default(), buffers: bufs },
                fragment: fs.map(|f| wgpu::FragmentState { module: &module, entry_point: Some(f), compilation_options: Default::default(), targets: &target }),
                primitive: prim, depth_stencil: ds, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
            })
        };
        let pl_terrain = mk("terrain", &lay_main, "vs_terrain", Some("fs_terrain"), &[mesh_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_terrain_sh = mk("terrain_sh", &lay_shadow, "vs_terrain_shadow", None, &[mesh_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_inst = mk("inst", &lay_main, "vs_inst", Some("fs_inst"), &[mesh_layout.clone(), inst_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_inst_sh = mk("inst_sh", &lay_shadow, "vs_inst_shadow", None, &[mesh_layout, inst_layout], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_sky = mk("sky", &lay_main, "vs_sky", Some("fs_sky"), &[], depth_state(false, wgpu::CompareFunction::Always, no_bias));

        let meshes = [mesh_buf(&device, &meshes::cube()), mesh_buf(&device, &meshes::cylinder()), mesh_buf(&device, &meshes::cone()), mesh_buf(&device, &meshes::sphere())];
        Raster { surface, device, queue, config, depth, shadow, ubuf, ibuf, bg_main, bg_shadow,
            pl_terrain, pl_terrain_sh, pl_inst, pl_inst_sh, pl_sky, meshes, tiles: HashMap::new(), _keep: keep }
    }

    pub fn device(&self) -> &wgpu::Device { &self.device }
    pub fn queue(&self) -> &wgpu::Queue { &self.queue }
    pub fn format(&self) -> wgpu::TextureFormat { self.config.format }

    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if w == self.config.width && h == self.config.height { return; }
        self.config.width = w; self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_tex(&self.device, w, h, false);
    }

    /// Draws one frame. A lost or outdated surface is reconfigured and the frame skipped (never panics).
    pub fn render(&mut self, world: &mut World) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => { self.surface.configure(&self.device, &self.config); return; }
            Err(e) => { log::warn!("surface error: {:?}", e); return; }
        };
        let view = frame.texture.create_view(&Default::default());
        let (cx, cz) = (tile_of(world.cam.pos.x), tile_of(world.cam.pos.z));
        world.ensure_props(cx, cz, DRAW_TILES);
        let wv = world.view(DRAW_TILES);

        // camera and light matrices
        let aspect = self.config.width as f32 / self.config.height as f32;
        let vp = mat_mul(perspective(1.0, aspect, 0.3, 900.0), look_at(wv.cam_pos, wv.cam_target, v3(0., 1., 0.)));
        let centre = wv.vehicle.pos;
        let up = if wv.sun.y.abs() > 0.99 { v3(0., 0., 1.) } else { v3(0., 1., 0.) };
        let light_vp = mat_mul(ortho(-70., 70., -70., 70., 1., 300.), look_at(centre + wv.sun * 150.0, centre, up));
        let uni = Uni { vp: flatten(vp), light_vp: flatten(light_vp), inv_vp: flatten(inverse(vp)),
            cam: [wv.cam_pos.x, wv.cam_pos.y, wv.cam_pos.z, 0.], sun: [wv.sun.x, wv.sun.y, wv.sun.z, 0.],
            fog: [1.0 / 330.0, 0., 0., 0.], sky_top: [0.22, 0.42, 0.80, 0.], sky_hor: [0.70, 0.80, 0.90, 0.] };
        self.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&uni));

        // terrain tiles: nearest first, a few per frame; the old LOD stays drawn until its replacement exists
        let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
        let (build, visible) = super::tile_plan(cx, cz, &have, DRAW_TILES);
        for &(tx, tz, lod) in build.iter().take(TILE_BUDGET) {
            self.tiles.insert((tx, tz, lod), mesh_buf(&self.device, &wv.terrain.tile_mesh(tx, tz, lod)));
        }
        let (_, visible) = if build.len() > TILE_BUDGET || !build.is_empty() {
            let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
            super::tile_plan(cx, cz, &have, DRAW_TILES)
        } else { (build, visible) };
        let keep: HashSet<_> = visible.iter().copied().collect();
        self.tiles.retain(|k, _| keep.contains(k));

        // instances: cube, cylinder, cone, sphere
        let mut lists: [Vec<Inst>; 4] = Default::default();
        let veh = wv.vehicle;
        let (rot, sp) = (veh.rot, veh.spec);
        lists[0].push(inst(rot, veh.pos + rot.rotate(v3(0., 0.35, 0.)), v3(1.9, 0.7, 4.2), [0.75, 0.12, 0.08]));
        lists[0].push(inst(rot, veh.pos + rot.rotate(v3(0., 0.95, -0.2)), v3(1.7, 0.6, 2.0), [0.12, 0.14, 0.17]));
        for w in 0..4 {
            let local = sp.wheels[w] - v3(0., sp.rest - veh.comp[w], 0.);
            let steer = if w < 2 { veh.steer } else { 0.0 };
            let r = rot.mul(Q::axis_angle(v3(0., 1., 0.), -steer)).mul(Q::axis_angle(v3(0., 0., 1.), std::f32::consts::FRAC_PI_2));
            lists[1].push(inst(r, veh.pos + rot.rotate(local), v3(sp.radius * 2.0, 0.35, sp.radius * 2.0), [0.07, 0.07, 0.08]));
        }
        for mk in &wv.markers {
            use crate::game::world::MarkerKind;
            match mk.kind {
                MarkerKind::Beacon => lists[1].push(inst(Q::IDENT, mk.pos + v3(0., 30., 0.), v3(3.0, 60.0, 3.0), [1.0, 0.85, 0.25])),
                MarkerKind::Ring => lists[1].push(inst(Q::IDENT, mk.pos + v3(0., 0.3, 0.), v3(mk.radius * 2.0, 0.4, mk.radius * 2.0), [0.25, 0.75, 1.0])),
                MarkerKind::Next => {
                    lists[1].push(inst(Q::IDENT, mk.pos + v3(0., 0.3, 0.), v3(mk.radius * 2.0, 0.4, mk.radius * 2.0), [1.0, 0.35, 0.85]));
                    lists[1].push(inst(Q::IDENT, mk.pos + v3(0., 7.5, 0.), v3(1.5, 15.0, 1.5), [1.0, 0.35, 0.85]));
                }
            }
        }
        for p in &wv.props {
            match p.kind {
                PropKind::Rock => {
                    let g = 0.35 + 0.15 * (p.pos.x * 0.37 + p.pos.z * 0.11).fract().abs();
                    lists[3].push(inst(Q::IDENT, p.pos + v3(0., p.radius * 0.4, 0.), v3(p.radius * 2.0, p.radius * 1.4, p.radius * 2.0), [g, g * 0.97, g * 0.93]));
                }
                PropKind::Tree => {
                    lists[1].push(inst(Q::IDENT, p.pos + v3(0., p.height * 0.5, 0.), v3(0.7, p.height, 0.7), [0.30, 0.20, 0.12]));
                    let t = (p.pos.x * 0.13 + p.pos.z * 0.29).fract().abs();
                    lists[2].push(inst(Q::IDENT, p.pos + v3(0., p.height * 1.35, 0.), v3(3.2, p.height * 0.9, 3.2), [0.08 + 0.05 * t, 0.28 + 0.1 * t, 0.09]));
                }
            }
        }
        let mut all: Vec<Inst> = Vec::new();
        let mut ranges = [(0u32, 0u32); 4];
        for k in 0..4 {
            let room = MAX_INST - all.len();
            let take = lists[k].len().min(room);
            ranges[k] = (all.len() as u32, (all.len() + take) as u32);
            all.extend_from_slice(&lists[k][..take]);
        }
        if !all.is_empty() { self.queue.write_buffer(&self.ibuf, 0, bytemuck::cast_slice(&all)); }

        let near = |tx: i32, tz: i32| {
            let c = v3((tx as f32 + 0.5) * TILE_SIZE, centre.y, (tz as f32 + 0.5) * TILE_SIZE);
            let d = c - centre; d.x.hypot(d.z) < 180.0
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"), color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &self.shadow.1,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                timestamp_writes: None, occlusion_query_set: None });
            rp.set_pipeline(&self.pl_terrain_sh);
            rp.set_bind_group(0, &self.bg_shadow, &[]);
            for (&(tx, tz, lod), m) in &self.tiles {
                if lod > 1 || !near(tx, tz) { continue; }
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, 0..1);
            }
            rp.set_pipeline(&self.pl_inst_sh);
            rp.set_vertex_buffer(1, self.ibuf.slice(..));
            for k in 0..4 {
                let (a, b) = ranges[k]; if b == a { continue; }
                let m = &self.meshes[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
        }
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.6, b: 0.7, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &self.depth.1,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }), stencil_ops: None }),
                timestamp_writes: None, occlusion_query_set: None });
            rp.set_bind_group(0, &self.bg_main, &[]);
            rp.set_pipeline(&self.pl_sky);
            rp.draw(0..3, 0..1);
            rp.set_pipeline(&self.pl_terrain);
            for m in self.tiles.values() {
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, 0..1);
            }
            rp.set_pipeline(&self.pl_inst);
            rp.set_vertex_buffer(1, self.ibuf.slice(..));
            for k in 0..4 {
                let (a, b) = ranges[k]; if b == a { continue; }
                let m = &self.meshes[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }
}
