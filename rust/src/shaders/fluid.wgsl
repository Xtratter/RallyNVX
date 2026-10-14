// The fluid dust: the compute passes of the wrap-around grid (the same algorithm as game/fluid.rs, which the host tests).
// A texel t of the window stands for the cell `start + posmod(t - start, n)`; the world position of a cell is (cell + 0.5) * dx.
struct F {
    start: vec4<i32>,   // first cell of the window; w = 1 when the window moved since the last step
    prev: vec4<i32>,    // first cell of the previous window
    g: vec4<f32>,       // dx, dt, settle (m/s), decay (1/s)
    wind: vec4<f32>,    // wind xyz, drag (1/s)
    car: vec4<f32>,     // car position xyz, yaw
    car_half: vec4<f32>,
    car_vel: vec4<f32>,
    wpos: array<vec4<f32>, 4>,   // wheel contact xyz, dust rate (density per second)
    wvel: array<vec4<f32>, 4>,   // the air velocity the wheel throws xyz, the radius of its influence
    wcol: array<vec4<f32>, 4>,   // dust colour rgb
};
@group(0) @binding(0) var<uniform> f: F;
@group(0) @binding(1) var vel_src: texture_3d<f32>;
@group(0) @binding(2) var vel_dst: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var dens_src: texture_3d<f32>;
@group(0) @binding(4) var dens_dst: texture_storage_3d<rgba16float, write>;
@group(0) @binding(5) var pres_src: texture_3d<f32>;
@group(0) @binding(6) var pres_dst: texture_storage_3d<r32float, write>;
@group(0) @binding(7) var div_src: texture_3d<f32>;
@group(0) @binding(8) var div_dst: texture_storage_3d<r32float, write>;
@group(0) @binding(9) var height: texture_2d<f32>;
@group(0) @binding(10) var smp: sampler;
@group(0) @binding(11) var solid_src: texture_3d<f32>;   // (velocity of the solid, 1) per texel, made by `solids` once per step
@group(0) @binding(12) var solid_dst: texture_storage_3d<rgba16float, write>;

fn pm(a: vec3<i32>, n: vec3<i32>) -> vec3<i32> { return ((a % n) + n) % n; }
fn cell_of(t: vec3<i32>, s: vec3<i32>, n: vec3<i32>) -> vec3<i32> { return s + pm(t - s, n); }
fn stale(t: vec3<i32>, n: vec3<i32>) -> bool { return f.start.w != 0 && any(cell_of(t, f.start.xyz, n) != cell_of(t, f.prev.xyz, n)); }
fn world(cell: vec3<i32>) -> vec3<f32> { return (vec3<f32>(cell) + vec3<f32>(0.5)) * f.g.x; }

// (velocity of the solid, 1) in a solid cell: the ground below the terrain, the car's box; (0, 0, 0, 0) in air
fn solid_calc(cell: vec3<i32>) -> vec4<f32> {
    let hn = vec2<i32>(textureDimensions(height));
    let h = textureLoad(height, vec2<i32>(((cell.x % hn.x) + hn.x) % hn.x, ((cell.z % hn.y) + hn.y) % hn.y), 0).x;
    let p = world(cell);
    if (p.y < h) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let q = p - f.car.xyz;
    let sy = sin(f.car.w); let cy = cos(f.car.w);
    let l = vec3<f32>(q.x * cy - q.z * sy, q.y, q.x * sy + q.z * cy);
    if (abs(l.x) < f.car_half.x && abs(l.y) < f.car_half.y && abs(l.z) < f.car_half.z) { return vec4<f32>(f.car_vel.xyz, 1.0); }
    return vec4<f32>(0.0);
}

fn solid_tex(c: vec3<i32>) -> vec4<f32> { return textureLoad(solid_src, c, 0); }

@compute @workgroup_size(4, 4, 4)
fn solids(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(solid_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    textureStore(solid_dst, c, solid_calc(cell_of(c, f.start.xyz, n)));
}

fn vel_load(c: vec3<i32>, n: vec3<i32>) -> vec3<f32> {
    let cc = pm(c, n);
    let s = solid_tex(cc);
    if (s.w > 0.5) { return s.xyz; }
    return textureLoad(vel_src, cc, 0).xyz;
}

@compute @workgroup_size(4, 4, 4)
fn advect_vel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(vel_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    let cell = cell_of(c, f.start.xyz, n);
    let s = solid_tex(c);
    if (s.w > 0.5) { textureStore(vel_dst, c, vec4<f32>(s.xyz, 0.0)); return; }
    let p = world(cell);
    let dt = f.g.y;
    var v = f.wind.xyz;
    if (!stale(c, n)) {
        let v0 = textureLoad(vel_src, c, 0).xyz;
        let back = p - v0 * dt;
        v = textureSampleLevel(vel_src, smp, back / f.g.x / vec3<f32>(n), 0.0).xyz;
        v = v + (f.wind.xyz - v) * min(f.wind.w * dt, 1.0);   // the air relaxes towards the wind
        for (var i = 0; i < 4; i++) {   // the wheels throw air
            let d = length(p - f.wpos[i].xyz);
            let r = max(f.wvel[i].w, 0.2);
            v = v + f.wvel[i].xyz * exp(-(d * d) / (r * r)) * dt * 6.0;
        }
    }
    textureStore(vel_dst, c, vec4<f32>(v, 0.0));
}

@compute @workgroup_size(4, 4, 4)
fn divergence(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(div_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    var d = 0.0;
    if (solid_tex(c).w < 0.5) {
        d = (vel_load(c + vec3<i32>(1, 0, 0), n).x - vel_load(c - vec3<i32>(1, 0, 0), n).x
           + vel_load(c + vec3<i32>(0, 1, 0), n).y - vel_load(c - vec3<i32>(0, 1, 0), n).y
           + vel_load(c + vec3<i32>(0, 0, 1), n).z - vel_load(c - vec3<i32>(0, 0, 1), n).z) / (2.0 * f.g.x);
    }
    textureStore(div_dst, c, vec4<f32>(d, 0.0, 0.0, 0.0));
}

fn pres_load(c: vec3<i32>, centre: f32, n: vec3<i32>) -> f32 {
    let cc = pm(c, n);
    if (solid_tex(cc).w > 0.5) { return centre; }
    return textureLoad(pres_src, cc, 0).x;
}

@compute @workgroup_size(4, 4, 4)
fn jacobi(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(pres_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    var p = 0.0;
    if (solid_tex(c).w < 0.5 && !stale(c, n)) {
        let centre = textureLoad(pres_src, c, 0).x;
        let sum = pres_load(c + vec3<i32>(1, 0, 0), centre, n) + pres_load(c - vec3<i32>(1, 0, 0), centre, n)
                + pres_load(c + vec3<i32>(0, 1, 0), centre, n) + pres_load(c - vec3<i32>(0, 1, 0), centre, n)
                + pres_load(c + vec3<i32>(0, 0, 1), centre, n) + pres_load(c - vec3<i32>(0, 0, 1), centre, n);
        p = (sum - textureLoad(div_src, c, 0).x * f.g.x * f.g.x) / 6.0;
    }
    textureStore(pres_dst, c, vec4<f32>(p, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(4, 4, 4)
fn project(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(vel_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    let s = solid_tex(c);
    if (s.w > 0.5) { textureStore(vel_dst, c, vec4<f32>(s.xyz, 0.0)); return; }
    let centre = textureLoad(pres_src, c, 0).x;
    let g = vec3<f32>(pres_load(c + vec3<i32>(1, 0, 0), centre, n) - pres_load(c - vec3<i32>(1, 0, 0), centre, n),
                      pres_load(c + vec3<i32>(0, 1, 0), centre, n) - pres_load(c - vec3<i32>(0, 1, 0), centre, n),
                      pres_load(c + vec3<i32>(0, 0, 1), centre, n) - pres_load(c - vec3<i32>(0, 0, 1), centre, n));
    let v = textureLoad(vel_src, c, 0).xyz - g / (2.0 * f.g.x);
    textureStore(vel_dst, c, vec4<f32>(v, 0.0));
}

// the dust: rgb = colour x density, a = density; carried by the air, settling, decaying, fed by the wheels
@compute @workgroup_size(4, 4, 4)
fn advect_dens(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = vec3<i32>(textureDimensions(dens_dst));
    let c = vec3<i32>(gid);
    if (any(c >= n)) { return; }
    let cell = cell_of(c, f.start.xyz, n);
    if (solid_tex(c).w > 0.5 || stale(c, n)) { textureStore(dens_dst, c, vec4<f32>(0.0)); return; }
    let p = world(cell);
    let dt = f.g.y;
    let v = textureLoad(vel_src, c, 0).xyz;
    let back = p - (v - vec3<f32>(0.0, f.g.z, 0.0)) * dt;
    var d = textureSampleLevel(dens_src, smp, back / f.g.x / vec3<f32>(n), 0.0) * max(1.0 - f.g.w * dt, 0.0);
    for (var i = 0; i < 4; i++) {
        let q = p - f.wpos[i].xyz;
        let w = exp(-dot(q, q) / 0.8) * f.wpos[i].w * dt;
        d = d + vec4<f32>(f.wcol[i].rgb * w, w);
    }
    textureStore(dens_dst, c, min(d, vec4<f32>(4.0)));
}
