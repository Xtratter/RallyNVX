//! wgpu side of the raster renderer.
use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
#[cfg(target_os = "android")]
use ndk::native_window::NativeWindow;
#[cfg(target_os = "android")]
use raw_window_handle::{AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle};

use super::{livery, airflow_gpu, blur_gpu, dust_gpu, sign_models, citygen, dustgen, heightatlas::{self, HeightAtlas}, roadgen, watergen, meshes, nature_models, texgen, vehicle_models as vm};
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
    cam: [f32; 4], sun: [f32; 4], fog: [f32; 4], sky_top: [f32; 4], sky_hor: [f32; 4], sun_col: [f32; 4],
    mode: [f32; 4], cam_tile: [f32; 4], moon: [f32; 4], disc: [f32; 4], hl_pos: [f32; 4], hl_dir: [f32; 4], tl_pos: [f32; 4], tl_col: [f32; 4], wind: [f32; 4],
    /// The car's lamps: headlights on, braking, reversing (the car model's lamp material shows them), 0.
    lamps: [f32; 4],
    /// The car's right-hand axis (rolls with the body): the beam's and the lamps' sideways direction.
    rt: [f32; 4],
    /// The four hub offsets of the car (metres, see `vehicle_models::susp_offsets`): the suspension parts of the car's mesh follow them.
    susp: [f32; 4],
    /// The weather: rain, wet ground, cloud cover, the visibility of the sun and moon discs.
    wx: [f32; 4],
    /// The car's position (the rain does not fall inside it) and the weather's clock.
    cr: [f32; 4],
}
const _: () = assert!(std::mem::size_of::<Uni>() == 512);

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

/// A street strip: even asphalt with lane marks drawn by the shader (colour alpha 4).
fn inst_street(rot: Q, pos: V3, s: V3, color: [f32; 3]) -> Inst { let mut i = inst(rot, pos, s, color); i.color[3] = 4.0; i }

/// The number of rain instances drawn (the shader keeps the share `rain` of them alive).
const RAIN_DROPS: u32 = 6000;

struct MeshBuf { vb: wgpu::Buffer, ib: wgpu::Buffer, n: u32, wb: Option<wgpu::Buffer> }

/// A vehicle model mesh in the model pipeline's vertex format (28 bytes).
struct ModelBuf { vb: wgpu::Buffer, ib: wgpu::Buffer, n: u32 }
/// A mesh for an unused model slot: one degenerate triangle (a buffer is never empty; nothing is ever drawn with it).
fn placeholder_mesh() -> vm::Mesh3 {
    let v = vm::Vert { pos: [0.0; 3], nrm: [0.0, 1.0, 0.0], col: [0; 4], uv: vm::NO_UV };
    vm::Mesh3 { verts: vec![v; 3], idx: vec![0, 1, 2] }
}

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
// slots: car body, wheel, 4 trees, 2 bushes, 3 rocks, grass tuft, the signs, the car's disc and caliper
const SLOT_TREE: usize = 2;
const SLOT_BUSH: usize = 6;
const SLOT_ROCK: usize = 8;
const SLOT_GRASS: usize = 11;
const SLOT_SIGN: usize = 12;   // one slot per `SignKind`
const SLOT_DISC: usize = SLOT_SIGN + 17;      // the car's brake discs (spin with the wheel)
const SLOT_CALIPER: usize = SLOT_SIGN + 18;   // the car's brake calipers (steer and follow the suspension, no spin)
const SLOT_INTERIOR: usize = SLOT_SIGN + 19;  // the car's interior and driver (opaque, drawn near the car only)
const SLOT_GLASS: usize = SLOT_SIGN + 20;     // the car's transparent glass (its own blended pass)
const SLOT_STEERING: usize = SLOT_SIGN + 21;  // the steering wheel (turns about its axis), drawn with the interior
const SLOT_DRIVER: usize = SLOT_SIGN + 22;    // the driver, drawn with the interior unless the camera is inside the car
const SLOT_WIPER: usize = SLOT_SIGN + 23;    // the two windscreen wipers (+0 the driver's side, +1 the other), sweeping in the rain
const SLOT_GLOVES: usize = SLOT_SIGN + 25;   // the driver's gloves (turn with the steering wheel), drawn with the interior in every view
const MODEL_SLOTS: usize = SLOT_SIGN + 26;
const MAX_MODEL_INST: usize = 6144;
const SIGN_SCALE: f32 = 2.2;   // the signs are drawn bigger than real ones so they read from a moving car
const NEAR_SHADOW: f32 = 90.0;   // plants within this distance of the car cast shadows
const MODEL_ATTRS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 8 => Unorm16x2];
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

/// Everything the car meshes depend on; the renderer rebuilds them when it changes (another car, size or paint).
fn car_key(s: &crate::game::vehicle::VehicleSpec) -> [u32; 13] {
    let b = |f: f32| f.to_bits();
    [s.style as u32, b(s.size.x), b(s.size.y), b(s.size.z), b(s.radius), b(s.look.wheel_width), b(s.look.body[0]), b(s.look.body[1]), b(s.look.body[2]), b(s.look.accent[0]), b(s.look.accent[1]), b(s.look.accent[2]), s.wheel_set as u32]
}

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
    pl_model_back: wgpu::RenderPipeline,   // the car's body seen from inside: back faces culled (the inside of the shell is not drawn)
    pl_model_sh: wgpu::RenderPipeline,
    models: Vec<ModelBuf>,
    mbuf: wgpu::Buffer,
    pl_water: wgpu::RenderPipeline,
    pl_road: wgpu::RenderPipeline,
    pl_glass: wgpu::RenderPipeline,
    pl_rain: wgpu::RenderPipeline, pl_dust: wgpu::RenderPipeline, pl_dust_pt: wgpu::RenderPipeline, pl_dust_specks: wgpu::RenderPipeline,
    /// The airflow debug view: made on first use (`airflow_tried`), so nothing is allocated while it is off.
    airflow: Option<airflow_gpu::AirflowGpu>, airflow_tried: bool, adapter: wgpu::Adapter,
    /// The dust on the GPU, made on first use.
    prof: Option<Prof>,
    /// The motion blur post pass (made when the setting is on and the surface can be copied), last frame's view-projection and camera position.
    blur: Option<blur_gpu::MotionBlur>, prev_vp: Option<[f32; 16]>, prev_cam: V3,
    dust_gpu: Option<dust_gpu::DustGpu>, dust_render: Option<dust_gpu::DustRender>, bgl_main_keep: wgpu::BindGroupLayout, module_keep: wgpu::ShaderModule,
    dust_buf: wgpu::Buffer,
    cache_seed: u64,
    car_key: [u32; 13],
    car_wipers: [Option<(V3, V3)>; 2],   // the wipers' spindles and sweep axes, when the car has them
    car_gloves: bool,   // the driver's gloves (they follow the steering wheel)
    car_steering: Option<(V3, V3)>,   // the steering wheel's pivot and axis, when the car has one
    car_has_driver: bool,
    car_parts: (bool, bool, bool, bool),   // whether the car model has brake discs / calipers / an interior / glass
    /// The signed distance field of the car's body mesh (for the dust's exact-shape option), rebuilt with the model; `sdf_serial` tells the dust a new one is there.
    car_sdf: Option<crate::game::sdf::Sdf>, sdf_serial: u32,
    /// The settled dust layer the dust compute writes (R = amount) and the terrain and road shaders tint the ground by.
    dep_tex: wgpu::Texture,
    atlas: HeightAtlas,
    atlas_tex: wgpu::Texture,
    hybrid: bool,
    water: HashMap<(i32, i32), Option<MeshBuf>>,
    roads: HashMap<(i32, i32), Option<MeshBuf>>,
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
            label: Some("game_device"), required_features: adapter.features() & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS), required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        }, None)).expect("request_device");
        let prof = if device.features().contains(wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) { Some(Prof::new(&device, queue.get_timestamp_period())) } else { None };
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) { wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC } else { wgpu::TextureUsages::RENDER_ATTACHMENT }, format, width: w.max(1), height: h.max(1),
            present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0], view_formats: vec![],
        };
        surface.configure(&device, &config);

        let depth = depth_tex(&device, config.width, config.height, true);
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
        // height atlas for the hybrid terrain ray shadows: 512x512 R16Float, emptied (far below the terrain) until tiles arrive
        let atlas_side = (heightatlas::ATLAS_TILES as u32) * heightatlas::TEXELS as u32;
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("height_atlas"), size: wgpu::Extent3d { width: atlas_side, height: atlas_side, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        let empty = vec![heightatlas::f32_to_f16(heightatlas::EMPTY); (atlas_side * atlas_side) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &atlas_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&empty), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(atlas_side * 2), rows_per_image: Some(atlas_side) },
            wgpu::Extent3d { width: atlas_side, height: atlas_side, depth_or_array_layers: 1 });
        let atlas_view = atlas_tex.create_view(&Default::default());
        let atlas_smp = device.create_sampler(&wgpu::SamplerDescriptor { label: Some("atlas_smp"),
            address_mode_u: wgpu::AddressMode::Repeat, address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
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
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering) },
            wgpu::BindGroupLayoutEntry { binding: 7, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 8, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 9, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering) },
            wgpu::BindGroupLayoutEntry { binding: 10, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
            wgpu::BindGroupLayoutEntry { binding: 11, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering) },
            wgpu::BindGroupLayoutEntry { binding: 12, visibility: wgpu::ShaderStages::FRAGMENT, count: None,
                ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false } },
        ] });
        // the rally car's atlases (JPEGs in the assets, mips built here): the livery and the interior; a missing or broken file leaves a white texel (the car keeps its vertex colours)
        let atlas_texture = |label: &str, jpeg: &[u8], format: wgpu::TextureFormat| -> (wgpu::TextureView, wgpu::Sampler) {
            let (lw, lh, lmips) = match livery::decode(jpeg) {
                Some((w, h, px)) => (w, h, livery::mips(&px, w, h)),
                None => { log::warn!("{label} atlas unusable; the car keeps its vertex colours"); (1, 1, vec![(1, 1, vec![128u8, 128, 255, 255])]) }
            };
            let tex = device.create_texture(&wgpu::TextureDescriptor { label: Some(label), size: wgpu::Extent3d { width: lw, height: lh, depth_or_array_layers: 1 }, mip_level_count: lmips.len() as u32, sample_count: 1,
                dimension: wgpu::TextureDimension::D2, format, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
            for (level, (w, h, px)) in lmips.iter().enumerate() {
                queue.write_texture(wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    px, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(*h) }, wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 });
            }
            let smp = device.create_sampler(&wgpu::SamplerDescriptor { label: Some(label), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, anisotropy_clamp: 4, ..Default::default() });
            (tex.create_view(&Default::default()), smp)
        };
        let (livery_view, livery_smp) = atlas_texture("livery", livery::LIVERY, wgpu::TextureFormat::Rgba8UnormSrgb);
        let (interior_view, interior_smp) = atlas_texture("interior", livery::INTERIOR, wgpu::TextureFormat::Rgba8UnormSrgb);
        let (normal_view, _) = atlas_texture("normal", livery::NORMAL, wgpu::TextureFormat::Rgba8Unorm);
        let dep_tex = device.create_texture(&wgpu::TextureDescriptor { label: Some("dust_layer"), size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
        let dep_view = dep_tex.create_view(&Default::default());
        let bgl_shadow = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("raster_shadow"), entries: &[ubo(wgpu::ShaderStages::VERTEX)] });
        let bg_main = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("raster_main"), layout: &bgl_main, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow.1) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&smp) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&mat_view) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&mat_smp) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&atlas_view) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(&atlas_smp) },
            wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&dep_view) },
            wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&livery_view) },
            wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::Sampler(&livery_smp) },
            wgpu::BindGroupEntry { binding: 10, resource: wgpu::BindingResource::TextureView(&interior_view) },
            wgpu::BindGroupEntry { binding: 11, resource: wgpu::BindingResource::Sampler(&interior_smp) },
            wgpu::BindGroupEntry { binding: 12, resource: wgpu::BindingResource::TextureView(&normal_view) },
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
        let target_blend = [Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })];
        let pl_water = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_water"), compilation_options: Default::default(), buffers: &[mesh_layout.clone()] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_water"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let road_bias = wgpu::DepthBiasState { constant: -4, slope_scale: -2.0, clamp: 0.0 };
        let pl_road = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("road"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_road"), compilation_options: Default::default(), buffers: &[mesh_layout.clone()] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_road"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, road_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        const DUST_ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let dust_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<dustgen::DustInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &DUST_ATTRS };
        let pl_dust = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dust"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_dust"), compilation_options: Default::default(), buffers: &[dust_layout] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_dust"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let pl_dust_pt = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dust_pt"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_dust"), compilation_options: Default::default(), buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<dustgen::DustInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &DUST_ATTRS }] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_dust_pt"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let pl_dust_specks = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("dust_specks"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_dust_specks"), compilation_options: Default::default(), buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<dustgen::DustInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &DUST_ATTRS }] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_dust_speck"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let dust_buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("dust_inst"), size: (dustgen::MAX_INST * std::mem::size_of::<dustgen::DustInst>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let pl_terrain_sh = mk("terrain_sh", &lay_shadow, "vs_terrain_shadow", None, &[mesh_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_inst = mk("inst", &lay_main, "vs_inst", Some("fs_inst"), &[mesh_layout.clone(), inst_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_inst_sh = mk("inst_sh", &lay_shadow, "vs_inst_shadow", None, &[mesh_layout, inst_layout], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let model_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<vm::Vert>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &MODEL_ATTRS };
        let minst_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<MInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &MINST_ATTRS };
        let pl_model = mk("model", &lay_main, "vs_model", Some("fs_model"), &[model_layout.clone(), minst_layout.clone()], depth_state(true, wgpu::CompareFunction::Less, no_bias));
        let pl_model_back = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model_back"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_model"), compilation_options: Default::default(), buffers: &[model_layout.clone(), minst_layout.clone()] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_model"), compilation_options: Default::default(), targets: &target }),
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() }, depth_stencil: depth_state(true, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let pl_rain = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rain"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_rain"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_rain"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let pl_glass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass"), layout: Some(&lay_main),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_model"), compilation_options: Default::default(), buffers: &[model_layout.clone(), minst_layout.clone()] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_glass"), compilation_options: Default::default(), targets: &target_blend }),
            primitive: prim, depth_stencil: depth_state(false, wgpu::CompareFunction::Less, no_bias), multisample: wgpu::MultisampleState::default(), multiview: None, cache: None,
        });
        let pl_model_sh = mk("model_sh", &lay_shadow, "vs_model_shadow", None, &[model_layout, minst_layout], depth_state(true, wgpu::CompareFunction::Less, sh_bias));
        let pl_sky = mk("sky", &lay_main, "vs_sky", Some("fs_sky"), &[], depth_state(false, wgpu::CompareFunction::Always, no_bias));

        let car = vm::build_car(&crate::game::vehicle::VehicleSpec::offroader());
        let nat = nature_models::build_all();
        let signs_meshes = sign_models::build_all();
        let models: Vec<ModelBuf> = [&car.body, &car.wheel].into_iter()
            .chain(nat.trees.iter()).chain(nat.bushes.iter()).chain(nat.rocks.iter()).chain([&nat.grass]).chain(signs_meshes.iter()).chain([&car.disc.clone().unwrap_or_else(placeholder_mesh), &car.caliper.clone().unwrap_or_else(placeholder_mesh), &car.interior.clone().unwrap_or_else(placeholder_mesh), &car.glass.clone().unwrap_or_else(placeholder_mesh), &car.steering.as_ref().map(|s| s.mesh.clone()).unwrap_or_else(placeholder_mesh), &car.driver.clone().unwrap_or_else(placeholder_mesh), &car.wipers[0].as_ref().map(|s| s.mesh.clone()).unwrap_or_else(placeholder_mesh), &car.wipers[1].as_ref().map(|s| s.mesh.clone()).unwrap_or_else(placeholder_mesh), &car.gloves.clone().unwrap_or_else(placeholder_mesh)]).map(|m| model_buf(&device, m)).collect();
        assert_eq!(models.len(), MODEL_SLOTS);
        let mbuf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("model_inst"), size: (MAX_MODEL_INST * std::mem::size_of::<MInst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let meshes = [mesh_buf(&device, &meshes::cube()), mesh_buf(&device, &meshes::cylinder()), mesh_buf(&device, &meshes::cone()), mesh_buf(&device, &meshes::sphere())];
        Raster { city_radius: 450.0, surface, device, queue, config, depth, shadow, ubuf, ibuf, bg_main, bg_shadow,
            pl_terrain, pl_terrain_sh, pl_inst, pl_inst_sh, pl_sky, pl_model, pl_model_back, pl_model_sh, models, mbuf, pl_water, pl_road, pl_glass, pl_rain, pl_dust, pl_dust_pt, pl_dust_specks, airflow: None, airflow_tried: false, adapter: adapter.clone(), prof, blur: None, prev_vp: None, prev_cam: V3::ZERO, dust_gpu: None, dust_render: None, bgl_main_keep: bgl_main.clone(), module_keep: module.clone(), dust_buf, cache_seed: u64::MAX, car_key: car_key(&crate::game::vehicle::VehicleSpec::offroader()), car_gloves: car.gloves.is_some(), car_wipers: std::array::from_fn(|k| car.wipers[k].as_ref().map(|s| (s.pivot, s.axis))), car_steering: car.steering.as_ref().map(|s| (s.pivot, s.axis)), car_has_driver: car.driver.is_some(), car_parts: (car.disc.is_some(), car.caliper.is_some(), car.interior.is_some(), car.glass.is_some()), car_sdf: None, sdf_serial: 0, dep_tex, atlas: HeightAtlas::new(), atlas_tex, hybrid: false, water: HashMap::new(), roads: HashMap::new(), bush_cache: HashMap::new(), grass_cache: None, wheel_spin: 0.0, last_time: 0.0, meshes, tiles: HashMap::new(), _keep: keep }
    }

    pub fn device(&self) -> &wgpu::Device { &self.device }
    pub fn queue(&self) -> &wgpu::Queue { &self.queue }
    pub fn format(&self) -> wgpu::TextureFormat { self.config.format }

    /// How far from the camera city buildings and tiles are drawn (metres).
    pub fn set_city_radius(&mut self, r: f32) { self.city_radius = r.clamp(100.0, 2000.0); }

    /// Hybrid mode: PCSS soft shadows, terrain ray shadows and forward ambient occlusion (no post pass).
    pub fn set_hybrid(&mut self, on: bool) { self.hybrid = on; }
    pub fn is_hybrid(&self) -> bool { self.hybrid }

    fn upload_slot(&self, w: &heightatlas::SlotWrite) {
        let t = heightatlas::TEXELS as u32;
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.atlas_tex, mip_level: 0, origin: wgpu::Origin3d { x: w.slot.0 * t, y: w.slot.1 * t, z: 0 }, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&w.data), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(t * 2), rows_per_image: Some(t) },
            wgpu::Extent3d { width: t, height: t, depth_or_array_layers: 1 });
    }

    /// Rebuilds the car body and wheel meshes when the car in the world is another one (garage, a bought car).
    fn ensure_car_model(&mut self, spec: &crate::game::vehicle::VehicleSpec) {
        let key = car_key(spec);
        if key == self.car_key && self.car_sdf.is_some() { return; }
        let car = vm::build_car(spec);
        self.models[0] = model_buf(&self.device, &car.body);
        self.models[1] = model_buf(&self.device, car.wheel_for(spec.wheel_set));
        self.models[SLOT_DISC] = model_buf(&self.device, &car.disc.clone().unwrap_or_else(placeholder_mesh));
        self.models[SLOT_CALIPER] = model_buf(&self.device, &car.caliper.clone().unwrap_or_else(placeholder_mesh));
        self.models[SLOT_INTERIOR] = model_buf(&self.device, &car.interior.clone().unwrap_or_else(placeholder_mesh));
        self.models[SLOT_GLASS] = model_buf(&self.device, &car.glass.clone().unwrap_or_else(placeholder_mesh));
        self.models[SLOT_STEERING] = model_buf(&self.device, &car.steering.as_ref().map(|s| s.mesh.clone()).unwrap_or_else(placeholder_mesh));
        self.models[SLOT_DRIVER] = model_buf(&self.device, &car.driver.clone().unwrap_or_else(placeholder_mesh));
        for k in 0..2 { self.models[SLOT_WIPER + k] = model_buf(&self.device, &car.wipers[k].as_ref().map(|s| s.mesh.clone()).unwrap_or_else(placeholder_mesh)); }
        self.models[SLOT_GLOVES] = model_buf(&self.device, &car.gloves.clone().unwrap_or_else(placeholder_mesh));
        self.car_gloves = car.gloves.is_some();
        self.car_wipers = std::array::from_fn(|k| car.wipers[k].as_ref().map(|s| (s.pivot, s.axis)));
        self.car_steering = car.steering.as_ref().map(|s| (s.pivot, s.axis));
        self.car_has_driver = car.driver.is_some();
        self.car_parts = (car.disc.is_some(), car.caliper.is_some(), car.interior.is_some(), car.glass.is_some());
        self.car_key = key;
        // the exact shape for the dust: the body mesh as a signed distance field (0.1 m voxels)
        let (verts, idx) = vm::sdf_input(&car);
        self.car_sdf = Some(crate::game::sdf::build(&verts, &idx, 0.1, 0.5));
        self.sdf_serial += 1;
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if w == self.config.width && h == self.config.height { return; }
        self.config.width = w; self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_tex(&self.device, w, h, true);
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
        self.ensure_car_model(&wv.vehicle.spec);

        // camera and light matrices
        let aspect = self.config.width as f32 / self.config.height as f32;
        let vp = mat_mul(perspective(wv.cam_fov.clamp(0.4, 1.6), aspect, vm::near_for((wv.cam_pos - wv.vehicle.pos).len()), 2500.0), look_at(wv.cam_pos, wv.cam_target, v3(0., 1., 0.)));
        let centre = wv.vehicle.pos;
        let up = if wv.sun.y.abs() > 0.99 { v3(0., 0., 1.) } else { v3(0., 1., 0.) };
        let light_vp = mat_mul(ortho(-70., 70., -70., 70., 1., 300.), look_at(centre + wv.sun * 150.0, centre, up));
        let hc = (wv.lamps.head[0] + wv.lamps.head[1]) * 0.5;
        let tc = (wv.lamps.tail[0] + wv.lamps.tail[1]) * 0.5 - v3(0., 0.3, 0.);
        let tail_glow = if wv.lights.reverse { [0.8, 0.8, 0.78, 7.0] } else if wv.lights.brake { [0.5, 0.04, 0.02, 7.0] } else if wv.lights.head { [0.12, 0.01, 0.005, 4.0] } else { [0.0; 4] };
        let uni = Uni { vp: flatten(vp), light_vp: flatten(light_vp), inv_vp: flatten(inverse(vp)),
            cam: [wv.cam_pos.x, wv.cam_pos.y, wv.cam_pos.z, 0.], sun: [wv.sun.x, wv.sun.y, wv.sun.z, 0.],
            fog: [crate::game::weather::fog_k(wv.weather), wv.time, 0., 0.], sky_top: [wv.daylight.sky_top[0], wv.daylight.sky_top[1], wv.daylight.sky_top[2], 0.],
            sky_hor: [wv.daylight.sky_hor[0], wv.daylight.sky_hor[1], wv.daylight.sky_hor[2], 0.],
            sun_col: [wv.daylight.sun_col[0], wv.daylight.sun_col[1], wv.daylight.sun_col[2], 0.],
            mode: { let b = crate::game::dust_ref::window_base([wv.vehicle.pos.x.floor() as i32, wv.vehicle.pos.z.floor() as i32]); [self.hybrid as u8 as f32, b[0] as f32, b[1] as f32, if wv.dust_gpu.as_ref().map_or(false, |f| f.opts & crate::game::dust::OPT_DEPOSIT != 0) { 1.0 } else { 0.0 }] }, cam_tile: [cx as f32, cz as f32, 0., 0.],
            moon: [wv.daylight.moon.x, wv.daylight.moon.y, wv.daylight.moon.z, 0.],
            disc: [wv.daylight.sun_disc.x, wv.daylight.sun_disc.y, wv.daylight.sun_disc.z, wv.daylight.night],
            hl_pos: [hc.x, hc.y, hc.z, if wv.lights.head { 1.0 } else { 0.0 }],
            hl_dir: [wv.lamps.head_dir.x, wv.lamps.head_dir.y, wv.lamps.head_dir.z, 0.927],
            tl_pos: [tc.x, tc.y, tc.z, (wv.lamps.tail[0] - wv.lamps.tail[1]).len() * 0.5],
            tl_col: tail_glow,
            wind: [wv.wind.x, wv.wind.z, wv.cloud_off.0, wv.cloud_off.1],
            lamps: [wv.lights.head as u8 as f32, wv.lights.brake as u8 as f32, wv.lights.reverse as u8 as f32, (wv.lamps.head[0] - wv.lamps.head[1]).len() * 0.5], rt: [wv.lamps.right.x, wv.lamps.right.y, wv.lamps.right.z, vm::dash_rpm(wv.vehicle.speed(), wv.vehicle.spec.max_speed)], susp: vm::susp_offsets(wv.vehicle),
            wx: [wv.weather.rain, wv.weather.wet, wv.weather.cloud, crate::game::weather::disc_visibility(wv.weather)], cr: [wv.vehicle.pos.x, wv.vehicle.pos.y, wv.vehicle.pos.z, wv.time] };
        self.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&uni));

        // a new level invalidates every cache keyed by tile coordinates
        if self.cache_seed != wv.terrain.seed {
            self.cache_seed = wv.terrain.seed;
            self.tiles.clear(); self.water.clear(); self.roads.clear(); self.bush_cache.clear(); self.grass_cache = None;
            for w in self.atlas.clear() { self.upload_slot(&w); }
        }
        // terrain tiles: nearest first, a few per frame; the old LOD stays drawn until its replacement exists
        let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
        let (build, visible) = super::tile_plan(cx, cz, &have, DRAW_TILES);
        for &(tx, tz, lod) in build.iter().take(TILE_BUDGET) {
            let (mesh, w) = wv.terrain.tile_mesh_with_weights(tx, tz, lod);
            let slot_write = self.atlas.put(tx, tz, &heightatlas::heights_from_mesh(&mesh.verts, lod));
            self.upload_slot(&slot_write);
            let mut mb = mesh_buf(&self.device, &mesh);
            mb.wb = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("tile_w"), contents: bytemuck::cast_slice(&w), usage: wgpu::BufferUsages::VERTEX }));
            self.tiles.insert((tx, tz, lod), mb);
            if !self.roads.contains_key(&(tx, tz)) { let rm = roadgen::road_mesh(wv.terrain, tx, tz).map(|m| mesh_buf(&self.device, &m)); self.roads.insert((tx, tz), rm); }
            if !self.water.contains_key(&(tx, tz)) { let wm = watergen::water_mesh(wv.terrain, tx, tz).map(|m| mesh_buf(&self.device, &m)); self.water.insert((tx, tz), wm); }
        }
        let (_, visible) = if build.len() > TILE_BUDGET || !build.is_empty() {
            let have: HashSet<(i32, i32, u32)> = self.tiles.keys().copied().collect();
            super::tile_plan(cx, cz, &have, DRAW_TILES)
        } else { (build, visible) };
        let keep: HashSet<_> = visible.iter().copied().collect();
        self.tiles.retain(|k, _| keep.contains(k));
        let present: HashSet<(i32, i32)> = self.tiles.keys().map(|k| (k.0, k.1)).collect();
        for w in self.atlas.evict_missing(&present) { self.upload_slot(&w); }
        self.water.retain(|&(tx, tz), _| (tx - cx).abs() <= DRAW_TILES + 1 && (tz - cz).abs() <= DRAW_TILES + 1);
        self.roads.retain(|&(tx, tz), _| (tx - cx).abs() <= DRAW_TILES + 1 && (tz - cz).abs() <= DRAW_TILES + 1);

        // instances: cube, cylinder, cone, sphere
        let mut lists: [Vec<Inst>; 4] = Default::default();
        // vehicle models: per slot lists of (rotation, position)
        let mut ml: [Vec<MInst>; MODEL_SLOTS] = std::array::from_fn(|_| Vec::new());
        let dt = (wv.time - self.last_time).clamp(0.0, 0.1);
        self.last_time = wv.time;
        let veh = wv.vehicle;
        self.wheel_spin += veh.speed() / veh.spec.radius * dt;
        self.wheel_spin %= std::f32::consts::TAU;
        let (bq, bp) = vm::car_body_pose(veh);
        ml[0].push(MInst { m: mmat(bq, bp), tint: [1.0, 1.0, 1.0, 0.5] });   // alpha 0.5: the player's car (not lit by its own lamps)
        // the glass (always) and the interior (near the car only) share the body's pose
        if self.car_parts.3 { ml[SLOT_GLASS].push(MInst { m: mmat(bq, bp), tint: [1.0, 1.0, 1.0, 0.5] }); }
        // the wipers sit on the outside of the glass: drawn with it, parked unless it rains
        let wa = vm::wiper_angle(wv.time, wv.weather.rain);
        for k in 0..2 {
            if let Some((pivot, axis)) = self.car_wipers[k] {
                ml[SLOT_WIPER + k].push(MInst { m: mmat(bq.mul(Q::axis_angle(axis, wa)), bp + bq.rotate(pivot)), tint: [1.0, 1.0, 1.0, 0.5] });
            }
        }
        if self.car_parts.2 && vm::draw_interior((wv.cam_pos - veh.pos).len()) {
            ml[SLOT_INTERIOR].push(MInst { m: mmat(bq, bp), tint: [1.0, 1.0, 1.0, 0.5] });
            // the steering wheel turns about its axis with the steering; the driver is left out when the camera is inside the car
            if let Some((pivot, axis)) = self.car_steering {
                let q = bq.mul(Q::axis_angle(axis, vm::steering_wheel_angle(veh.steer)));
                ml[SLOT_STEERING].push(MInst { m: mmat(q, bp + bq.rotate(pivot)), tint: [1.0, 1.0, 1.0, 0.5] });
                if self.car_gloves { ml[SLOT_GLOVES].push(MInst { m: mmat(q, bp + bq.rotate(pivot)), tint: [1.0, 1.0, 1.0, 0.5] }); }
            }
            if self.car_has_driver && vm::draw_driver((wv.cam_pos - veh.pos).len()) { ml[SLOT_DRIVER].push(MInst { m: mmat(bq, bp), tint: [1.0, 1.0, 1.0, 0.5] }); }
        }
        for (w, p) in vm::car_wheel_poses(veh, self.wheel_spin).iter().enumerate() {
            ml[1].push(MInst { m: mmat(p.0, p.1), tint: [1.0, 1.0, 1.0, 0.5] });
            // the disc turns with the wheel; its tint alpha carries the brake heat (0.1 .. 0.5), see `fs_model`
            if self.car_parts.0 { ml[SLOT_DISC].push(MInst { m: mmat(p.0, p.1), tint: [1.0, 1.0, 1.0, 0.1 + 0.4 * veh.brake_heat[w].clamp(0.0, 1.0)] }); }
        }
        if self.car_parts.1 { for p in vm::car_caliper_poses(veh) { ml[SLOT_CALIPER].push(MInst { m: mmat(p.0, p.1), tint: [1.0, 1.0, 1.0, 0.5] }); } }
        {
            let (lights, lamps, rot) = (&wv.lights, &wv.lamps, wv.vehicle.rot);
            let mut glow = |p: V3, s: V3, col: [f32; 3]| lists[0].push(inst_glow(rot, p, s, col));
            let own_lamps = veh.spec.style == crate::game::vehicle::CarStyle::Rally;   // the rally car's mesh has real lamp shapes that show their own state
            if own_lamps {} else if lights.head { for p in lamps.head { glow(p, v3(0.24, 0.13, 0.08), [1.0, 0.97, 0.85]); } }
            let tail_col = if lights.brake { [1.0, 0.05, 0.03] } else if lights.head { [0.35, 0.02, 0.01] } else { [0.0; 3] };
            if own_lamps {} else if tail_col[0] > 0.0 { for p in lamps.tail { glow(p, v3(0.28, 0.1, 0.06), tail_col); } }
            if !own_lamps && lights.reverse { for p in lamps.rev { glow(p, v3(0.16, 0.1, 0.06), [1.0, 1.0, 0.95]); } }
        }
        if let Some(level) = wv.terrain.water_level() {
            if veh.speed().abs() > 2.0 {
                let tick = (wv.time * 16.0) as i32;
                for (wi, p) in vm::car_wheel_poses(veh, 0.0).iter().enumerate() {
                    if wv.terrain.height(p.1.x, p.1.z) >= level - 0.05 { continue; }
                    for n in 0..10i32 {
                        let h = |k: i32| crate::game::terrain::hash01(0x5B1A + wi as u64, tick.wrapping_mul(31).wrapping_add(n), k);
                        let (a, r, rise) = (h(1) * std::f32::consts::TAU, 0.3 + h(2) * 0.8, 0.1 + h(3) * 0.6);
                        let s = 0.04 + 0.06 * h(4);
                        lists[0].push(inst(Q::IDENT, v3(p.1.x + a.cos() * r, level + rise, p.1.z + a.sin() * r), v3(s, s, s), [0.75, 0.88, 0.95]));
                    }
                }
            }
        }
        for tr in wv.tracks.iter() {
            lists[0].push(inst(crate::game::tracks::track_rot(tr.dir, tr.normal), tr.pos + tr.normal * 0.03, v3(0.3, 0.02, 0.45), [0.10, 0.075, 0.05]));
        }
        // plants and rocks: generated models, the ones near the car first (they cast shadows)
        let vpos = wv.vehicle.pos;
        let draw_r = self.city_radius;
        let mut far: [Vec<MInst>; MODEL_SLOTS] = std::array::from_fn(|_| Vec::new());
        let mut put = |ml: &mut [Vec<MInst>; MODEL_SLOTS], slot: usize, p: V3, m: MInst| {
            let d = (p.x - vpos.x).hypot(p.z - vpos.z);
            if d < NEAR_SHADOW { ml[slot].push(m); } else if d < draw_r { far[slot].push(m); }
        };
        for p in &wv.props {
            let yaw = Q::axis_angle(v3(0., 1., 0.), yaw_of(p.pos));
            let k = nature_models::tint(p.pos); let tint = [k[0], k[1], k[2], 1.0];
            match p.kind {
                PropKind::Rock => put(&mut ml, SLOT_ROCK + nature_models::variant(2, p.pos), p.pos, MInst { m: mmat_s(yaw, p.pos + v3(0., 0.4 * p.radius, 0.), p.radius), tint: [tint[0], tint[1], tint[2], 3.0] }),
                PropKind::Tree => put(&mut ml, SLOT_TREE + nature_models::variant(0, p.pos), p.pos, MInst { m: mmat_s(yaw, p.pos, p.height), tint: [tint[0], tint[1], tint[2], 2.0] }),
            }
        }
        // signs along the stage: a board model per kind, drawn within the draw radius
        for s in wv.signs.iter() {
            let q = Q::axis_angle(v3(0., 1., 0.), s.yaw);
            put(&mut ml, SLOT_SIGN + s.kind.index(), s.pos, MInst { m: mmat_s(q, s.pos, SIGN_SCALE), tint: [1.0, 1.0, 1.0, 1.0] });
        }
        let (bx, bz) = (tile_of(wv.cam_pos.x), tile_of(wv.cam_pos.z));
        for tz in bz - DRAW_TILES..=bz + DRAW_TILES { for tx in bx - DRAW_TILES..=bx + DRAW_TILES {
            let bushes = self.bush_cache.entry((tx, tz)).or_insert_with(|| scatter::tile_bushes(wv.terrain, tx, tz));
            for b in bushes.iter() {
                let k = nature_models::tint(b.pos);
                put(&mut ml, SLOT_BUSH + nature_models::variant(1, b.pos), b.pos, MInst { m: mmat_s(Q::axis_angle(v3(0., 1., 0.), yaw_of(b.pos)), b.pos, b.scale), tint: [k[0], k[1], k[2], 2.0] });
            }
        } }
        self.bush_cache.retain(|&(tx, tz), _| (tx - bx).abs() <= DRAW_TILES + 2 && (tz - bz).abs() <= DRAW_TILES + 2);
        let mut mnear = [0u32; MODEL_SLOTS];
        for k in (SLOT_TREE..SLOT_GRASS).chain(SLOT_SIGN..MODEL_SLOTS) { mnear[k] = ml[k].len() as u32; let f = std::mem::take(&mut far[k]); ml[k].extend(f); }
        // grass tufts around the camera: recomputed only when the camera leaves its 8 m cell (or the quality changes)
        let grass_cap = if self.city_radius < 350.0 { 0 } else if self.city_radius < 420.0 { 1500 } else { 3000 };
        let high = wv.cam_pos.y - wv.terrain.height(wv.cam_pos.x, wv.cam_pos.z) > 45.0;   // no tufts seen from up there
        let grass_cap = if high { 0 } else { grass_cap };
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
        let dinst = dustgen::instances(&wv.dust, wv.cam_pos);
        if !dinst.is_empty() { self.queue.write_buffer(&self.dust_buf, 0, bytemuck::cast_slice(&dinst)); }
        if wv.airflow.is_some() && self.airflow.is_none() && !self.airflow_tried {
            self.airflow_tried = true;
            self.airflow = airflow_gpu::AirflowGpu::new(&self.adapter, &self.device, self.config.format);
        }
        // the headlights' beam in the air is drawn by the low-resolution dust pass, so that pass runs with the lights on even without GPU dust
        let beam_on = wv.lights.head;
        if wv.dust_gpu.is_some() && self.dust_gpu.is_none() {
            self.dust_gpu = Some(dust_gpu::DustGpu::new(&self.device, &self.queue, &self.dep_tex));
        }
        if (wv.dust_gpu.is_some() || beam_on) && self.dust_render.is_none() {
            self.dust_render = Some(dust_gpu::DustRender::new(&self.device, &self.bgl_main_keep, &self.module_keep, self.config.format));
        }
        let dust_low = (wv.dust_gpu.is_some() || beam_on) && self.dust_render.is_some();
        let blur_params = blur_gpu::params(BLUR_LEVEL.load(std::sync::atomic::Ordering::Relaxed)).filter(|_| self.config.usage.contains(wgpu::TextureUsages::COPY_SRC));
        if blur_params.is_some() && self.blur.is_none() { self.blur = Some(blur_gpu::MotionBlur::new(&self.device, self.config.format)); }
        let blur_on = blur_params.is_some() && self.blur.is_some();
        let dust_n = dinst.len() as u32;
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
        for p in &cp.parts { all.push(if p.facade { inst_facade(p.rot, p.pos, p.scale, p.color) } else if p.glow { inst_glow(p.rot, p.pos, p.scale, p.color) } else if p.street { inst_street(p.rot, p.pos, p.scale, p.color) } else { inst(p.rot, p.pos, p.scale, p.color) }); }
        let (city_end, city_near_end) = (all.len() as u32, city_start + cp.near as u32);
        if !all.is_empty() { self.queue.write_buffer(&self.ibuf, 0, bytemuck::cast_slice(&all)); }

        let near = |tx: i32, tz: i32| {
            let c = v3((tx as f32 + 0.5) * TILE_SIZE, centre.y, (tz as f32 + 0.5) * TILE_SIZE);
            let d = c - centre; d.x.hypot(d.z) < 180.0
        };
        let mut enc = self.device.create_command_encoder(&Default::default());
        if let Some(p) = &self.prof { p.mark(&mut enc, 0); }
                if let (Some(af), Some(fi)) = (self.airflow.as_mut(), wv.airflow.as_ref()) {
            af.update(&self.device, &self.queue, &mut enc, fi.inflow, &fi.boxes, wv.time);
        }
        if let Some(p) = &self.prof { p.mark(&mut enc, 1); }
        if let (Some(dg), Some(fr)) = (self.dust_gpu.as_mut(), wv.dust_gpu.as_ref()) {
            let grid = self.airflow.as_ref().filter(|_| wv.airflow.is_some()).map(|a| { let (v, s) = a.fields(); (v, s, a.box_size(), a.dens_buffer()) });
            if dg.sdf_serial() != self.sdf_serial { if let Some(s) = self.car_sdf.as_ref() { dg.set_sdf(&self.device, &self.queue, s, self.sdf_serial); } }
            dg.update(&self.device, &self.queue, &mut enc, wv.terrain, fr, &wv.props, grid);
        }
        if let Some(p) = &self.prof { p.mark(&mut enc, 2); }
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
                if k == SLOT_GLASS || k == SLOT_INTERIOR || k == SLOT_STEERING || k == SLOT_DRIVER || k == SLOT_GLOVES { continue; }   // the glass and the interior cast no shadows
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
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: wv.daylight.sky_hor[0] as f64 * 0.7, g: wv.daylight.sky_hor[1] as f64 * 0.7, b: wv.daylight.sky_hor[2] as f64 * 0.7, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &self.depth.1,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: if dust_low || blur_on { wgpu::StoreOp::Store } else { wgpu::StoreOp::Discard } }), stencil_ops: None }),
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
            let inside = (wv.cam_pos - wv.vehicle.pos).len() < 1.5;   // the camera is in the car: its body is drawn with the back faces culled
            for k in 0..MODEL_SLOTS {
                if k == SLOT_GLASS { continue; }   // drawn blended, after everything opaque
                let (a, b) = mranges[k]; if b == a { continue; }
                let m = &self.models[k];
                if k == 0 && inside { rp.set_pipeline(&self.pl_model_back); rp.set_vertex_buffer(1, self.mbuf.slice(..)); }
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, a..b);
                if k == 0 && inside { rp.set_pipeline(&self.pl_model); rp.set_vertex_buffer(1, self.mbuf.slice(..)); }
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
            rp.set_pipeline(&self.pl_road);
            for m in self.roads.values().flatten() {
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, 0..1);
            }
            rp.set_pipeline(&self.pl_water);
            for m in self.water.values().flatten() {
                rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..m.n, 0, 0..1);
            }
            {   // the car's glass: blended over the opaque scene, no depth write (see `fs_glass`)
                let (a, b) = mranges[SLOT_GLASS];
                if b > a {
                    let m = &self.models[SLOT_GLASS];
                    rp.set_pipeline(&self.pl_glass);
                    rp.set_vertex_buffer(1, self.mbuf.slice(..));
                    rp.set_vertex_buffer(0, m.vb.slice(..)); rp.set_index_buffer(m.ib.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..m.n, 0, a..b);
                }
            }
            if wv.weather.rain > 0.02 {   // the rain streaks: stateless instanced quads round the camera (`vs_rain`)
                rp.set_pipeline(&self.pl_rain);
                rp.draw(0..6, 0..RAIN_DROPS);
            }
            if let (Some(dg), true) = (self.dust_gpu.as_ref(), wv.dust_gpu.is_some()) {
                // the specks of grit at full resolution with the scene's depth test; the cloud itself follows after the pass at low resolution
                rp.set_pipeline(&self.pl_dust_specks);
                rp.set_vertex_buffer(0, dg.render_buffer().slice(..));
                rp.draw(0..6, 0..dg.count());
            } else if dust_n > 0 {
                rp.set_pipeline(&self.pl_dust);
                rp.set_vertex_buffer(0, self.dust_buf.slice(..));
                rp.draw(0..6, 0..dust_n);
            }
        }
        if let Some(p) = &self.prof { p.mark(&mut enc, 3); }
        if let (true, Some(dr)) = (dust_low, self.dust_render.as_mut()) {
            let (buf, count) = match (self.dust_gpu.as_ref(), wv.dust_gpu.is_some()) { (Some(dg), true) => (Some(dg.render_buffer()), dg.count()), _ => (None, 0) };
            dr.draw(&self.device, &mut enc, &view, &self.depth.1, &self.bg_main, buf, count, beam_on, (self.config.width, self.config.height));
        }
        if let Some(p) = &self.prof { p.mark(&mut enc, 4); }
        if let (Some(af), Some(_), true) = (self.airflow.as_ref(), wv.airflow.as_ref(), crate::game::airflow::debug_on()) {
            af.draw(&self.device, &mut enc, &view, (self.config.width, self.config.height));
        }
        // the speed effect: camera motion blur over the finished frame, the car masked so it stays sharp
        if let (true, Some(bp), Some(mb)) = (blur_on, blur_params, self.blur.as_mut()) {
            let cam_jump = self.prev_vp.is_none() || (wv.cam_pos - self.prev_cam).len() > 30.0;
            if !cam_jump {
                let cp = wv.vehicle.pos + v3(0.0, 0.8, 0.0);
                let clip = [0, 1, 2, 3].map(|r| vp[0][r] * cp.x + vp[1][r] * cp.y + vp[2][r] * cp.z + vp[3][r]);
                let inside = (wv.cam_pos - wv.vehicle.pos).len() < 2.5;   // the bonnet and bumper views sit in the car: nothing to mask
                let cot = 1.0 / (wv.cam_fov.clamp(0.4, 1.6) * 0.5).tan();
                let car = if clip[3] > 0.5 && !inside { [clip[0] / clip[3], clip[1] / clip[3], 1.5 * cot / clip[3], 1.0] } else { [0.0; 4] };
                mb.draw(&self.device, &self.queue, &mut enc, &frame.texture, &view, &self.depth.1, (self.config.width, self.config.height),
                    flatten(inverse(vp)), self.prev_vp.unwrap(), bp, blur_gpu::speed_gate(wv.vehicle.vel.len()), car);
            }
        }
        self.prev_vp = Some(flatten(vp)); self.prev_cam = wv.cam_pos;
        if let Some(p) = &self.prof { p.mark(&mut enc, 5); p.resolve(&mut enc); }
        self.queue.submit([enc.finish()]);
        if let Some(p) = self.prof.as_mut() { p.report(&self.device); }
        if let Some(dg) = self.dust_gpu.as_mut() { dg.prof_report(&self.device); }
        frame.present();
    }
}


/// GPU timings of the frame's stages (debug: the extra `gpuprof=1`): timestamps between the stages, read back and logged every 90 frames.
static BLUR_LEVEL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
/// The Settings option "Motion blur": 0 off, 1 low, 2 high.
pub fn set_blur(level: i32) { BLUR_LEVEL.store(level.clamp(0, 2), std::sync::atomic::Ordering::Relaxed); }

static PROF_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn set_prof(on: bool) { PROF_ON.store(on, std::sync::atomic::Ordering::Relaxed); }
pub fn prof_on() -> bool { PROF_ON.load(std::sync::atomic::Ordering::Relaxed) }
const PROF_N: u32 = 6;
const PROF_NAMES: [&str; 5] = ["airflow", "dust compute", "shadow+main", "dust low+comp", "rest"];

struct Prof { qs: wgpu::QuerySet, resolve: wgpu::Buffer, read: wgpu::Buffer, period: f32, frame: u32 }

impl Prof {
    fn new(dev: &wgpu::Device, period: f32) -> Prof {
        Prof { qs: dev.create_query_set(&wgpu::QuerySetDescriptor { label: Some("prof"), ty: wgpu::QueryType::Timestamp, count: PROF_N }),
            resolve: dev.create_buffer(&wgpu::BufferDescriptor { label: Some("prof_resolve"), size: 8 * PROF_N as u64, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false }),
            read: dev.create_buffer(&wgpu::BufferDescriptor { label: Some("prof_read"), size: 8 * PROF_N as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }),
            period, frame: 0 }
    }
    fn mark(&self, enc: &mut wgpu::CommandEncoder, i: u32) { if PROF_ON.load(std::sync::atomic::Ordering::Relaxed) { enc.write_timestamp(&self.qs, i); } }
    fn resolve(&self, enc: &mut wgpu::CommandEncoder) {
        if !PROF_ON.load(std::sync::atomic::Ordering::Relaxed) { return; }
        enc.resolve_query_set(&self.qs, 0..PROF_N, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, 8 * PROF_N as u64);
    }
    /// After the submit: every 90th frame wait for the frame and log the stage times.
    fn report(&mut self, dev: &wgpu::Device) {
        if !PROF_ON.load(std::sync::atomic::Ordering::Relaxed) { return; }
        self.frame += 1;
        if self.frame % 90 != 0 { return; }
        let slice = self.read.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        dev.poll(wgpu::Maintain::Wait);
        let ts: Vec<u64> = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();
        self.read.unmap();
        let ms = |a: usize| (ts[a + 1].wrapping_sub(ts[a])) as f32 * self.period / 1.0e6;
        let parts: Vec<String> = (0..5).map(|i| format!("{} {:.1}", PROF_NAMES[i], ms(i))).collect();
        log::info!("gpuprof ms: {} | total {:.1}", parts.join(", "), (ts[5].wrapping_sub(ts[0])) as f32 * self.period / 1.0e6);
    }
}
