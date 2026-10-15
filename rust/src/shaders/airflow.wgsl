// The airflow around the car (aero step 2): the compute passes of the box that moves with the car, the same algorithm as game/airflow.rs
// (which the host tests). Cell (x, y, z): x lateral, y up, z forward, cell centre ((c + 0.5) * dx) in the box frame; the air comes at the car
// from the front (z max) with `inflow`, the road is the floor.
struct A {
    n: vec4<i32>,        // nx, ny, nz, number of boxes
    g: vec4<f32>,        // dx, dt, confinement, wake length in cells
    inflow: vec4<f32>,
    boxes: array<vec4<f32>, 16>,   // (min.xyz, 0), (max.xyz, 0) per box, x measured from the box's centre line
};
@group(0) @binding(0) var<uniform> a: A;
@group(0) @binding(1) var vel_src: texture_3d<f32>;
@group(0) @binding(2) var t_src: texture_3d<f32>;
@group(0) @binding(3) var pres_src: texture_3d<f32>;
@group(0) @binding(4) var div_src: texture_3d<f32>;
@group(0) @binding(5) var solid_src: texture_3d<f32>;
@group(0) @binding(6) var dst_v: texture_storage_3d<rgba16float, write>;
@group(0) @binding(7) var dst_s: texture_storage_3d<r32float, write>;

const WAKE_RATE: f32 = 10.0;
const WAKE_FLOOR: f32 = 0.2;

fn outside(c: vec3<i32>) -> bool { return c.z >= a.n.z || c.y < 0; }
fn outside_vel(c: vec3<i32>) -> vec3<f32> {
    if (c.z >= a.n.z) { return a.inflow.xyz; }
    return vec3<f32>(a.inflow.x, 0.0, a.inflow.z);   // the road
}
fn cl(c: vec3<i32>) -> vec3<i32> { return clamp(c, vec3<i32>(0), a.n.xyz - vec3<i32>(1)); }
fn is_solid(c: vec3<i32>) -> bool { return !outside(c) && textureLoad(solid_src, cl(c), 0).x > 0.5; }
// the velocity of a texture at a cell: beyond the box the incoming air, a solid is at rest
fn vload(t: texture_3d<f32>, c: vec3<i32>) -> vec3<f32> {
    if (outside(c)) { return outside_vel(c); }
    return textureLoad(t, cl(c), 0).xyz;
}
fn vload_s(t: texture_3d<f32>, c: vec3<i32>) -> vec3<f32> {
    if (outside(c)) { return outside_vel(c); }
    if (textureLoad(solid_src, cl(c), 0).x > 0.5) { return vec3<f32>(0.0); }
    return textureLoad(t, cl(c), 0).xyz;
}
fn centre(c: vec3<i32>) -> vec3<f32> { return (vec3<f32>(c) + vec3<f32>(0.5)) * a.g.x; }
// trilinear sample at a position in metres, with the box rules of `vload`
fn sample(t: texture_3d<f32>, p: vec3<f32>) -> vec3<f32> {
    let g = p / a.g.x - vec3<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let c0 = vec3<i32>(i);
    let x00 = mix(vload(t, c0), vload(t, c0 + vec3<i32>(1, 0, 0)), f.x);
    let x10 = mix(vload(t, c0 + vec3<i32>(0, 1, 0)), vload(t, c0 + vec3<i32>(1, 1, 0)), f.x);
    let x01 = mix(vload(t, c0 + vec3<i32>(0, 0, 1)), vload(t, c0 + vec3<i32>(1, 0, 1)), f.x);
    let x11 = mix(vload(t, c0 + vec3<i32>(0, 1, 1)), vload(t, c0 + vec3<i32>(1, 1, 1)), f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}
fn trace(p: vec3<f32>, v: vec3<f32>, dt: f32) -> vec3<f32> {
    let m = 4.0 * a.g.x;
    return p - clamp(v * dt, vec3<f32>(-m), vec3<f32>(m));
}

@compute @workgroup_size(4, 4, 4)
fn solids(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    var p = centre(c);
    p.x = p.x - f32(a.n.x) * a.g.x * 0.5;
    var s = 0.0;
    for (var i = 0; i < a.n.w; i++) {
        let lo = a.boxes[2 * i].xyz; let hi = a.boxes[2 * i + 1].xyz;
        if (all(p >= lo) && all(p < hi)) { s = 1.0; }
    }
    textureStore(dst_s, c, vec4<f32>(s, 0.0, 0.0, 0.0));
}

// BFECC, 1: forward
@compute @workgroup_size(4, 4, 4)
fn adv_fwd(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    let v = textureLoad(vel_src, c, 0).xyz;
    textureStore(dst_v, c, vec4<f32>(sample(vel_src, trace(centre(c), v, a.g.y)), 0.0));
}
// BFECC, 2: backward from the forward result, then the corrected field v + (v - back) / 2
@compute @workgroup_size(4, 4, 4)
fn adv_corr(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    let f = textureLoad(t_src, c, 0).xyz;
    let back = sample(t_src, trace(centre(c), f, -a.g.y));
    let v = textureLoad(vel_src, c, 0).xyz;
    textureStore(dst_v, c, vec4<f32>(v + 0.5 * (v - back), 0.0));
}
// BFECC, 3: the corrected field advected, clamped to stay near the plain result; the wake; the incoming air at the front
@compute @workgroup_size(4, 4, 4)
fn adv_fin(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    if (textureLoad(solid_src, c, 0).x > 0.5) { textureStore(dst_v, c, vec4<f32>(0.0)); return; }
    if (c.z == a.n.z - 1) { textureStore(dst_v, c, vec4<f32>(a.inflow.xyz, 0.0)); return; }
    let v0 = textureLoad(vel_src, c, 0).xyz;
    let q = trace(centre(c), v0, a.g.y);
    let plain = sample(vel_src, q);
    var v = sample(t_src, q);
    let m = 0.5 * max(abs(v0), abs(plain)) + vec3<f32>(1.0);
    v = clamp(v, plain - m, plain + m);
    // the wake: a solid up to the wake length upstream in the same column slows the air down, most right behind it
    let len = i32(a.g.w);
    for (var d = 1; d <= len; d++) {
        let n = c + vec3<i32>(0, 0, d);
        if (!outside(n) && textureLoad(solid_src, n, 0).x > 0.5) {
            let t = f32(d) / f32(len);
            let k = min(WAKE_RATE * (1.0 - t) * a.g.y, 1.0);
            v = v + (a.inflow.xyz * (WAKE_FLOOR + (1.0 - WAKE_FLOOR) * t) - v) * k;
            break;
        }
    }
    textureStore(dst_v, c, vec4<f32>(v, 0.0));
}

// the vorticity (xyz) and its magnitude (w)
@compute @workgroup_size(4, 4, 4)
fn vort(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    let h = 2.0 * a.g.x;
    let xp = vload_s(vel_src, c + vec3<i32>(1, 0, 0)); let xm = vload_s(vel_src, c - vec3<i32>(1, 0, 0));
    let yp = vload_s(vel_src, c + vec3<i32>(0, 1, 0)); let ym = vload_s(vel_src, c - vec3<i32>(0, 1, 0));
    let zp = vload_s(vel_src, c + vec3<i32>(0, 0, 1)); let zm = vload_s(vel_src, c - vec3<i32>(0, 0, 1));
    let w = vec3<f32>((yp.z - ym.z - zp.y + zm.y) / h, (zp.x - zm.x - xp.z + xm.z) / h, (xp.y - xm.y - yp.x + ym.x) / h);
    textureStore(dst_v, c, vec4<f32>(w, length(w)));
}
fn mag_at(c: vec3<i32>) -> f32 { return textureLoad(t_src, cl(c), 0).w; }
// vorticity confinement
@compute @workgroup_size(4, 4, 4)
fn confine(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    var v = textureLoad(vel_src, c, 0).xyz;
    if (textureLoad(solid_src, c, 0).x < 0.5 && c.z != a.n.z - 1) {
        let grad = vec3<f32>(mag_at(c + vec3<i32>(1, 0, 0)) - mag_at(c - vec3<i32>(1, 0, 0)), mag_at(c + vec3<i32>(0, 1, 0)) - mag_at(c - vec3<i32>(0, 1, 0)), mag_at(c + vec3<i32>(0, 0, 1)) - mag_at(c - vec3<i32>(0, 0, 1)));
        let l = length(grad);
        if (l > 1e-5) {
            let w = textureLoad(t_src, c, 0).xyz;
            v = v + cross(grad / l, w) * a.g.z * a.g.x * a.g.y;
        }
    }
    textureStore(dst_v, c, vec4<f32>(v, 0.0));
}

@compute @workgroup_size(4, 4, 4)
fn divergence(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    var d = 0.0;
    if (textureLoad(solid_src, c, 0).x < 0.5) {
        d = (vload_s(vel_src, c + vec3<i32>(1, 0, 0)).x - vload_s(vel_src, c - vec3<i32>(1, 0, 0)).x
           + vload_s(vel_src, c + vec3<i32>(0, 1, 0)).y - vload_s(vel_src, c - vec3<i32>(0, 1, 0)).y
           + vload_s(vel_src, c + vec3<i32>(0, 0, 1)).z - vload_s(vel_src, c - vec3<i32>(0, 0, 1)).z) / (2.0 * a.g.x);
    }
    textureStore(dst_s, c, vec4<f32>(d, 0.0, 0.0, 0.0));
}

// the pressure of a neighbour: 0 beyond the back (open), the cell's own against a solid; the road and the sides and top copy the cell
fn pres_nb(c: vec3<i32>, n: vec3<i32>, centre_p: f32) -> f32 {
    if (n.z < 0) { return 0.0; }
    if (n.z >= a.n.z) { return 0.0; }
    if (is_solid(n)) { return centre_p; }
    return textureLoad(pres_src, cl(n), 0).x;
}
@compute @workgroup_size(4, 4, 4)
fn jacobi(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    var p = 0.0;
    if (textureLoad(solid_src, c, 0).x < 0.5 && c.z != a.n.z - 1) {
        let me = textureLoad(pres_src, c, 0).x;
        let s = pres_nb(c, c + vec3<i32>(1, 0, 0), me) + pres_nb(c, c - vec3<i32>(1, 0, 0), me) + pres_nb(c, c + vec3<i32>(0, 1, 0), me)
              + pres_nb(c, c - vec3<i32>(0, 1, 0), me) + pres_nb(c, c + vec3<i32>(0, 0, 1), me) + pres_nb(c, c - vec3<i32>(0, 0, 1), me);
        p = (s - textureLoad(div_src, c, 0).x * a.g.x * a.g.x) / 6.0;
    }
    textureStore(dst_s, c, vec4<f32>(p, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(4, 4, 4)
fn project(@builtin(global_invocation_id) gid: vec3<u32>) {
    let c = vec3<i32>(gid);
    if (any(c >= a.n.xyz)) { return; }
    var v = textureLoad(vel_src, c, 0).xyz;
    if (textureLoad(solid_src, c, 0).x > 0.5) { v = vec3<f32>(0.0); }
    else if (c.z != a.n.z - 1) {
        let me = textureLoad(pres_src, c, 0).x;
        let g = vec3<f32>(pres_nb(c, c + vec3<i32>(1, 0, 0), me) - pres_nb(c, c - vec3<i32>(1, 0, 0), me),
                          pres_nb(c, c + vec3<i32>(0, 1, 0), me) - pres_nb(c, c - vec3<i32>(0, 1, 0), me),
                          pres_nb(c, c + vec3<i32>(0, 0, 1), me) - pres_nb(c, c - vec3<i32>(0, 0, 1), me));
        v = v - g / (2.0 * a.g.x);
    }
    textureStore(dst_v, c, vec4<f32>(v, 0.0));
}

// the debug view: a side slice through the car's middle plane (the car faces right), speed relative to the inflow, solids black
struct SOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs_slice(@builtin(vertex_index) i: u32) -> SOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: SOut;
    o.pos = vec4<f32>(p * 2.0 - vec2<f32>(1.0), 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}
@fragment
fn fs_slice(in: SOut) -> @location(0) vec4<f32> {
    let n = a.n.xyz;
    let c = vec3<i32>(n.x / 2, i32((1.0 - in.uv.y) * f32(n.y)), i32(in.uv.x * f32(n.z)));
    if (textureLoad(solid_src, clamp(c, vec3<i32>(0), n - vec3<i32>(1)), 0).x > 0.5) { return vec4<f32>(0.05, 0.05, 0.05, 1.0); }
    let v = textureLoad(vel_src, clamp(c, vec3<i32>(0), n - vec3<i32>(1)), 0).xyz;
    let r = clamp(length(v) / max(length(a.inflow.xyz), 1.0), 0.0, 1.6);
    var col = mix(vec3<f32>(0.1, 0.2, 0.9), vec3<f32>(0.95, 0.95, 0.95), clamp(r, 0.0, 1.0));   // slow blue .. free-stream white
    col = mix(col, vec3<f32>(0.95, 0.3, 0.1), clamp((r - 1.0) / 0.6, 0.0, 1.0));              // faster than the inflow: red
    return vec4<f32>(col, 1.0);
}
