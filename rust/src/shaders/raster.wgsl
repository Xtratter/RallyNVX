struct U {
    vp: mat4x4<f32>, light_vp: mat4x4<f32>, inv_vp: mat4x4<f32>,
    cam: vec4<f32>, sun: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, sky_hor: vec4<f32>, sun_col: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var shadow_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_smp: sampler_comparison;
@group(0) @binding(3) var mat_tex: texture_2d_array<f32>;
@group(0) @binding(4) var mat_smp: sampler;

fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

fn sky_gradient(dir: vec3<f32>) -> vec3<f32> {
    let t = clamp(dir.y * 1.6, 0.0, 1.0);
    let c = mix(u.sky_hor.rgb, u.sky_top.rgb, sqrt(t));
    return mix(c, u.sky_hor.rgb * 1.05, (1.0 - smoothstep(0.0, 0.12, dir.y)) * 0.6);
}

fn vnoise2(p: vec2<f32>) -> f32 {
    let i = floor(p); let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash2(i); let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0)); let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

fn fbm2(p0: vec2<f32>) -> f32 {
    var p = p0; var a = 0.5; var s = 0.0;
    for (var i = 0; i < 4; i++) { s += a * vnoise2(p); p = p * 2.03 + vec2<f32>(17.0, 9.0); a *= 0.5; }
    return s;
}

fn finish(c: vec3<f32>) -> vec3<f32> {
    let t = vec3<f32>(1.0) - exp(-c * 1.15);
    return pow(t, vec3<f32>(1.0 / 2.2));
}

fn shadow_at(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    let p = u.light_vp * vec4<f32>(wp + n * 0.2, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let inside = select(0.0, 1.0, uv.x > 0.0 && uv.x < 1.0 && uv.y > 0.0 && uv.y < 1.0 && p.z < 1.0);
    let texel = 1.0 / 2048.0;
    var s = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            s += textureSampleCompareLevel(shadow_tex, shadow_smp, uv + vec2<f32>(f32(i), f32(j)) * texel, p.z - 0.002);
        }
    }
    return mix(1.0, s / 9.0, inside);
}

fn lit(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return shade(albedo, wp, n, 0.0, 0.0);
}

// spec: 0..1 sun glint strength, sharp: glint exponent scale (0 = none)
fn shade(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>, spec: f32, sharp: f32) -> vec3<f32> {
    let sun = normalize(u.sun.xyz);
    let sh = shadow_at(wp, n);
    let diff = max(dot(n, sun), 0.0) * sh;
    let amb = mix(vec3<f32>(0.20, 0.22, 0.26), u.sky_top.rgb * 0.55, n.y * 0.5 + 0.5);
    var c = albedo * (u.sun_col.rgb * diff + amb);
    if (sharp > 0.0) {
        let v = normalize(u.cam.xyz - wp);
        let h = normalize(sun + v);
        c = c + u.sun_col.rgb * 0.4 * spec * pow(max(dot(n, h), 0.0), sharp) * sh * step(0.0, dot(n, sun));
        let fr = pow(1.0 - max(dot(n, v), 0.0), 4.0);
        c = c + sky_gradient(reflect(-v, n)) * fr * spec * 0.5;
    }
    let d = length(wp - u.cam.xyz);
    let f = 1.0 - exp(-d * u.fog.x);
    c = mix(c, u.sky_hor.rgb, clamp(f, 0.0, 1.0));
    return finish(c);
}

struct TOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) w: vec4<f32> };

@vertex fn vs_terrain(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) w: vec4<f32>) -> TOut {
    var o: TOut; o.pos = u.vp * vec4<f32>(p, 1.0); o.wp = p; o.n = n; o.w = w; return o;
}
@vertex fn vs_terrain_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * vec4<f32>(p, 1.0);
}


fn mat_sample(layer: i32, uv: vec2<f32>) -> vec3<f32> {
    return textureSample(mat_tex, mat_smp, uv, layer).rgb;
}

@fragment fn fs_terrain(i: TOut) -> @location(0) vec4<f32> {
    let n = normalize(i.n);
    let uv = i.wp.xz * 0.25;
    let uv2 = i.wp.xz * 0.031 + vec2<f32>(0.37, 0.61);   // second scale breaks up the tiling
    let w = i.w / max(i.w.x + i.w.y + i.w.z + i.w.w, 1e-4);
    var a = mat_sample(0, uv) * w.x + mat_sample(1, uv) * w.y + mat_sample(2, uv) * w.z + mat_sample(3, uv) * w.w;
    let far = mat_sample(0, uv2) * w.x + mat_sample(1, uv2) * w.y + mat_sample(2, uv2) * w.z + mat_sample(3, uv2) * w.w;
    a = a * (0.65 + 0.7 * dot(far, vec3<f32>(0.33)) / max(dot(a, vec3<f32>(0.33)), 0.05) * 0.5);
    a = pow(a, vec3<f32>(1.35));   // the maps are stored linear and bright; this deepens and saturates them
    let snow = smoothstep(34.0, 44.0, i.wp.y) * (1.0 - smoothstep(0.1, 0.3, 1.0 - n.y));
    a = mix(a, vec3<f32>(0.85, 0.86, 0.90), snow);
    return vec4<f32>(lit(a, i.wp, n), 1.0);
}

struct MOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) col: vec3<f32>, @location(3) mat: f32 };

fn model_world(p: vec3<f32>, m0: vec4<f32>, m1: vec4<f32>, m2: vec4<f32>, m3: vec4<f32>) -> vec3<f32> {
    return (mat4x4<f32>(m0, m1, m2, m3) * vec4<f32>(p, 1.0)).xyz;
}

@vertex fn vs_model(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>,
    @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
    @location(7) tint: vec4<f32>) -> MOut {
    var o: MOut;
    var w = model_world(p, m0, m1, m2, m3);
    // foliage (material 6) sways in the wind, more towards the top of the model
    if (i32(c.a * 255.0 + 0.5) == 6) {
        let k = max(p.y, 0.0);
        w.x += sin(u.fog.y * 2.0 + w.x * 0.7 + w.z * 0.5) * 0.06 * k;
        w.z += cos(u.fog.y * 1.7 + w.z * 0.6) * 0.04 * k;
    }
    o.pos = u.vp * vec4<f32>(w, 1.0);
    o.wp = w;
    o.n = normalize(m0.xyz * n.x + m1.xyz * n.y + m2.xyz * n.z);   // uniform scale + rotation
    o.col = c.rgb * tint.rgb;
    o.mat = c.a * 255.0;
    return o;
}
@vertex fn vs_model_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>,
    @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
    @location(7) tint: vec4<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * vec4<f32>(model_world(p, m0, m1, m2, m3), 1.0);
}
@fragment fn fs_model(i: MOut) -> @location(0) vec4<f32> {
    let n = normalize(i.n);
    let m = i32(i.mat + 0.5);
    if (m == 5) { return vec4<f32>(finish(i.col * 2.2), 1.0); }       // lamps
    if (m == 1) { return vec4<f32>(shade(i.col, i.wp, n, 1.2, 90.0), 1.0); }   // glass
    if (m == 4) { return vec4<f32>(shade(i.col, i.wp, n, 0.9, 60.0), 1.0); }   // chrome
    if (m == 6) { return vec4<f32>(shade(i.col, i.wp, n, 0.0, 0.0), 1.0); }   // foliage
    if (m == 0) { return vec4<f32>(shade(i.col, i.wp, n, 0.5, 40.0), 1.0); }   // paint
    if (m == 2) { return vec4<f32>(shade(i.col, i.wp, n, 0.04, 8.0), 1.0); }   // rubber
    return vec4<f32>(shade(i.col, i.wp, n, 0.15, 16.0), 1.0);                  // plastic
}

struct IOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) col: vec3<f32>, @location(3) emi: f32,
    @location(4) ctr: vec3<f32>, @location(5) size: vec3<f32> };

fn inst_world(p: vec3<f32>, m0: vec4<f32>, m1: vec4<f32>, m2: vec4<f32>, m3: vec4<f32>) -> vec3<f32> {
    return (mat4x4<f32>(m0, m1, m2, m3) * vec4<f32>(p, 1.0)).xyz;
}

@vertex fn vs_inst(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>,
    @location(2) m0: vec4<f32>, @location(3) m1: vec4<f32>, @location(4) m2: vec4<f32>, @location(5) m3: vec4<f32>,
    @location(6) sc: vec4<f32>, @location(7) col: vec4<f32>) -> IOut {
    var o: IOut;
    let w = inst_world(p, m0, m1, m2, m3);
    o.pos = u.vp * vec4<f32>(w, 1.0);
    o.wp = w;
    o.n = normalize((m0.xyz / sc.x) * (n.x / sc.x) + (m1.xyz / sc.y) * (n.y / sc.y) + (m2.xyz / sc.z) * (n.z / sc.z));
    o.col = col.rgb;
    o.emi = col.a;
    o.ctr = m3.xyz;
    o.size = sc.xyz;
    return o;
}
@vertex fn vs_inst_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>,
    @location(2) m0: vec4<f32>, @location(3) m1: vec4<f32>, @location(4) m2: vec4<f32>, @location(5) m3: vec4<f32>,
    @location(6) sc: vec4<f32>, @location(7) col: vec4<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * vec4<f32>(inst_world(p, m0, m1, m2, m3), 1.0);
}
@fragment fn fs_inst(i: IOut) -> @location(0) vec4<f32> {
    // colour alpha 4 marks street strips: even asphalt with lane marks (strip start is a crossing centre, pitch 48, street 12)
    if (i.emi > 3.5) {
        let n = normalize(i.n);
        var a = vec3<f32>(0.17, 0.17, 0.19) * (0.92 + 0.16 * hash2(floor(i.wp.xz * 9.0)));
        if (n.y > 0.5) {
            let ax = i.size.x > i.size.z;
            let along = select(i.wp.z - (i.ctr.z - i.size.z * 0.5), i.wp.x - (i.ctr.x - i.size.x * 0.5), ax);
            let lat = select(i.wp.x - i.ctr.x, i.wp.z - i.ctr.z, ax);
            let m = along - floor(along / 48.0) * 48.0;
            let crossing = m < 6.0 || m > 42.0;
            if (!crossing) {
                let dash = step(fract(along / 6.0), 0.5) * step(abs(lat), 0.12);
                let edge = step(abs(abs(lat) - 5.3), 0.1);
                a = mix(a, vec3<f32>(0.85, 0.80, 0.55), dash);
                a = mix(a, vec3<f32>(0.80, 0.80, 0.80), edge);
            }
        }
        return vec4<f32>(lit(a, i.wp, n), 1.0);
    }
    // colour alpha 3 marks buildings: windows on the side faces from the world position
    if (i.emi > 2.5) {
        let n = normalize(i.n);
        var albedo = i.col;
        if (abs(n.y) < 0.5) {
            let t = normalize(vec2<f32>(-n.z, n.x));
            let cell = vec2<f32>(dot(i.wp.xz, t) / 3.0, i.wp.y / 3.5);
            let f = fract(cell);
            let win = step(0.25, f.x) * step(f.x, 0.75) * step(0.3, f.y) * step(f.y, 0.75);
            let v = normalize(u.cam.xyz - i.wp);
            let fr = pow(1.0 - max(dot(n, v), 0.0), 3.0);
            albedo = mix(albedo, vec3<f32>(0.50, 0.68, 0.80) * 0.8 + sky_gradient(reflect(-v, n)) * (0.1 + 0.5 * fr), win * 0.85);
        }
        return vec4<f32>(lit(albedo, i.wp, n), 1.0);
    }
    // colour alpha > 1.5 marks glowing mission markers: unlit and not hidden by fog
    if (i.emi > 1.5) { return vec4<f32>(finish(i.col * 1.6), 1.0); }
    return vec4<f32>(lit(i.col, i.wp, normalize(i.n)), 1.0);
}

struct SOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_sky(@builtin(vertex_index) vi: u32) -> SOut {
    var o: SOut;
    let p = vec2<f32>(f32((vi << 1u) & 2u), f32(vi & 2u));
    o.pos = vec4<f32>(p * 2.0 - 1.0, 1.0, 1.0);
    o.uv = p;
    return o;
}
@fragment fn fs_sky(i: SOut) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(i.uv.x * 2.0 - 1.0, 1.0 - i.uv.y * 2.0, 1.0, 1.0);
    let w = u.inv_vp * ndc;
    let dir = normalize(w.xyz / w.w - u.cam.xyz);
    var c = sky_gradient(dir);
    let sd = normalize(u.sun.xyz);
    let s = max(dot(dir, sd), 0.0);
    // clouds: noise on a plane above the viewer, drifting with the wind
    if (dir.y > 0.02) {
        let p = dir.xz / (dir.y + 0.12) * 0.35 + vec2<f32>(u.fog.y * 0.012, u.fog.y * 0.004);
        let n = fbm2(p);
        let cov = smoothstep(0.50, 0.78, n) * smoothstep(0.02, 0.25, dir.y);
        let lit_side = clamp(0.55 + 0.6 * dot(vec2<f32>(0.0), p) + 0.45 * (1.0 - n), 0.0, 1.0);
        let sun_tint = normalize(u.sun_col.rgb + vec3<f32>(0.001)) * 1.7;
        let cloud = mix(vec3<f32>(0.62, 0.65, 0.72), vec3<f32>(1.0, 0.98, 0.95) * sun_tint, lit_side);
        c = mix(c, cloud, cov * 0.92);
    }
    c = c + u.sun_col.rgb * 0.4 * (pow(s, 600.0) * 8.0 + pow(s, 12.0) * 0.18);
    return vec4<f32>(finish(c), 1.0);
}

struct WOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) depth: f32 };
@vertex fn vs_water(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> WOut {
    var o: WOut; o.pos = u.vp * vec4<f32>(p, 1.0); o.wp = p; o.depth = n.x; return o;
}
@fragment fn fs_water(i: WOut) -> @location(0) vec4<f32> {
    let t = u.fog.y;
    // wave normal from two drifting noise layers
    let e = 0.15;
    let p1 = i.wp.xz * 0.35 + vec2<f32>(t * 0.15, t * 0.08);
    let p2 = i.wp.xz * 0.9 + vec2<f32>(-t * 0.2, t * 0.11);
    let h0 = vnoise2(p1) * 0.6 + vnoise2(p2) * 0.4;
    let hx = vnoise2(p1 + vec2<f32>(e, 0.0)) * 0.6 + vnoise2(p2 + vec2<f32>(e, 0.0)) * 0.4;
    let hz = vnoise2(p1 + vec2<f32>(0.0, e)) * 0.6 + vnoise2(p2 + vec2<f32>(0.0, e)) * 0.4;
    let n = normalize(vec3<f32>(-(hx - h0) * 1.6, 1.0, -(hz - h0) * 1.6));
    let v = normalize(u.cam.xyz - i.wp);
    let d = max(i.depth, 0.0);
    let body = mix(vec3<f32>(0.05, 0.20, 0.22), vec3<f32>(0.01, 0.06, 0.13), smoothstep(0.0, 5.0, d)) * (0.5 + 0.5 * length(u.sun_col.rgb) / 2.6);
    let fr = pow(1.0 - max(dot(n, v), 0.0), 5.0);
    var c = mix(body, sky_gradient(reflect(-v, n)), clamp(0.04 + 0.96 * fr, 0.0, 1.0));
    let sd = normalize(u.sun.xyz);
    c = c + u.sun_col.rgb * 0.4 * pow(max(dot(reflect(-sd, n), v), 0.0), 120.0);
    let f = clamp(1.0 - exp(-length(i.wp - u.cam.xyz) * u.fog.x), 0.0, 1.0);
    c = mix(c, u.sky_hor.rgb, f);
    let alpha = smoothstep(0.0, 0.5, i.depth) * mix(0.35, 0.92, smoothstep(0.0, 4.0, d));
    return vec4<f32>(finish(c), alpha);
}
