//! wgpu side of the raster renderer.
use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[cfg(target_os = "android")]
use ndk::native_window::NativeWindow;
#[cfg(target_os = "android")]
use raw_window_handle::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::{citygen, meshes, models::{self, Part}, nature_models, texgen, vehicle_models as vm};
use crate::game::math::*;
use crate::game::props::PropKind;
use crate::game::scatter::{self, Bush};
use crate::game::terrain::{Mesh, TILE_SIZE};
use crate::game::world::{tile_of, World};

const SHADOW_SIZE: u32 = 2048;
const DRAW_TILES: i32 = 3;
const MAX_INST: usize = 16384;
const CITY_BUDGET: usize = 6000;
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

/// Like `inst`, but flagged as a glowing marker (colour alpha 2): drawn unlit and fog-free.
fn inst_glow(rot: Q, pos: V3, s: V3, color: [f32; 3]) -> Inst { let mut i = inst(rot, pos, s, color); i.color[3] = 2.0; i }

/// A building: lit like a normal part, with windows drawn by the shader (colour alpha 3).
fn inst_facade(rot: Q, pos: V3, s: V3, color: [f32; 3]) -> Inst { let mut i = inst(rot, pos, s, color); i.color[3] = 3.0; i }

fn push_part(lists: &mut [Vec<Inst>; 4], p: &Part) {
    lists[p.mesh].push(if p.facade { inst_facade(p.rot, p.pos, p.scale, p.color) } else if p.glow { inst_glow(p.rot, p.pos, p.scale, p.color) } else { inst(p.rot, p.pos, p.scale, p.color) });
}

struct MeshBuf { vb: wgpu::Buffer, ib: wgpu::Buffer, n: u32, wb: Option<wgpu::Buffer> }

/// A vehicle model mesh in the model pipeline's vertex format (28 bytes).
struct ModelBuf { vb: wgpu::Buffer, ib: wgpu::Buffer, n: u32 }
fn model_buf(dev: &wgpu::Device, m: &vm::Mesh3) -> ModelBuf {
    ModelBuf {
        vb: dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("model_v"), contents: bytemuck::cast_slice(&m.verts), usage: wgpu::BufferUsages::VERTEX }),
        ib: dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("model_i"), contents: bytemuck::cast_slice(&m.idx), usage: wgpu::BufferUsages::INDEX }),
        n: m.idx.len() as u32,
    }
}

/// One model instance: a rigid model matrix and a tint (80 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MInst { m: [f32; 16], tint: [f32; 4] }
fn mmat_s(q: Q, p: V3, s: f32) -> [f32; 16] {
    let m = mmat(q, p);
    [m[0] * s, m[1] * s, m[2] * s, 0., m[4] * s, m[5] * s, m[6] * s, 0., m[8] * s, m[9] * s, m[10] * s, 0., p.x, p.y, p.z, 1.]
}
/// Pseudo-random heading from a position (stable per prop).
fn yaw_of(p: V3) -> f32 { ((p.x * 12.9898 + p.z * 78.233).sin() * 43758.547).fract().abs() * std::f32::consts::TAU }
fn mmat(q: Q, p: V3) -> [f32; 16] {
    let (x, y, z) = (q.rotate(v3(1., 0., 0.)), q.rotate(v3(0., 1., 0.)), q.rotate(v3(0., 0., 1.)));
    [x.x, x.y, x.z, 0., y.x, y.y, y.z, 0., z.x, z.y, z.z, 0., p.x, p.y, p.z, 1.]
}
// slots: car body, wheel, heli body, rotor, tail rotor, drone body, propeller, 4 trees, 2 bushes, 3 rocks, grass tuft
const SLOT_TREE: usize = 7;
const SLOT_BUSH: usize = 11;
const SLOT_ROCK: usize = 13;
const SLOT_GRASS: usize = 16;
const MODEL_SLOTS: usize = 17;
const MAX_MODEL_INST: usize = 6144;
const NEAR_SHADOW: f32 = 90.0;   // plants within this distance of the car cast shadows
const MODEL_ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4];
const MINST_ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];
const WEIGHT_ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![2 => Unorm8x4];
fn mesh_buf(dev: &wgpu::Device, m: &Mesh) -> MeshBuf {
    MeshBuf {
        wb: None,
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
    city_radius: f32,
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
    pl_model: wgpu::RenderPipeline,
    pl_model_sh: wgpu::RenderPipeline,
    models: Vec<ModelBuf>,
    mbuf: wgpu::Buffer,
    bush_cache: HashMap<(i32, i32), Vec<Bush>>,
    grass_cache: Option<((i32, i32, usize), Vec<(V3, f32)>)>,
    wheel_spin: f32,
    last_time: f32,
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

        // generated material maps: one texture array, 5 layers x 9 mips
        let mat_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("materials"), size: wgpu::Extent3d { width: texgen::SIZE as u32, height: texgen::SIZE as u32, depth_or_array_layers: texgen::LAYERS as u32 },
            mip_level_count: 9, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        for (layer, m) in texgen::Material::ALL.iter().enumerate() {
            for (level, data) in texgen::mips(&texgen::make(*m, texgen::SIZE), texgen::SIZE).iter().enumerate() {
                let s = (texgen::SIZE >> level) as u32;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo { texture: &mat_tex, mip_level: level as u32, origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 }, aspect: wgpu::TextureAspect::All },
                    data, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(s * 4), rows_per_image: Some(s) },
                    wgpu::Extent3d { width: s, height: s, depth_or_array_layers: 1 });
            }
        }
        let mat_view = mat_tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let mat_smp = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("materials_smp"),
            address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat, address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear, anisotropy_clamp: 4, ..Default::default() });
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let ubo = |vis| wgpu::BindGroupLayoutEntry { binding: 0, visibility: vis, count: None,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None } };
        let bgl_main = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("raster_main"), entries: &[
            ubo(vf),
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison) },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering) },
        ] });
        let bgl_shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("raster_shadow"), entries: &[ubo(wgpu::ShaderStages::VERTEX)] });
        let bg_main = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("raster_main"), layout: &bgl_main, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow.1) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&smp) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&mat_view) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&mat_smp) },
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
        let weight_layout = wgpu::VertexBufferLayout { array_stride: 4, step_mode: wgpu::VertexStepMode::Vertex, attributes: &WEIGHT_ATTRS };
        let pl_terrain = mk("terrain", &lay_main, "vs_terrain", Some("fs_terrain"), &[mesh_layout.clone(), weight_layout], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_terrain_sh = mk("terrain_sh", &lay_shadow, "vs_terrain_shadow", None, &[mesh_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_inst = mk("inst", &lay_main, "vs_inst", Some("fs_inst"), &[mesh_layout.clone(), inst_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_inst_sh = mk("inst_sh", &lay_shadow, "vs_inst_shadow", None, &[mesh_layout, inst_layout], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let model_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<vm::Vert>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &MODEL_ATTRS };
        let minst_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<MInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &MINST_ATTRS };
        let pl_model = mk("model", &lay_main, "vs_model", Some("fs_model"), &[model_layout.clone(), minst_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_model_sh = mk("model_sh", &lay_shadow, "vs_model_shadow", None, &[model_layout, minst_layout], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_sky = mk("sky", &lay_main, "vs_sky", Some("fs_sky"), &[], depth_state(false, wgpu::CompareFunction::Always, no_bias));

        let (car, heli, drone) = (vm::build_car(&crate::game::vehicle::VehicleSpec::offroader()), vm::build_heli(&crate::game::flight::AircraftSpec::of(crate::game::flight::AircraftKind::Heli)), vm::build_drone(&crate::game::flight::AircraftSpec::of(crate::game::flight::AircraftKind::Drone)));
        let nat = nature_models::build_all();
        let models: Vec<ModelBuf> = [&car.body, &car.wheel, &heli.body, &heli.rotor, &heli.tail_rotor, &drone.body, &drone.prop].into_iter()
            .chain(nat.trees.iter()).chain(nat.bushes.iter()).chain(nat.rocks.iter()).chain(std::iter::once(&nat.grass)).map(|m| model_buf(&device, m)).collect();
        assert_eq!(models.len(), MODEL_SLOTS);
        let mbuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("model_inst"), size: (MAX_MODEL_INST * std::mem::size_of::<MInst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let meshes = [mesh_buf(&device, &meshes::cube()), mesh_buf(&device, &meshes::cylinder()), mesh_buf(&device, &meshes::cone()), mesh_buf(&device, &meshes::sphere())];
        Raster { city_radius: 450.0, surface, device, queue, config, depth, shadow, ubuf, ibuf, bg_main, bg_shadow,
            pl_terrain, pl_terrain_sh, pl_inst, pl_inst_sh, pl_sky, pl_model, pl_model_sh, models, mbuf, bush_cache: HashMap::new(), grass_cache: None, wheel_spin: 0.0, last_time: 0.0, meshes, tiles: HashMap::new(), _keep: keep }
    }

    pub fn device(&self) -> &wgpu::Device { &self.device }
    pub fn queue(&self) -> &wgpu::Queue { &self.queue }
    pub fn format(&self) -> wgpu::TextureFormat { self.config.format }

    /// How far from the camera city buildings and tiles are drawn (metres).
    pub fn set_city_radius(&mut self, r: f32) { self.city_radius = r.clamp(100.0, 2000.0); }

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
        let vp = mat_mul(perspective(1.0, aspect, 0.3, 2500.0), look_at(wv.cam_pos, wv.cam_target, v3(0., 1., 0.)));
        let centre = wv.vehicle.pos;
        let up = if wv.sun.y.abs() > 0.99 { v3(0., 0., 1.) } else { v3(0., 1., 0.) };
        let light_vp = mat_mul(ortho(-70., 70., -70., 70., 1., 300.), look_at(centre + wv.sun * 150.0, centre, up));
        let uni = Uni { vp: flatten(vp), light_vp: flatten(light_vp), inv_vp: flatten(inverse(vp)),
            cam: [wv.cam_pos.x, wv.cam_pos.y, wv.cam_pos.z, 0.], sun: [wv.sun.x, wv.sun.y, wv.sun.z, 0.],
            fog: [1.0 / 330.0, wv.time, 0., 0.], sky_top: [0.22, 0.42, 0.80, 0.], sky_hor: [0.70, 0.80, 0.90, 0.] };
        self.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&uni));

        // terrain tiles: nearest first, a few per frame; the old LOD stays drawn until its replacement exists
        let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
        let (build, visible) = super::tile_plan(cx, cz, &have, DRAW_TILES);
        for &(tx, tz, lod) in build.iter().take(TILE_BUDGET) {
            let (mesh, w) = wv.terrain.tile_mesh_with_weights(tx, tz, lod);
            let mut mb = mesh_buf(&self.device, &mesh);
            mb.wb = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("tile_w"), contents: bytemuck::cast_slice(&w), usage: wgpu::BufferUsages::VERTEX }));
            self.tiles.insert((tx, tz, lod), mb);
        }
        let (_, visible) = if build.len() > TILE_BUDGET || !build.is_empty() {
            let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
            super::tile_plan(cx, cz, &have, DRAW_TILES)
        } else { (build, visible) };
        let keep: HashSet<_> = visible.iter().copied().collect();
        self.tiles.retain(|k, _| keep.contains(k));

        // instances: cube, cylinder, cone, sphere
        let mut lists: [Vec<Inst>; 4] = Default::default();
        // vehicle models: per slot lists of (rotation, position)
        let mut ml: [Vec<MInst>; MODEL_SLOTS] = Default::default();
        let dt = (wv.time - self.last_time).clamp(0.0, 0.1);
        self.last_time = wv.time;
        if let Some(ac) = &wv.aircraft {
            let mi = |(q, p): (Q, V3), tint: [f32; 4]| MInst { m: mmat(q, p), tint };
            match ac.kind {
                crate::game::flight::AircraftKind::Heli => {
                    let h = vm::heli_poses(ac, wv.time);
                    ml[2].push(mi(h.body, [1.0; 4])); ml[3].push(mi(h.rotor, [1.0; 4])); ml[4].push(mi(h.tail, [1.0; 4]));
                }
                _ => {
                    let (body, props) = vm::drone_poses(ac, wv.time);
                    ml[5].push(mi(body, [1.0; 4]));
                    for p in props { ml[6].push(mi(p, [1.0; 4])); }
                }
            }
        } else {
            let veh = wv.vehicle;
            self.wheel_spin += veh.speed() / veh.spec.radius * dt;
            self.wheel_spin %= std::f32::consts::TAU;
            let (bq, bp) = vm::car_body_pose(veh);
            ml[0].push(MInst { m: mmat(bq, bp), tint: [1.0; 4] });
            for p in vm::car_wheel_poses(veh, self.wheel_spin) { ml[1].push(MInst { m: mmat(p.0, p.1), tint: [1.0; 4] }); }
        }
        for tr in wv.tracks.iter() {
            lists[0].push(inst(crate::game::tracks::track_rot(tr.dir, tr.normal), tr.pos + tr.normal * 0.03, v3(0.3, 0.02, 0.45), [0.10, 0.075, 0.05]));
        }
        if let Some(ac) = &wv.aircraft {
            if let Some(s) = &wv.sling {
                if s.attached { push_part(&mut lists, &models::rope_part(ac.pos + ac.rotation().rotate(v3(0., vm::HELI_LIFT - 1.3, 0.)), s.pos)); }
                push_part(&mut lists, &models::cargo_part(s.pos));
            }
        }
        for mk in &wv.markers {
            use crate::game::world::MarkerKind;
            match mk.kind {
                MarkerKind::Beacon => lists[1].push(inst_glow(Q::IDENT, mk.pos + v3(0., 60., 0.), v3(7.0, 120.0, 7.0), [1.0, 0.85, 0.25])),
                MarkerKind::Ring => lists[1].push(inst_glow(Q::IDENT, mk.pos + v3(0., 0.3, 0.), v3(mk.radius * 2.0, 0.4, mk.radius * 2.0), [0.25, 0.75, 1.0])),
                MarkerKind::AirRing | MarkerKind::AirNext => for p in models::ring_parts(mk.pos, mk.normal, mk.radius, mk.kind == MarkerKind::AirNext) { push_part(&mut lists, &p); },
                MarkerKind::Pad => push_part(&mut lists, &models::pad_part(mk.pos, mk.radius)),
                MarkerKind::Zone => for p in models::zone_parts(mk.pos, wv.terrain.height(mk.pos.x, mk.pos.z), mk.radius) { push_part(&mut lists, &p); },
                MarkerKind::Next => {
                    lists[1].push(inst_glow(Q::IDENT, mk.pos + v3(0., 0.3, 0.), v3(mk.radius * 2.0, 0.4, mk.radius * 2.0), [1.0, 0.35, 0.85]));
                    lists[1].push(inst_glow(Q::IDENT, mk.pos + v3(0., 7.5, 0.), v3(1.5, 15.0, 1.5), [1.0, 0.35, 0.85]));
                }
            }
        }
        // plants and rocks: generated models, the ones near the car first (they cast shadows)
        let vpos = wv.vehicle.pos;
        let draw_r = self.city_radius;
        let mut far: [Vec<MInst>; MODEL_SLOTS] = Default::default();
        let mut put = |ml: &mut [Vec<MInst>; MODEL_SLOTS], slot: usize, p: V3, m: MInst| {
            let d = (p.x - vpos.x).hypot(p.z - vpos.z);
            if d < NEAR_SHADOW { ml[slot].push(m); } else if d < draw_r { far[slot].push(m); }
        };
        for p in &wv.props {
            let yaw = Q::axis_angle(v3(0., 1., 0.), yaw_of(p.pos));
            let k = nature_models::tint(p.pos); let tint = [k[0], k[1], k[2], 1.0];
            match p.kind {
                PropKind::Rock => put(&mut ml, SLOT_ROCK + nature_models::variant(2, p.pos), p.pos, MInst { m: mmat_s(yaw, p.pos + v3(0., 0.4 * p.radius, 0.), p.radius), tint }),
                PropKind::Tree => put(&mut ml, SLOT_TREE + nature_models::variant(0, p.pos), p.pos, MInst { m: mmat_s(yaw, p.pos, p.height), tint }),
            }
        }
        let (bx, bz) = (tile_of(wv.cam_pos.x), tile_of(wv.cam_pos.z));
        for tz in bz - DRAW_TILES..=bz + DRAW_TILES { for tx in bx - DRAW_TILES..=bx + DRAW_TILES {
            let bushes = self.bush_cache.entry((tx, tz)).or_insert_with(|| scatter::tile_bushes(wv.terrain, tx, tz));
            for b in bushes.iter() {
                let k = nature_models::tint(b.pos);
                put(&mut ml, SLOT_BUSH + nature_models::variant(1, b.pos), b.pos, MInst { m: mmat_s(Q::axis_angle(v3(0., 1., 0.), yaw_of(b.pos)), b.pos, b.scale), tint: [k[0], k[1], k[2], 1.0] });
            }
        } }
        self.bush_cache.retain(|&(tx, tz), _| (tx - bx).abs() <= DRAW_TILES + 2 && (tz - bz).abs() <= DRAW_TILES + 2);
        let mut mnear = [0u32; MODEL_SLOTS];
        for k in SLOT_TREE..SLOT_GRASS { mnear[k] = ml[k].len() as u32; let f = std::mem::take(&mut far[k]); ml[k].extend(f); }
        // grass tufts around the camera: recomputed only when the camera leaves its 8 m cell (or the quality changes)
        let grass_cap = if self.city_radius < 350.0 { 0 } else if self.city_radius < 420.0 { 1500 } else { 3000 };
        let cell = ((wv.cam_pos.x / 8.0).floor() as i32, (wv.cam_pos.z / 8.0).floor() as i32, grass_cap);
        if self.grass_cache.as_ref().map_or(true, |(c, _)| *c != cell) {
            let centre = v3((cell.0 as f32 + 0.5) * 8.0, 0.0, (cell.1 as f32 + 0.5) * 8.0);
            self.grass_cache = Some((cell, scatter::grass_spots(wv.terrain, centre, 48.0, grass_cap)));
        }
        if let Some((_, spots)) = &self.grass_cache {
            for (p, sc) in spots {
                if (p.x - wv.cam_pos.x).hypot(p.z - wv.cam_pos.z) > 42.0 { continue; }
                ml[SLOT_GRASS].push(MInst { m: mmat_s(Q::axis_angle(v3(0., 1., 0.), yaw_of(*p)), *p, *sc), tint: [1.0; 4] });
            }
        }
        let mut mall: Vec<MInst> = Vec::new();
        let mut mranges = [(0u32, 0u32); MODEL_SLOTS];
        for k in 0..MODEL_SLOTS {
            let take = ml[k].len().min(MAX_MODEL_INST - mall.len());
            mranges[k] = (mall.len() as u32, (mall.len() + take) as u32);
            mall.extend_from_slice(&ml[k][..take]);
        }
        if !mall.is_empty() { self.queue.write_buffer(&self.mbuf, 0, bytemuck::cast_slice(&mall)); }
        let mut all: Vec<Inst> = Vec::new();
        let mut ranges = [(0u32, 0u32); 4];
        for k in 0..4 {
            let room = (MAX_INST - CITY_BUDGET) - all.len();
            let take = lists[k].len().min(room);
            ranges[k] = (all.len() as u32, (all.len() + take) as u32);
            all.extend_from_slice(&lists[k][..take]);
        }
        let cp = citygen::city_parts(wv.cities, wv.cam_pos, self.city_radius, CITY_BUDGET, 150.0);
        let city_start = all.len() as u32;
        for p in &cp.parts { all.push(if p.facade { inst_facade(p.rot, p.pos, p.scale, p.color) } else { inst(p.rot, p.pos, p.scale, p.color) }); }
        let (city_end, city_near_end) = (all.len() as u32, city_start + cp.near as u32);
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
            rp.set_pipeline(&self.pl_model_sh);
            rp.set_vertex_buffer(1, self.mbuf.slice(..));
            for k in 0..MODEL_SLOTS {
                let (a, mut b) = mranges[k];
                if k >= SLOT_TREE { b = if k == SLOT_GRASS { a } else { a + mnear[k].min(b - a) }; }
                if b == a { continue; }
                let m = &self.models[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
            rp.set_pipeline(&self.pl_inst_sh);
            rp.set_vertex_buffer(1, self.ibuf.slice(..));
            for k in 0..4 {
                let (a, b) = ranges[k]; if b == a { continue; }
                let m = &self.meshes[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
            if city_near_end > city_start {
                let m = &self.meshes[0];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, city_start..city_near_end);
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
                if let Some(w) = &m.wb { rp.set_vertex_buffer(1, w.slice(..)); }
                rp.draw_indexed(0..m.n, 0, 0..1);
            }
            rp.set_pipeline(&self.pl_model);
            rp.set_vertex_buffer(1, self.mbuf.slice(..));
            for k in 0..MODEL_SLOTS {
                let (a, b) = mranges[k]; if b == a { continue; }
                let m = &self.models[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
            rp.set_pipeline(&self.pl_inst);
            rp.set_vertex_buffer(1, self.ibuf.slice(..));
            for k in 0..4 {
                let (a, b) = ranges[k]; if b == a { continue; }
                let m = &self.meshes[k];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
            }
            if city_end > city_start {
                let m = &self.meshes[0];
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, city_start..city_end);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }
}
