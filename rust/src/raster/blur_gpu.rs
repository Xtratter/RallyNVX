//! Camera motion blur as a post pass (the Settings option "Motion blur"): the finished frame is copied, then a full-screen pass reads the copy and the depth
//! buffer and writes the blurred frame back (`shaders/blur.wgsl`).
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BUni { inv_vp: [f32; 16], prev_vp: [f32; 16], p: [f32; 4], car: [f32; 4] }

/// How the blur is set: the share of the frame's motion that is blurred, the longest blur as a share of the screen's width, the taps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlurParams { pub shutter: f32, pub max_len: f32, pub taps: u32 }

/// The levels of the setting: 1 low, 2 high (0 is off, no blur at all).
pub fn params(level: i32) -> Option<BlurParams> {
    match level { 1 => Some(BlurParams { shutter: 1.0, max_len: 0.022, taps: 8 }), l if l >= 2 => Some(BlurParams { shutter: 2.0, max_len: 0.05, taps: 12 }), _ => None }
}

/// How much of the blur the car's speed (m/s) allows: none when crawling, all of it from about 20 m/s, so an orbit round a parked car stays sharp.
pub fn speed_gate(speed: f32) -> f32 { let t = ((speed.abs() - 3.0) / 17.0).clamp(0.0, 1.0); t * t * (3.0 - 2.0 * t) }

pub struct MotionBlur {
    pl: wgpu::RenderPipeline, bgl: wgpu::BindGroupLayout, smp: wgpu::Sampler, ubuf: wgpu::Buffer,
    copy: Option<((u32, u32), wgpu::Texture, wgpu::TextureView)>,
}

impl MotionBlur {
    pub fn new(dev: &wgpu::Device, format: wgpu::TextureFormat) -> MotionBlur {
        let f = wgpu::ShaderStages::FRAGMENT;
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("blur"), entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: f, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: f, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: f, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: f, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ] });
        let module = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("blur"), source: wgpu::ShaderSource::Wgsl(crate::shaders::blur().into()) });
        let lay = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
        let pl = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor { label: Some("blur"), layout: Some(&lay),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_blur"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs_blur"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview: None, cache: None });
        let smp = dev.create_sampler(&wgpu::SamplerDescriptor { label: Some("blur_smp"), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let ubuf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("blur_u"), size: std::mem::size_of::<BUni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        MotionBlur { pl, bgl, smp, ubuf, copy: None }
    }

    /// Copies `frame` and draws it blurred back into `target`. `inv_vp` and `prev_vp` are flattened matrices; `car` is the car's centre in ndc, its ndc radius and
    /// whether to mask it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&mut self, dev: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, frame: &wgpu::Texture, target: &wgpu::TextureView, depth: &wgpu::TextureView,
                size: (u32, u32), inv_vp: [f32; 16], prev_vp: [f32; 16], p: BlurParams, gate: f32, car: [f32; 4]) {
        if self.copy.as_ref().map_or(true, |c| c.0 != size) {
            let t = dev.create_texture(&wgpu::TextureDescriptor { label: Some("blur_copy"), size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1,
                dimension: wgpu::TextureDimension::D2, format: frame.format(), usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
            let v = t.create_view(&Default::default());
            self.copy = Some((size, t, v));
        }
        let (_, tex, view) = self.copy.as_ref().unwrap();
        enc.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo { texture: frame, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 });
        let u = BUni { inv_vp, prev_vp, p: [p.shutter * gate, p.max_len, size.0 as f32 / size.1 as f32, p.taps as f32], car };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let bg = dev.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("blur_bg"), layout: &self.bgl, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: self.ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(depth) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.smp) },
        ] });
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("blur"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None });
        rp.set_pipeline(&self.pl);
        rp.set_bind_group(0, &bg, &[]);
        rp.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn levels_are_off_low_and_high() {
        assert!(params(0).is_none() && params(-1).is_none());
        let (lo, hi) = (params(1).unwrap(), params(2).unwrap());
        assert!(hi.shutter > lo.shutter && hi.max_len > lo.max_len && hi.taps >= lo.taps);
        assert_eq!(params(9), params(2));
    }
    #[test] fn the_speed_gate_is_closed_when_crawling_open_when_fast_and_symmetric() {
        assert_eq!(speed_gate(0.0), 0.0); assert_eq!(speed_gate(3.0), 0.0);
        assert_eq!(speed_gate(20.0), 1.0); assert_eq!(speed_gate(60.0), 1.0);
        assert!(speed_gate(10.0) > 0.0 && speed_gate(10.0) < 1.0);
        assert_eq!(speed_gate(-15.0), speed_gate(15.0));
        let mut last = 0.0;
        for i in 0..40 { let g = speed_gate(i as f32); assert!(g >= last); last = g; }
    }
}
