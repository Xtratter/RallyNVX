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
    cp: vec4<f32>,      // the dust's push, the masses of fine and coarse grit, the strength of the vortex street
    sun: vec4<f32>,     // the direction to the sun (world), the absorption of the dust per unit of mass
    body: vec4<f32>,    // the car's half width, half length, height, and the strength of its trailing vortices
    bpos: vec4<f32>,    // where the body mesh is drawn (world) and its three axes: the mesh frame's x, y, z in the world
    bax0: vec4<f32>,
    bax1: vec4<f32>,
    bax2: vec4<f32>,
    sdf_o: vec4<f32>,   // the signed distance field of the body: its corner in the mesh frame, its voxel
    sdf_n: vec4<f32>,   // its size in voxels, 1 when there is one
    rs: vec4<u32>,      // the reserved ring of lifted particles: its start (= the main ring's size), its size, the frame number, 0
    pb: vec4<f32>,      // the particle pressure: the kernel radius, the rest density, the relaxation, the viscosity
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read_write> state: array<S>;
@group(0) @binding(2) var<storage, read_write> render: array<DI>;
@group(0) @binding(4) var<storage, read> emits: array<S>;   // this frame's new particles from the wheels
@group(0) @binding(7) var<storage, read> colliders: array<vec4<f32>>;   // per collider two vec4: (a, r) and (b, 0): trunk capsules and rock spheres
@group(0) @binding(8) var<storage, read> ccells: array<vec4<u32>>;     // per cell of the ground window up to four collider indices + 1
@group(0) @binding(9) var<storage, read_write> dep: array<atomic<u32>>;       // the settled layer per cell of the ground window (fixed point, 256 = one unit); the last element is the reserved ring's head
@group(0) @binding(10) var<storage, read_write> dep_cell: array<vec2<i32>>;   // the world cell each slot holds now
@group(0) @binding(11) var dep_img: texture_storage_2d<rgba8unorm, write>;      // the layer as a texture for the terrain and road shaders
@group(0) @binding(12) var<storage, read_write> pbd_grid: array<atomic<u32>>;   // the pressure's cell lists: a count per cell, then 8 slots (particle index + 1) per cell
@group(0) @binding(13) var<storage, read_write> pbd_dat: array<vec4<f32>>;      // per taking part particle two vec4: (correction, lambda) and (velocity change, 0)
@group(0) @binding(3) var ground_tex: texture_2d<f32>;   // terrain heights of the window, toroidal (cell mod 64)
@group(1) @binding(0) var grid_vel: texture_3d<f32>;     // the airflow's velocity relative to the car, in the car's frame
@group(1) @binding(1) var grid_solid: texture_3d<f32>;
@group(0) @binding(5) var<storage, read_write> mass: array<f32>;      // the dust mass per cell (floats)
@group(0) @binding(6) var<storage, read_write> fld: array<vec4<f32>>;  // per cell: the mass gradient and the sun light
@group(1) @binding(3) var sdf_tex: texture_3d<f32>;   // the body mesh's signed distance field (negative inside)
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
// ---- the world: trees, rocks (the cell lists of the ground window) and the slope of the ground
fn world_collide(pos: ptr<function, vec3<f32>>, vel: ptr<function, vec3<f32>>) {
    let c = vec2<i32>(floor((*pos).xz));
    let b = u.gw.xy;
    if (any(c < b) || any(c >= b + vec2<i32>(WINDOW))) { return; }
    let cell = ccells[((c.y % WINDOW + WINDOW) % WINDOW) * WINDOW + ((c.x % WINDOW + WINDOW) % WINDOW)];
    for (var k = 0; k < 4; k++) {
        let id = cell[k];
        if (id == 0u) { break; }
        let a0 = colliders[(id - 1u) * 2u]; let b0 = colliders[(id - 1u) * 2u + 1u];
        let ab = b0.xyz - a0.xyz;
        let l2 = dot(ab, ab);
        var q = a0.xyz;
        if (l2 > 1e-9) { q = a0.xyz + ab * clamp(dot(*pos - a0.xyz, ab) / l2, 0.0, 1.0); }
        let d = *pos - q;
        let len = length(d);
        let reach = a0.w + 0.1;
        if (len < reach) {
            var n = vec3<f32>(0.0, 1.0, 0.0);
            if (len > 1e-5) { n = d / len; }
            *pos = q + n * reach;
            *vel = *vel - n * min(dot(*vel, n), 0.0);
        }
    }
}
fn ground_normal(x: f32, z: f32) -> vec3<f32> {
    let e = 0.5;
    return normalize(vec3<f32>(-(ground(x + e, z) - ground(x - e, z)) / (2.0 * e), 1.0, -(ground(x, z + e) - ground(x, z - e)) / (2.0 * e)));
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
fn dfade(t: f32) -> f32 { return 30.0 * t * t * (t - 1.0) * (t - 1.0); }
// the value noise and its analytic gradient in one go (game/dust_ref.rs::vnoise3_grad)
fn vnoise3_grad(p: vec3<f32>, k: u32) -> vec4<f32> {
    let i = floor(p);
    let t = p - i;
    let u3 = vec3<f32>(fade(t.x), fade(t.y), fade(t.z));
    let d3 = vec3<f32>(dfade(t.x), dfade(t.y), dfade(t.z));
    let c = vec3<i32>(i);
    let c000 = hash3(c, k); let c100 = hash3(c + vec3<i32>(1, 0, 0), k); let c010 = hash3(c + vec3<i32>(0, 1, 0), k); let c110 = hash3(c + vec3<i32>(1, 1, 0), k);
    let c001 = hash3(c + vec3<i32>(0, 0, 1), k); let c101 = hash3(c + vec3<i32>(1, 0, 1), k); let c011 = hash3(c + vec3<i32>(0, 1, 1), k); let c111 = hash3(c + vec3<i32>(1, 1, 1), k);
    let x00 = mix(c000, c100, u3.x); let x10 = mix(c010, c110, u3.x); let x01 = mix(c001, c101, u3.x); let x11 = mix(c011, c111, u3.x);
    let y0 = mix(x00, x10, u3.y); let y1 = mix(x01, x11, u3.y);
    let dx = d3.x * mix(mix(c100 - c000, c110 - c010, u3.y), mix(c101 - c001, c111 - c011, u3.y), u3.z);
    let dy = d3.y * mix(x10 - x00, x11 - x01, u3.z);
    let dz = d3.z * (y1 - y0);
    return vec4<f32>(mix(y0, y1, u3.z), dx, dy, dz);
}
fn curl_noise(q: vec3<f32>) -> vec3<f32> {
    let g0 = vnoise3_grad(q, 0u).yzw; let g1 = vnoise3_grad(q, 1u).yzw; let g2 = vnoise3_grad(q, 2u).yzw;
    return vec3<f32>(g2.y - g1.z, g0.z - g2.x, g1.x - g0.y);
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
    var wake = 0.0;
    if (u.bx.w > 0.0) {
        let behind = -dot(p - u.car.xyz, fwd_axis()) - u.body.y;
        wake = exp(-max(behind, 0.0) / 10.0) * smoothstep(-1.0, 1.0, behind) * box_weight(to_box(p));
    }
    // the ambient air's turbulence (what slowly spreads an old cloud) plus the car's wake behind it, which churns the dust hard for the first seconds only
    let amp = u.ph2.x * (1.2 + (0.6 + 0.05 * u.ph2.y) * 2.5 * wake);
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
// The per-cell field the particles read (plain loads, no atomics): `mass` (the splat counters as floats) and `fld` = (the mass gradient, the sun light),
// both made once per frame by `dust_field_mass` and `dust_field_light` from the counters.
fn cell_ok(c: vec3<i32>) -> bool { let n = box_cells(); return all(c >= vec3<i32>(0)) && all(c < n); }
fn cell_index(c: vec3<i32>) -> i32 { let n = box_cells(); return (c.z * n.y + c.y) * n.x + c.x; }
fn mass_cell(c: vec3<i32>) -> f32 { if (!cell_ok(c)) { return 0.0; } return mass[cell_index(c)]; }
fn fld_cell(c: vec3<i32>) -> vec4<f32> { if (!cell_ok(c)) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } return fld[cell_index(c)]; }
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
fn fld_at(b: vec3<f32>) -> vec4<f32> {
    let g = b / u.bx.w - vec3<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let c = vec3<i32>(i);
    let x00 = mix(fld_cell(c), fld_cell(c + vec3<i32>(1, 0, 0)), f.x);
    let x10 = mix(fld_cell(c + vec3<i32>(0, 1, 0)), fld_cell(c + vec3<i32>(1, 1, 0)), f.x);
    let x01 = mix(fld_cell(c + vec3<i32>(0, 0, 1)), fld_cell(c + vec3<i32>(1, 0, 1)), f.x);
    let x11 = mix(fld_cell(c + vec3<i32>(0, 1, 1)), fld_cell(c + vec3<i32>(1, 1, 1)), f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}
// the acceleration (world) that pushes a particle away from denser dust: minus the mass gradient of its cell (game/dust_ref.rs::Field::push)
fn push_from(g: vec3<f32>, w: f32) -> vec3<f32> {
    let a = -u.cp.x * g * w;
    return right_axis() * a.x + vec3<f32>(0.0, a.y, 0.0) + fwd_axis() * a.z;
}

// ---- the settled layer
fn dep_slot(c: vec2<i32>) -> i32 { return ((c.y % WINDOW + WINDOW) % WINDOW) * WINDOW + ((c.x % WINDOW + WINDOW) % WINDOW); }
fn add_deposit(p: vec3<f32>, amount: u32) {
    let c = vec2<i32>(floor(p.xz));
    let b = u.gw.xy;
    if (any(c < b) || any(c >= b + vec2<i32>(WINDOW))) { return; }
    atomicAdd(&dep[dep_slot(c)], amount);
}
fn hash_u(k: u32) -> f32 { return f32(pcg(k)) / 4294967295.0; }

// per cell of the ground window: new cells start empty, the layer decays slowly, the car's wake lifts what lies in its way back into the air (new particles in the
// reserved ring), and the result goes to the texture the ground is tinted by
@compute @workgroup_size(64)
fn dust_deposit(@builtin(global_invocation_id) gid: vec3<u32>) {
    let id = i32(gid.x);
    if (id >= WINDOW * WINDOW) { return; }
    let sl = vec2<i32>(id % WINDOW, id / WINDOW);
    let b = u.gw.xy;
    let cell = b + (((sl - b) % WINDOW) + WINDOW) % WINDOW;
    if (any(dep_cell[id] != cell)) { dep_cell[id] = cell; atomicStore(&dep[id], 0u); }
    var v = atomicLoad(&dep[id]);
    if (v > 0u) {
        atomicSub(&dep[id], v / 4096u);
        v = atomicLoad(&dep[id]);
        // the wake of the car: behind it, within its width and a little more
        let q = vec2<f32>(cell) + vec2<f32>(0.5) - u.car.xz;
        let f = fwd_axis(); let r = right_axis();
        let along = q.x * f.x + q.y * f.z;
        let lat = q.x * r.x + q.y * r.z;
        let hw = u.body.x; let hl = u.body.y;
        let w = smoothstep(-hl - 9.0, -hl - 1.0, along) * (1.0 - smoothstep(hl, hl + 1.0, along)) * (1.0 - smoothstep(hw + 0.5, hw + 1.5, abs(lat)));
        let sp = clamp(u.ph2.y / 25.0, 0.0, 1.0);
        let take = f32(v) * 0.16 * sp * sp * w;
        var n = u32(take / 64.0 + hash_u(u32(id) * 977u + u.rs.z * 31u));
        n = min(n, 3u);
        if (n > 0u) {
            atomicSub(&dep[id], min(v, n * 64u));
            for (var k = 0u; k < n; k++) {
                let kk = u32(id) * 131u + u.rs.z * 7919u + k * 17u;
                let slot = u.rs.x + (atomicAdd(&dep[WINDOW * WINDOW], 1u) % u.rs.y);
                let px = vec3<f32>(f32(cell.x) + hash_u(kk + 1u), ground(f32(cell.x) + 0.5, f32(cell.y) + 0.5) + 0.06, f32(cell.y) + hash_u(kk + 2u));
                let vel = vec3<f32>(u.car_vel.x * 0.2 * hash_u(kk + 3u), 0.4 + 1.4 * hash_u(kk + 4u), u.car_vel.z * 0.2 * hash_u(kk + 5u));
                var ns: S;
                ns.a = vec4<f32>(px, 0.1);
                ns.b = vec4<f32>(vel, 0.0);
                ns.c = vec4<f32>(0.78, 0.62, 0.45, 3.5);
                ns.d = vec4<f32>(0.0, hash_u(kk + 6u), 1.0, 0.0);
                state[slot] = ns;
            }
            v = atomicLoad(&dep[id]);
        }
    }
    textureStore(dep_img, sl, vec4<f32>(clamp(f32(v) / 1024.0, 0.0, 1.0), 0.0, 0.0, 1.0));
}

// ---- particle pressure (position based fluids; game/dust_ref.rs::poly6, spiky_grad and pbd_corrections are the host-tested twins)
const PBD_K = 8u;
const PBD_STRIDE = 8u;
fn poly6(r2: f32) -> f32 {
    let h = u.pb.x; let h2 = h * h;
    if (r2 >= h2) { return 0.0; }
    let d = h2 - r2;
    return 315.0 / (64.0 * 3.14159265 * pow(h, 9.0)) * d * d * d;
}
fn spiky_grad(r: vec3<f32>) -> vec3<f32> {
    let h = u.pb.x; let l = length(r);
    if (l >= h || l < 1e-6) { return vec3<f32>(0.0); }
    return r * (-45.0 / (3.14159265 * pow(h, 6.0)) * (h - l) * (h - l) / l);
}
fn pbd_total_cells() -> u32 { let n = box_cells(); return u32(n.x * n.y * n.z); }
fn pbd_cell(p: vec3<f32>) -> i32 {
    if (u.bx.w <= 0.0) { return -1; }
    let c = vec3<i32>(floor(to_box(p) / u.bx.w));
    if (!cell_ok(c)) { return -1; }
    return cell_index(c);
}
// the particle a taking-part thread stands for (-1: none)
fn pbd_particle(tid: u32) -> i32 {
    let i = tid * PBD_STRIDE;
    if (i >= u.n.z + u.rs.y) { return -1; }
    let s = state[i];
    if (s.d.z < 0.5 || s.d.x > 1.5) { return -1; }
    return i32(i);
}
@compute @workgroup_size(64)
fn pbd_clear(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x < pbd_total_cells()) { atomicStore(&pbd_grid[gid.x], 0u); }
}
@compute @workgroup_size(64)
fn pbd_bin(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p = pbd_particle(gid.x);
    if (p < 0) { return; }
    let ci = pbd_cell(state[p].a.xyz);
    if (ci < 0) { return; }
    let k = atomicAdd(&pbd_grid[ci], 1u);
    if (k < PBD_K) { atomicStore(&pbd_grid[pbd_total_cells() + u32(ci) * PBD_K + k], u32(p) + 1u); }
}
// the constraint multiplier of every particle: lambda = -C / (sum of the squared gradients + eps), C = density / rest density - 1 (only compression is resisted)
@compute @workgroup_size(64)
fn pbd_lambda(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p = pbd_particle(gid.x);
    if (p < 0) { return; }
    let pos = state[p].a.xyz;
    let b = to_box(pos);
    let c0 = vec3<i32>(floor(b / u.bx.w));
    let total = pbd_total_cells();
    var rho = 0.0; var gsum = vec3<f32>(0.0); var sq = 0.0;
    for (var dz = -1; dz <= 1; dz++) { for (var dy = -1; dy <= 1; dy++) { for (var dx = -1; dx <= 1; dx++) {
        let c = c0 + vec3<i32>(dx, dy, dz);
        if (!cell_ok(c)) { continue; }
        let ci = u32(cell_index(c));
        let cnt = min(atomicLoad(&pbd_grid[ci]), PBD_K);
        for (var k = 0u; k < cnt; k++) {
            let j = i32(atomicLoad(&pbd_grid[total + ci * PBD_K + k])) - 1;
            if (j < 0) { continue; }
            let r = pos - state[j].a.xyz;
            rho = rho + poly6(dot(r, r));
            if (j != p) { let g = spiky_grad(r) / u.pb.y; gsum = gsum + g; sq = sq + dot(g, g); }
        }
    } } }
    let cc = rho / u.pb.y - 1.0;
    var lam = 0.0;
    if (cc > 0.0) { lam = -cc / (sq + dot(gsum, gsum) + u.pb.z); }
    pbd_dat[gid.x * 2u] = vec4<f32>(0.0, 0.0, 0.0, lam);
}
// the position correction, with the artificial pressure against clumping, and the XSPH velocity smoothing
@compute @workgroup_size(64)
fn pbd_delta(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p = pbd_particle(gid.x);
    if (p < 0) { return; }
    let pos = state[p].a.xyz; let vel = state[p].b.xyz;
    let lam_i = pbd_dat[gid.x * 2u].w;
    let c0 = vec3<i32>(floor(to_box(pos) / u.bx.w));
    let total = pbd_total_cells();
    let w_dq = max(poly6(0.04 * u.pb.x * u.pb.x), 1e-12);
    var d = vec3<f32>(0.0); var dv = vec3<f32>(0.0);
    for (var dz = -1; dz <= 1; dz++) { for (var dy = -1; dy <= 1; dy++) { for (var dx = -1; dx <= 1; dx++) {
        let c = c0 + vec3<i32>(dx, dy, dz);
        if (!cell_ok(c)) { continue; }
        let ci = u32(cell_index(c));
        let cnt = min(atomicLoad(&pbd_grid[ci]), PBD_K);
        for (var k = 0u; k < cnt; k++) {
            let j = i32(atomicLoad(&pbd_grid[total + ci * PBD_K + k])) - 1;
            if (j < 0 || j == p) { continue; }
            let r = pos - state[j].a.xyz;
            let lam_j = pbd_dat[(u32(j) / PBD_STRIDE) * 2u].w;
            let s_corr = -0.1 * pow(poly6(dot(r, r)) / w_dq, 4.0);
            d = d + spiky_grad(r) * (lam_i + lam_j + s_corr);
            dv = dv + (state[j].b.xyz - vel) * (poly6(dot(r, r)) / u.pb.y);
        }
    } } }
    pbd_dat[gid.x * 2u] = vec4<f32>(d / u.pb.y, lam_i);
    pbd_dat[gid.x * 2u + 1u] = vec4<f32>(dv, 0.0);
}
@compute @workgroup_size(64)
fn pbd_apply(@builtin(global_invocation_id) gid: vec3<u32>) {
    let p = pbd_particle(gid.x);
    if (p < 0) { return; }
    var s = state[p];
    var d = pbd_dat[gid.x * 2u].xyz;
    let l = length(d);
    if (l > 0.12) { d = d * (0.12 / l); }
    s.a = vec4<f32>(max(s.a.xyz + d, vec3<f32>(-1.0e6, ground(s.a.x + d.x, s.a.z + d.z) + 0.04, -1.0e6)), s.a.w);
    s.b = vec4<f32>(s.b.xyz + d / max(u.g.x, 0.008) * 0.25 + pbd_dat[gid.x * 2u + 1u].xyz * u.pb.w, s.b.w);
    state[p] = s;
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
    if (i >= u.n.z + u.rs.y || u.bx.w <= 0.0) { return; }
    if (i % 3u != 0u) { return; }   // one particle in three stands for all (the field is statistics): a third of the atomics, three times the mass
    let s = state[i];
    if (s.d.z < 0.5) { return; }
    let b = to_box(s.a.xyz);
    let c = vec3<i32>(floor(b / u.bx.w));
    let n = box_cells();
    if (any(b < vec3<f32>(0.0)) || any(c >= n)) { return; }
    let m = select(u.cp.y, u.cp.z, s.d.x > 0.5 && s.d.x < 1.5) * 3.0;
    let rel = s.b.xyz - u.car_vel.xyz;
    let v = vec3<f32>(dot(rel, right_axis()), rel.y, dot(rel, fwd_axis()));
    let k = dens_index(c);
    atomicAdd(&dens[k], i32(m * 1024.0));
    atomicAdd(&dens[k + 1], i32(m * v.x * 1024.0));
    atomicAdd(&dens[k + 2], i32(m * v.y * 1024.0));
    atomicAdd(&dens[k + 3], i32(m * v.z * 1024.0));
}

fn smooth01(x: f32) -> f32 { let t = clamp(x, 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
// the car's trailing vortices in the car's frame (game/dust_ref.rs::trailing_vortices): c = (across, up from the road, along) from the car's centre
fn trailing_vortices(c: vec3<f32>) -> vec3<f32> {
    let hw = u.body.x; let hl = u.body.y; let h = u.body.z;
    let behind = -c.z - hl;
    if (behind < -1.0) { return vec3<f32>(0.0); }
    let ramp = smooth01((behind + 1.0) / 1.5);
    let b = max(behind, 0.0);
    let v_peak = u.body.w * u.ph2.y * ramp * exp(-b / 11.0);
    let rc = 0.7 + 0.2 * b;
    var out = vec3<f32>(0.0);
    for (var j = 0; j < 2; j++) {
        let s = select(1.0, -1.0, j == 1);
        let rx = c.x - s * hw * 0.8; let ry = c.y - (0.55 * h + 0.15);
        let k = 2.0 * v_peak * rc / (rx * rx + ry * ry + rc * rc);
        out = out + vec3<f32>(-s * ry * k, s * rx * k, 0.0);
    }
    return out;
}
// the recirculation right behind the car's rear face (game/dust_ref.rs::wake_inflow)
fn wake_inflow(c: vec3<f32>) -> vec3<f32> {
    let hw = u.body.x; let hl = u.body.y; let h = u.body.z; let speed = u.ph2.y;
    let behind = -c.z - hl;
    if (behind < -0.5 || behind > 8.0 || speed < 0.5) { return vec3<f32>(0.0); }
    let f = exp(-max(behind, 0.0) / 2.5) * smooth01((behind + 0.5) / 1.0);
    let wide = exp(-pow(c.x / (hw + 1.0), 2.0));
    let tall = exp(-pow(c.y / (h * 1.1), 2.0));
    let m = speed * f;
    return vec3<f32>(-c.x / (hw + 0.5) * 0.6 * m * tall, 0.12 * m * wide * tall, 0.35 * m * wide * tall);
}
// the vortex street behind the car (game/dust_ref.rs::karman_street): vortices with vertical axes shed alternately from the two sides
fn karman_street(c: vec3<f32>) -> vec3<f32> {
    let hw = u.body.x; let hl = u.body.y; let h = u.body.z; let speed = u.ph2.y; let strength = u.cp.w;
    if (speed < 0.5 || c.y > 2.0 * h + 1.0 || strength <= 0.0) { return vec3<f32>(0.0); }
    let period = clamp(2.0 * hw / (0.2 * speed), 0.12, 2.0);
    let n = floor(u.g.y / period);
    let s = u.g.y / period - n;
    let up = 1.0 - smooth01((c.y - h) / (0.8 * h + 0.2));
    var out = vec3<f32>(0.0);
    for (var j = 0; j < 5; j++) {
        let age = (f32(j) + s) * period;
        let z = -hl - 0.6 - 0.8 * speed * age;
        let even = (i32(f32(j) + n) % 2) == 0;
        let x = select(-hw * 0.9, hw * 0.9, even);
        let sign = select(-1.0, 1.0, x > 0.0);
        let fade = clamp(1.0 - age / (5.0 * period), 0.0, 1.0) * smooth01(age / (0.4 * period));
        let rc = 0.6 + 0.12 * speed * age * 0.25;
        let rx = c.x - x; let rz = c.z - z;
        let k = 2.0 * strength * speed * fade * rc / (rx * rx + rz * rz + rc * rc);
        out = out + vec3<f32>(-sign * rz * k, 0.0, sign * rx * k);
    }
    return out * up;
}
// the direction (world) away from the car's body for a particle inside it or close to it (zero in clear air): from the solid cells around it
fn solid_at(b: vec3<f32>) -> bool {
    let n = vec3<i32>(textureDimensions(grid_solid));
    let c = vec3<i32>(floor(b / u.bx.w));
    if (any(c < vec3<i32>(0)) || any(c >= n)) { return false; }
    return textureLoad(grid_solid, c, 0).x > 0.5;
}
// ---- the exact shape of the body: the signed distance field of its mesh
fn mesh_local(p: vec3<f32>) -> vec3<f32> {
    let d = p - u.bpos.xyz;
    return vec3<f32>(dot(d, u.bax0.xyz), dot(d, u.bax1.xyz), dot(d, u.bax2.xyz));
}
fn sdf_cell(c: vec3<i32>) -> f32 {
    let n = vec3<i32>(u.sdf_n.xyz);
    return textureLoad(sdf_tex, clamp(c, vec3<i32>(0), n - vec3<i32>(1)), 0).x;
}
fn sdf_at(pm: vec3<f32>) -> f32 {
    let g = (pm - u.sdf_o.xyz) / u.sdf_o.w - vec3<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let c = vec3<i32>(i);
    let x00 = mix(sdf_cell(c), sdf_cell(c + vec3<i32>(1, 0, 0)), f.x);
    let x10 = mix(sdf_cell(c + vec3<i32>(0, 1, 0)), sdf_cell(c + vec3<i32>(1, 1, 0)), f.x);
    let x01 = mix(sdf_cell(c + vec3<i32>(0, 0, 1)), sdf_cell(c + vec3<i32>(1, 0, 1)), f.x);
    let x11 = mix(sdf_cell(c + vec3<i32>(0, 1, 1)), sdf_cell(c + vec3<i32>(1, 1, 1)), f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}
// the push out of the body for a particle close to the mesh: world direction and how far (0 away from it)
fn sdf_push(p: vec3<f32>) -> vec4<f32> {
    if (u.sdf_n.w < 0.5) { return vec4<f32>(0.0); }
    let pm = mesh_local(p);
    let size = u.sdf_n.xyz * u.sdf_o.w;
    let rel = pm - u.sdf_o.xyz;
    if (any(rel < vec3<f32>(0.0)) || any(rel > size)) { return vec4<f32>(0.0); }
    let d = sdf_at(pm);
    let margin = 0.12;
    if (d >= margin) { return vec4<f32>(0.0); }
    let e = 0.5 * u.sdf_o.w;
    let g = vec3<f32>(sdf_at(pm + vec3<f32>(e, 0.0, 0.0)) - sdf_at(pm - vec3<f32>(e, 0.0, 0.0)),
                      sdf_at(pm + vec3<f32>(0.0, e, 0.0)) - sdf_at(pm - vec3<f32>(0.0, e, 0.0)),
                      sdf_at(pm + vec3<f32>(0.0, 0.0, e)) - sdf_at(pm - vec3<f32>(0.0, 0.0, e)));
    let l = length(g);
    if (l < 1e-5) { return vec4<f32>(0.0, 1.0, 0.0, min(margin - d, 0.4)); }
    let n = g / l;
    return vec4<f32>(u.bax0.xyz * n.x + u.bax1.xyz * n.y + u.bax2.xyz * n.z, min(margin - d, 0.4));
}
fn body_normal(p: vec3<f32>) -> vec3<f32> {
    if (u.bx.w <= 0.0) { return vec3<f32>(0.0); }
    let b = to_box(p);
    let m = 0.3;
    let inside = solid_at(b);
    var n = vec3<f32>(0.0);
    for (var j = 0; j < 6; j++) {
        var d = vec3<f32>(0.0);
        if (j == 0) { d.x = 1.0; } else if (j == 1) { d.x = -1.0; } else if (j == 2) { d.y = 1.0; } else if (j == 3) { d.y = -1.0; } else if (j == 4) { d.z = 1.0; } else { d.z = -1.0; }
        let s = solid_at(b + d * m);
        if (inside) { if (!s) { n = n + d; } } else if (s) { n = n - d; }
    }
    if (inside && dot(n, n) < 0.01) { n = vec3<f32>(0.0, 1.0, 0.0); }
    return right_axis() * n.x + vec3<f32>(0.0, n.y, 0.0) + fwd_axis() * n.z;
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
            let cc = vec3<f32>(b.x - u.bx.x * 0.5, b.y, b.z - u.bx.z * 0.5);
            let vv = trailing_vortices(cc) + karman_street(cc) + wake_inflow(cc);
            air = air + (right_axis() * vv.x + vec3<f32>(0.0, vv.y, 0.0) + fwd_axis() * vv.z) * w;
        }
    }
    // the plume: behind the car the turbulent wake carries the dust up, the higher the less (it hangs at about the plume's height)
    let behind = -dot(p - u.car.xyz, fwd_axis()) - 2.0;
    if (behind > 0.0 && u.ph2.z > 0.0) {
        let h = max(p.y - u.g.z, 0.0);
        air.y = air.y + u.ph2.z * u.ph2.y * exp(-behind / 7.0) * (1.0 - smoothstep(0.5 * u.ph2.w * u.body.z, u.ph2.w * u.body.z, h));   // the wake mixes the dust up to about 1.7 car heights, not beyond
    }
    return air + turbulence(p, age);
}

@compute @workgroup_size(64)
fn dust_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.z + u.rs.y) { return; }
    var s = state[i];
    var dead = DI(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    if (s.d.z < 0.5) { render[i] = dead; return; }
    let dt = u.g.x;
    var pos = s.a.xyz; var vel = s.b.xyz; var age = s.b.w + dt;
    let life = s.c.w;
    if (age >= life) { if ((u.n.w & 8u) != 0u && s.d.x < 1.5) { add_deposit(pos, 64u); } s.d.z = 0.0; state[i] = s; render[i] = dead; return; }
    let coarse = s.d.x > 0.5 && s.d.x < 1.5;
    let smoke = s.d.x > 2.5;
    if (s.d.x > 1.5 && s.d.x < 2.5) {   // a speck of grit: ballistic (gravity, a little drag), bounces and slides on the ground, never part of the dust field
        var sv = vel;
        sv.y = sv.y - 9.8 * dt;
        sv = sv + (u.wind.xyz - sv) * (1.0 - exp(-dt / 2.5));
        var sp = pos + sv * dt;
        let sg = ground(sp.x, sp.z) + 0.02;
        if (sp.y < sg) {
            sp.y = sg;
            if (sv.y < 0.0) { sv.y = -sv.y * 0.35; }
            sv.x = sv.x * 0.6; sv.z = sv.z * 0.6;
        }
        s.a = vec4<f32>(sp, s.a.w); s.b = vec4<f32>(sv, age);
        state[i] = s;
        let sx = clamp(age / life, 0.0, 1.0);
        render[i] = DI(vec4<f32>(sp, s.a.w), vec4<f32>(s.c.xyz, 0.95 * min(sx / 0.05, 1.0) * (1.0 - sx * sx)), vec4<f32>(s.d.y, sx, 2.0, 1.0));
        return;
    }
    let tau = select(u.ph.x, u.ph.y, coarse) * (0.6 + 0.8 * fract(s.d.y * 7.31));   // every grain answers the air a little differently
    var settle = select(u.ph.z, u.ph.w, coarse);
    if (smoke) { settle = -0.45; }   // tyre smoke is warm and fine: it drifts up a little
    var air = air_at(pos, age / life);
    if (any(air != air)) { air = vec3<f32>(0.0); }
    // pushed away from the dust around it; lit by the sun through the dust above it (the cells' light), the dense middle darker
    var light = 1.0;
    var shade = 1.0;
    var push = vec3<f32>(0.0);
    if (u.bx.w > 0.0) {
        let b = to_box(pos);
        let w = box_weight(b);
        if (w > 0.0) {
            let f = fld_at(b);
            push = push_from(f.xyz, w);
            light = mix(1.0, f.w, w);
            shade = mix(1.0, 0.9, smoothstep(0.2, 2.0, mass_at(b)) * w);
        }
    }
    let target_v = vec3<f32>(air.x, air.y - settle, air.z);
    let k = 1.0 - exp(-dt / tau);
    vel = vel + (target_v - vel) * k + push * dt;
    let before = pos;
    pos = pos + vel * dt;
    // the car's body: dust does not go through it or sit in it: out of the body, and no speed into it (the rest slides along it)
    if ((u.n.w & 2u) != 0u && u.sdf_n.w > 0.5) {
        // the exact silhouette: out along the mesh's distance field, the speed into the body lost
        let sp = sdf_push(pos);
        if (sp.w > 0.0) {
            pos = pos + sp.xyz * sp.w;
            vel = vel - sp.xyz * min(dot(vel, sp.xyz), 0.0);
        }
    } else {
        let bn = body_normal(pos);
        if (dot(bn, bn) > 0.0) {
            let nn = normalize(bn);
            pos = pos + nn * 0.08;
            vel = vel - nn * min(dot(vel, nn), 0.0);
        }
    }
    let world_on = (u.n.w & 4u) != 0u;
    if (world_on) { world_collide(&pos, &vel); }
    let g = ground(pos.x, pos.z) + 0.04;
    if (pos.y < g) {
        if (coarse) { if ((u.n.w & 8u) != 0u) { add_deposit(pos, 128u); } s.d.z = 0.0; state[i] = s; render[i] = dead; return; }
        pos.y = g;
        let sl = 1.0 - min(2.5 * dt, 1.0);
        if (world_on) {
            // on the ground the dust slides along the slope, a wall of earth stops it
            let gn = ground_normal(pos.x, pos.z);
            let vn = dot(vel, gn);
            if (vn < 0.0) { vel = vel - gn * vn; }
        } else if (vel.y < 0.0) { vel.y = 0.0; }
        vel.x = vel.x * sl; vel.z = vel.z * sl;
    }
    s.a = vec4<f32>(pos, s.a.w); s.b = vec4<f32>(vel, age);
    state[i] = s;
    // what the sprite needs: it grows as it spreads, fades in over the first 8 % of its life and out at the end
    let x = clamp(age / life, 0.0, 1.0);
    let fade_in = min(x / 0.08, 1.0);
    let size = s.a.w * (1.0 + select(select(0.8, 0.25, coarse), 1.1, smoke) * sqrt(age) + 0.35 * x);   // small grains hardly grow, smoke billows out
    let alpha = select(select(0.38, 0.46, coarse), 0.3, smoke) * fade_in * pow(1.0 - x, select(1.4, 0.8, smoke));
    render[i] = DI(vec4<f32>(pos, size), vec4<f32>(s.c.xyz * shade, alpha), vec4<f32>(s.d.y, x, select(0.0, 1.0, coarse), light));
}

// the counters as floats
@compute @workgroup_size(64)
fn dust_field_mass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = box_cells();
    if (i32(gid.x) >= n.x * n.y * n.z) { return; }
    mass[gid.x] = f32(atomicLoad(&dens[gid.x * 4u])) / 1024.0;
}
// per cell: the mass gradient (central, per cell) and the sun light (6 steps of about one cell towards the sun through the masses)
@compute @workgroup_size(64)
fn dust_field_light(@builtin(global_invocation_id) gid: vec3<u32>) {
    let n = box_cells();
    let id = i32(gid.x);
    if (id >= n.x * n.y * n.z) { return; }
    let c = vec3<i32>(id % n.x, (id / n.x) % n.y, id / (n.x * n.y));
    let g = vec3<f32>(mass_cell(c + vec3<i32>(1, 0, 0)) - mass_cell(c - vec3<i32>(1, 0, 0)), mass_cell(c + vec3<i32>(0, 1, 0)) - mass_cell(c - vec3<i32>(0, 1, 0)), mass_cell(c + vec3<i32>(0, 0, 1)) - mass_cell(c - vec3<i32>(0, 0, 1))) * 0.5;
    let sb = vec3<f32>(dot(u.sun.xyz, right_axis()), u.sun.y, dot(u.sun.xyz, fwd_axis()));
    let step_v = sb / max(max(abs(sb.x), abs(sb.y)), max(abs(sb.z), 0.0001));
    var acc = 0.0;
    for (var j = 1; j <= 6; j++) { acc = acc + mass_cell(vec3<i32>(floor(vec3<f32>(c) + vec3<f32>(0.5) + step_v * f32(j)))); }
    fld[id] = vec4<f32>(g, mix(0.68, 1.0, exp(-u.sun.w * acc * 0.4)));
}
