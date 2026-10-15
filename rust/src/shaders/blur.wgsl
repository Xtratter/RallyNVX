// Camera motion blur, the speed effect: every pixel is reprojected with last frame's view-projection (its world position from the depth buffer), the
// difference is its velocity on the screen, and the picture is averaged along that velocity. The car (which moves with the camera) is masked out by an
// ellipse round its projected position so it stays sharp while the scenery streaks past. Optional (Settings), off by default.
struct B {
    inv_vp: mat4x4<f32>,
    prev_vp: mat4x4<f32>,
    p: vec4<f32>,     // shutter (how much of the frame's motion is blurred), the longest blur in uv units, aspect, taps
    car: vec4<f32>,   // the car's centre in ndc, its radius in ndc y, 1 when the mask is on
};
@group(0) @binding(0) var<uniform> b: B;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var dep: texture_depth_2d;
@group(0) @binding(3) var smp: sampler;

struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_blur(@builtin(vertex_index) i: u32) -> VOut {
    var q = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: VOut;
    o.pos = vec4<f32>(q[i], 0.0, 1.0);
    o.uv = vec2<f32>(q[i].x * 0.5 + 0.5, 0.5 - q[i].y * 0.5);
    return o;
}

fn ign(p: vec2<f32>) -> f32 { return fract(52.9829189 * fract(dot(p, vec2<f32>(0.06711056, 0.00583715)))); }   // interleaved gradient noise

@fragment
fn fs_blur(in: VOut) -> @location(0) vec4<f32> {
    let dims = vec2<f32>(textureDimensions(src));
    let px = vec2<i32>(in.pos.xy);
    let d = textureLoad(dep, px, 0);
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let w = b.inv_vp * vec4<f32>(ndc, d, 1.0);
    let world = w.xyz / w.w;
    let pc = b.prev_vp * vec4<f32>(world, 1.0);
    let pndc = pc.xy / max(pc.w, 0.0001);
    var vel = (ndc - pndc) * vec2<f32>(0.5, -0.5) * b.p.x;
    // the car stays sharp
    if (b.car.w > 0.5) {
        let dd = (ndc - b.car.xy) * vec2<f32>(b.p.z, 1.0) / max(b.car.z, 0.001);
        vel = vel * smoothstep(0.85, 1.25, length(dd));
    }
    let len = length(vel);
    if (len < 0.0004) { return textureSampleLevel(src, smp, in.uv, 0.0); }
    vel = vel * min(1.0, b.p.y / len);
    let taps = i32(b.p.w);
    let jit = ign(in.pos.xy);
    var acc = vec3<f32>(0.0);
    for (var i = 0; i < taps; i++) {
        let t = (f32(i) + jit) / f32(taps) - 0.5;
        acc = acc + textureSampleLevel(src, smp, clamp(in.uv + vel * t, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0).rgb;
    }
    return vec4<f32>(acc / f32(taps), 1.0);
}
