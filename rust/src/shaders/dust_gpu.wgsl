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
    emits: array<S, 64>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read_write> state: array<S>;
@group(0) @binding(2) var<storage, read_write> render: array<DI>;

@compute @workgroup_size(64)
fn emit(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= u.n.x) { return; }
    state[(u.n.y + i) % u.n.z] = u.emits[i];
}

// the wind: weaker near the ground (the CPU path's `air_velocity` without the wake and the turbulence)
fn air_at(p: vec3<f32>) -> vec3<f32> {
    let h = max(p.y - u.g.z, 0.0);
    return u.wind.xyz * (0.35 + 0.65 * min(h / 4.0, 1.0));
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
    pos = pos + vel * dt;
    let g = u.g.z + 0.04;
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
