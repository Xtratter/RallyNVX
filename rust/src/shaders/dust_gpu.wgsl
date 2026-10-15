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
    ph2: vec4<f32>,     // turbulence, the horizontal speed of the car, 0, 0
    car: vec4<f32>,     // car position, yaw
    car_vel: vec4<f32>,
    bx: vec4<f32>,      // the airflow box in metres (x, y, z) and its cell; cell 0 = no airflow
    gw: vec4<i32>,      // the first cell (x, z) of the ground window
    emits: array<S, 64>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read_write> state: array<S>;
@group(0) @binding(2) var<storage, read_write> render: array<DI>;
@group(0) @binding(3) var ground_tex: texture_2d<f32>;   // terrain heights of the window, toroidal (cell mod 64)
@group(1) @binding(0) var grid_vel: texture_3d<f32>;     // the airflow's velocity relative to the car, in the car's frame
@group(1) @binding(1) var grid_solid: texture_3d<f32>;

@compute @workgroup_size(64)
fn emit(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.x) { return; }
    state[(u.n.y + i) % u.n.z] = u.emits[i];
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
// the small-scale turbulence the grid cannot carry
fn turbulence(p: vec3<f32>) -> vec3<f32> {
    let t = u.g.y;
    let amp = u.ph2.x * (0.35 + 0.05 * u.ph2.y);
    let n = vec3<f32>(sin(p.y * 0.9 + t * 1.7 + p.z * 0.35) + 0.5 * sin(p.z * 1.9 - t * 2.3),
        0.6 * sin(p.x * 0.8 + t * 1.1 + p.z * 0.5),
        sin(p.y * 1.1 - t * 1.5 + p.x * 0.4) + 0.5 * sin(p.x * 2.1 + t * 2.9));
    return n * amp;
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
// the world air velocity: the box's air inside (relative to the car, in the car's frame), the wind outside, blended at the faces
fn air_at(p: vec3<f32>) -> vec3<f32> {
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
    return air + turbulence(p);
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
    let tau = select(u.ph.x, u.ph.y, coarse);
    let settle = select(u.ph.z, u.ph.w, coarse);
    var air = air_at(pos);
    if (any(air != air)) { air = vec3<f32>(0.0); }
    let target_v = vec3<f32>(air.x, air.y - settle, air.z);
    let k = 1.0 - exp(-dt / tau);
    vel = vel + (target_v - vel) * k;
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
    let size = s.a.w * (1.0 + select(1.7, 0.8, coarse) * sqrt(age) + 1.2 * x);
    let alpha = select(0.55, 0.6, coarse) * fade_in * pow(1.0 - x, 1.4);
    render[i] = DI(vec4<f32>(pos, size), vec4<f32>(s.c.xyz, alpha), vec4<f32>(s.d.y, x, select(0.0, 1.0, coarse), 0.0));
}
