// The dust on the GPU: the particles live in a ring buffer, the wheels' new ones are written in by `emit`, `step` moves every particle (the
// reference is game/dust_ref.rs, which the host tests) and writes what the dust sprites need (the layout of DustInst, a vertex buffer).
struct S {
    a: vec4<f32>,   // position, size at birth
    b: vec4<f32>,   // velocity, age (s)
    c: vec4<f32>,   // colour, life (s)
    d: vec4<f32>,   // coarse (1) or fine (0), seed, alive (1), 0
};
struct DI { p: vec4<f32>, c: vec4<f32>, e: vec4<f32> };
struct U {
    n: vec4<u32>,       // emits this frame, ring head, capacity, 0
    g: vec4<f32>,       // dt, time, ground y, 0
    wind: vec4<f32>,
    ph: vec4<f32>,      // tau fine, tau coarse, settle fine, settle coarse
    ph2: vec4<f32>,     // turbulence, the horizontal speed of the car, the plume's lift, the plume's height
    car: vec4<f32>,     // car position, yaw
    car_vel: vec4<f32>,
    bx: vec4<f32>,      // the airflow box in metres (x, y, z) and its cell; cell 0 = no airflow
    gw: vec4<i32>,      // the first cell (x, z) of the ground window
    cp: vec4<f32>,      // the dust's push, the masses of fine and coarse grit, 0
    sun: vec4<f32>,     // the direction to the sun (world), the absorption of the dust per unit of mass
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read_write> state: array<S>;
@group(0) @binding(2) var<storage, read_write> render: array<DI>;
@group(0) @binding(4) var<storage, read> emits: array<S>;   // this frame's new particles from the wheels
@group(0) @binding(3) var ground_tex: texture_2d<f32>;   // terrain heights of the window, toroidal (cell mod 64)
@group(1) @binding(0) var grid_vel: texture_3d<f32>;     // the airflow's velocity relative to the car, in the car's frame
@group(1) @binding(1) var grid_solid: texture_3d<f32>;
@group(1) @binding(2) var<storage, read_write> dens: array<atomic<i32>>;   // the dust field: per cell mass and momentum x, y, z (fixed point 1024), the airflow's cell order

@compute @workgroup_size(64)
fn emit(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.x) { return; }
    state[(u.n.y + i) % u.n.z] = emits[i];
}

const WINDOW: i32 = 64;
fn ground_cell(c: vec2<i32>) -> f32 {
    let b = u.gw.xy;
    if (any(c < b) || any(c >= b + vec2<i32>(WINDOW))) { return u.g.z; }
    return textureLoad(ground_tex, vec2<i32>(((c.x % WINDOW) + WINDOW) % WINDOW, ((c.y % WINDOW) + WINDOW) % WINDOW), 0).x;
}
// the terrain height at (x, z): bilinear over the window (game/dust_ref.rs::ground_height), the car's ground beyond it
fn ground(x: f32, z: f32) -> f32 {
    let g = vec2<f32>(x - 0.5, z - 0.5);
    let i = floor(g);
    let f = g - i;
    let c = vec2<i32>(i);
    let a = mix(ground_cell(c), ground_cell(c + vec2<i32>(1, 0)), f.x);
    let b = mix(ground_cell(c + vec2<i32>(0, 1)), ground_cell(c + vec2<i32>(1, 1)), f.x);
    return mix(a, b, f.y);
}
// the wind: weaker near the ground (the CPU path's `air_velocity` without the wake)
fn wind_at(p: vec3<f32>) -> vec3<f32> {
    let h = max(p.y - u.g.z, 0.0);
    return u.wind.xyz * (0.35 + 0.65 * min(h / 4.0, 1.0));
}
// ---- curl noise (game/dust_ref.rs::curl_noise, the host-tested twin): divergence-free turbulence, particles swirl without clumping
fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}
fn hash3(c: vec3<i32>, k: u32) -> f32 {
    let h = pcg(u32(c.x) ^ pcg(u32(c.y) ^ pcg(u32(c.z) ^ pcg(k))));
    return f32(h) / 4294967295.0 * 2.0 - 1.0;
}
fn fade(t: f32) -> f32 { return t * t * t * (t * (t * 6.0 - 15.0) + 10.0); }
fn vnoise3(p: vec3<f32>, k: u32) -> f32 {
    let i = floor(p);
    let f = fade(p.x - i.x); let g = fade(p.y - i.y); let h = fade(p.z - i.z);
    let c = vec3<i32>(i);
    let x00 = mix(hash3(c, k), hash3(c + vec3<i32>(1, 0, 0), k), f);
    let x10 = mix(hash3(c + vec3<i32>(0, 1, 0), k), hash3(c + vec3<i32>(1, 1, 0), k), f);
    let x01 = mix(hash3(c + vec3<i32>(0, 0, 1), k), hash3(c + vec3<i32>(1, 0, 1), k), f);
    let x11 = mix(hash3(c + vec3<i32>(0, 1, 1), k), hash3(c + vec3<i32>(1, 1, 1), k), f);
    return mix(mix(x00, x10, g), mix(x01, x11, g), h);
}
fn dnoise(q: vec3<f32>, k: u32, a: i32) -> f32 {
    let e = 0.1;
    var d = vec3<f32>(0.0);
    if (a == 0) { d.x = e; } else if (a == 1) { d.y = e; } else { d.z = e; }
    return (vnoise3(q + d, k) - vnoise3(q - d, k)) / (2.0 * e);
}
fn curl_noise(q: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dnoise(q, 2u, 1) - dnoise(q, 1u, 2), dnoise(q, 0u, 2) - dnoise(q, 2u, 0), dnoise(q, 1u, 0) - dnoise(q, 0u, 1));
}
// three octaves (6, 2.5, 1 m), carried along by the wind and evolving in time; `amp` is the speed of the largest octave
fn turbulence_curl(p: vec3<f32>, amp: f32) -> vec3<f32> {
    let t = u.g.y;
    let q = p - u.wind.xyz * t;
    var v = vec3<f32>(0.0);
    for (var i = 0; i < 3; i++) {
        let fi = f32(i);
        let lambda = select(select(6.0, 2.5, i == 1), 1.0, i == 2);
        let w = select(select(1.0, 0.55, i == 1), 0.3, i == 2);
        let shift = vec3<f32>(0.0, 0.0, t * 0.35 * (fi + 1.0)) + vec3<f32>(17.0 * fi, 31.0 * fi, 7.0 * fi);
        v = v + curl_noise(q / lambda + shift) * (amp * w * 0.35);
    }
    return v;
}
fn turbulence(p: vec3<f32>, age: f32) -> vec3<f32> {
    let amp = u.ph2.x * (0.35 + 0.05 * u.ph2.y) * (1.0 + 0.8 * age);   // the cloud expands: older dust is churned more
    return turbulence_curl(p, amp);
}
// the car's axes on the ground plane: forward (sin yaw, 0, cos yaw), right (-cos yaw, 0, sin yaw)
fn fwd_axis() -> vec3<f32> { return vec3<f32>(sin(u.car.w), 0.0, cos(u.car.w)); }
fn right_axis() -> vec3<f32> { let f = fwd_axis(); return vec3<f32>(-f.z, 0.0, f.x); }
// a world position in the box's coordinates (metres from its corner), as game/dust_ref.rs::to_box
fn to_box(p: vec3<f32>) -> vec3<f32> {
    let q = p - u.car.xyz;
    return vec3<f32>(dot(q, right_axis()) + u.bx.x * 0.5, p.y - u.g.z, dot(q, fwd_axis()) + u.bx.z * 0.5);
}
fn box_weight(c: vec3<f32>) -> f32 {
    if (c.y < 0.0) { return 0.0; }
    let d = min(min(min(c.x, u.bx.x - c.x), u.bx.y - c.y), min(c.z, u.bx.z - c.z));
    return smoothstep(0.0, 1.5, d);
}
fn grid_load(c: vec3<i32>) -> vec3<f32> {
    let n = vec3<i32>(textureDimensions(grid_vel));
    return textureLoad(grid_vel, clamp(c, vec3<i32>(0), n - vec3<i32>(1)), 0).xyz;
}
fn grid_sample(b: vec3<f32>) -> vec3<f32> {
    let g = b / u.bx.w - vec3<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let c = vec3<i32>(i);
    let x00 = mix(grid_load(c), grid_load(c + vec3<i32>(1, 0, 0)), f.x);
    let x10 = mix(grid_load(c + vec3<i32>(0, 1, 0)), grid_load(c + vec3<i32>(1, 1, 0)), f.x);
    let x01 = mix(grid_load(c + vec3<i32>(0, 0, 1)), grid_load(c + vec3<i32>(1, 0, 1)), f.x);
    let x11 = mix(grid_load(c + vec3<i32>(0, 1, 1)), grid_load(c + vec3<i32>(1, 1, 1)), f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}
fn in_solid(p: vec3<f32>) -> bool {
    if (u.bx.w <= 0.0) { return false; }
    let b = to_box(p) / u.bx.w;
    let n = vec3<i32>(textureDimensions(grid_solid));
    let c = vec3<i32>(floor(b));
    if (any(c < vec3<i32>(0)) || any(c >= n)) { return false; }
    return textureLoad(grid_solid, c, 0).x > 0.5;
}
// ---- the dust field: the density pushes the particles apart, the momentum drags the air (in the airflow's solver)
fn box_cells() -> vec3<i32> { return vec3<i32>(round(u.bx.xyz / u.bx.w)); }
fn dens_index(c: vec3<i32>) -> i32 { let n = box_cells(); return ((c.z * n.y + c.y) * n.x + c.x) * 4; }
fn mass_cell(c: vec3<i32>) -> f32 {
    let n = box_cells();
    if (any(c < vec3<i32>(0)) || any(c >= n)) { return 0.0; }
    return f32(atomicLoad(&dens[dens_index(c)])) / 1024.0;
}
fn mass_at(b: vec3<f32>) -> f32 {
    let g = b / u.bx.w - vec3<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let c = vec3<i32>(i);
    let x00 = mix(mass_cell(c), mass_cell(c + vec3<i32>(1, 0, 0)), f.x);
    let x10 = mix(mass_cell(c + vec3<i32>(0, 1, 0)), mass_cell(c + vec3<i32>(1, 1, 0)), f.x);
    let x01 = mix(mass_cell(c + vec3<i32>(0, 0, 1)), mass_cell(c + vec3<i32>(1, 0, 1)), f.x);
    let x11 = mix(mass_cell(c + vec3<i32>(0, 1, 1)), mass_cell(c + vec3<i32>(1, 1, 1)), f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}
// the acceleration (world) that pushes a particle away from denser dust (game/dust_ref.rs::Field::push)
fn push_at(p: vec3<f32>) -> vec3<f32> {
    if (u.bx.w <= 0.0) { return vec3<f32>(0.0); }
    let b = to_box(p);
    let w = box_weight(b);
    if (w <= 0.0) { return vec3<f32>(0.0); }
    let d = u.bx.w;
    let gx = (mass_at(b + vec3<f32>(d, 0.0, 0.0)) - mass_at(b - vec3<f32>(d, 0.0, 0.0))) * 0.5;
    let gy = (mass_at(b + vec3<f32>(0.0, d, 0.0)) - mass_at(b - vec3<f32>(0.0, d, 0.0))) * 0.5;
    let gz = (mass_at(b + vec3<f32>(0.0, 0.0, d)) - mass_at(b - vec3<f32>(0.0, 0.0, d))) * 0.5;
    let a = -u.cp.x * vec3<f32>(gx, gy, gz) * w;
    return right_axis() * a.x + vec3<f32>(0.0, a.y, 0.0) + fwd_axis() * a.z;
}

@compute @workgroup_size(64)
fn dust_clear(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = box_cells();
    let total = u32(n.x * n.y * n.z * 4);
    if (gid.x < total) { atomicStore(&dens[gid.x], 0); }
}

// every live particle adds its mass and momentum (relative to the car, in the car's frame) to its cell
@compute @workgroup_size(64)
fn dust_splat(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.z || u.bx.w <= 0.0) { return; }
    let s = state[i];
    if (s.d.z < 0.5) { return; }
    let b = to_box(s.a.xyz);
    let c = vec3<i32>(floor(b / u.bx.w));
    let n = box_cells();
    if (any(b < vec3<f32>(0.0)) || any(c >= n)) { return; }
    let m = select(u.cp.y, u.cp.z, s.d.x > 0.5);
    let rel = s.b.xyz - u.car_vel.xyz;
    let v = vec3<f32>(dot(rel, right_axis()), rel.y, dot(rel, fwd_axis()));
    let k = dens_index(c);
    atomicAdd(&dens[k], i32(m * 1024.0));
    atomicAdd(&dens[k + 1], i32(m * v.x * 1024.0));
    atomicAdd(&dens[k + 2], i32(m * v.y * 1024.0));
    atomicAdd(&dens[k + 3], i32(m * v.z * 1024.0));
}

// the world air velocity: the box's air inside (relative to the car, in the car's frame), the wind outside, blended at the faces
fn air_at(p: vec3<f32>, age: f32) -> vec3<f32> {
    let outside = wind_at(p);
    var air = outside;
    if (u.bx.w > 0.0) {
        let b = to_box(p);
        let w = box_weight(b);
        if (w > 0.0) {
            let gv = grid_sample(b);
            let inside = u.car_vel.xyz + right_axis() * gv.x + vec3<f32>(0.0, gv.y, 0.0) + fwd_axis() * gv.z;
            air = outside + (inside - outside) * w;
        }
    }
    // the plume: behind the car the turbulent wake carries the dust up, the higher the less (it hangs at about the plume's height)
    let behind = -dot(p - u.car.xyz, fwd_axis()) - 2.0;
    if (behind > 0.0 && u.ph2.z > 0.0) {
        let h = max(p.y - u.g.z, 0.0);
        air.y = air.y + u.ph2.z * u.ph2.y * exp(-behind / 12.0) * (1.0 - smoothstep(0.4 * u.ph2.w, u.ph2.w, h));
    }
    return air + turbulence(p, age);
}

@compute @workgroup_size(64)
fn dust_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.z) { return; }
    var s = state[i];
    var dead = DI(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    if (s.d.z < 0.5) { render[i] = dead; return; }
    let dt = u.g.x;
    var pos = s.a.xyz; var vel = s.b.xyz; var age = s.b.w + dt;
    let life = s.c.w;
    if (age >= life) { s.d.z = 0.0; state[i] = s; render[i] = dead; return; }
    let coarse = s.d.x > 0.5;
    let tau = select(u.ph.x, u.ph.y, coarse) * (0.6 + 0.8 * fract(s.d.y * 7.31));   // every grain answers the air a little differently
    let settle = select(u.ph.z, u.ph.w, coarse);
    var air = air_at(pos, age / life);
    if (any(air != air)) { air = vec3<f32>(0.0); }
    let target_v = vec3<f32>(air.x, air.y - settle, air.z);
    let k = 1.0 - exp(-dt / tau);
    vel = vel + (target_v - vel) * k;
    vel = vel + push_at(pos) * dt;   // pushed away from the dust around it
    let before = pos;
    pos = pos + vel * dt;
    if (in_solid(pos)) { pos = before; vel = vel * -0.2; }   // the car's body: it does not go through
    let g = ground(pos.x, pos.z) + 0.04;
    if (pos.y < g) {
        if (coarse) { s.d.z = 0.0; state[i] = s; render[i] = dead; return; }
        pos.y = g;
        if (vel.y < 0.0) { vel.y = 0.0; }
        let sl = 1.0 - min(2.5 * dt, 1.0);
        vel.x = vel.x * sl; vel.z = vel.z * sl;
    }
    s.a = vec4<f32>(pos, s.a.w); s.b = vec4<f32>(vel, age);
    state[i] = s;
    // what the sprite needs: it grows as it spreads, fades in over the first 8 % of its life and out at the end
    let x = clamp(age / life, 0.0, 1.0);
    let fade_in = min(x / 0.08, 1.0);
    let size = s.a.w * (1.0 + select(0.45, 0.25, coarse) * sqrt(age) + 0.35 * x);   // small grains: they hardly grow
    let alpha = select(0.14, 0.2, coarse) * fade_in * pow(1.0 - x, 1.4);
    // the light of a particle: the dust above it towards the sun shades it (6 samples through the dust field), the dense middle is darker
    var light = 1.0;
    var shade = 1.0;
    if (u.bx.w > 0.0) {
        let b = to_box(pos);
        let sb = vec3<f32>(dot(u.sun.xyz, right_axis()), u.sun.y, dot(u.sun.xyz, fwd_axis())) * 0.7;
        var acc = 0.0;
        for (var j = 1; j <= 6; j++) { acc = acc + mass_at(b + sb * f32(j)); }
        light = mix(0.25, 1.0, exp(-u.sun.w * acc * 0.7));
        shade = mix(1.0, 0.7, smoothstep(0.3, 3.0, mass_at(b)));
    }
    render[i] = DI(vec4<f32>(pos, size), vec4<f32>(s.c.xyz * shade, alpha), vec4<f32>(s.d.y, x, select(0.0, 1.0, coarse), light));
}
