struct U {
    vp: mat4x4<f32>, light_vp: mat4x4<f32>, inv_vp: mat4x4<f32>,
    cam: vec4<f32>, sun: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, sky_hor: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var shadow_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_smp: sampler_comparison;
@group(0) @binding(3) var mat_tex: texture_2d_array<f32>;
@group(0) @binding(4) var mat_smp: sampler;

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
    var c = albedo * (vec3<f32>(1.0, 0.93, 0.80) * 2.6 * diff + amb);
    if (sharp > 0.0) {
        let v = normalize(u.cam.xyz - wp);
        let h = normalize(sun + v);
        c = c + vec3<f32>(1.0, 0.95, 0.85) * spec * pow(max(dot(n, h), 0.0), sharp) * sh * step(0.0, dot(n, sun));
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

fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

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
    let w = model_world(p, m0, m1, m2, m3);
    o.pos = u.vp * vec4<f32>(w, 1.0);
    o.wp = w;
    o.n = normalize(m0.xyz * n.x + m1.xyz * n.y + m2.xyz * n.z);   // rigid matrix: rotation only
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
    if (m == 0) { return vec4<f32>(shade(i.col, i.wp, n, 0.5, 40.0), 1.0); }   // paint
    if (m == 2) { return vec4<f32>(shade(i.col, i.wp, n, 0.04, 8.0), 1.0); }   // rubber
    return vec4<f32>(shade(i.col, i.wp, n, 0.15, 16.0), 1.0);                  // plastic
}

struct IOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) col: vec3<f32>, @location(3) emi: f32 };

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
    return o;
}
@vertex fn vs_inst_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>,
    @location(2) m0: vec4<f32>, @location(3) m1: vec4<f32>, @location(4) m2: vec4<f32>, @location(5) m3: vec4<f32>,
    @location(6) sc: vec4<f32>, @location(7) col: vec4<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * vec4<f32>(inst_world(p, m0, m1, m2, m3), 1.0);
}
@fragment fn fs_inst(i: IOut) -> @location(0) vec4<f32> {
    // colour alpha 3 marks buildings: windows on the side faces from the world position
    if (i.emi > 2.5) {
        let n = normalize(i.n);
        var albedo = i.col;
        if (abs(n.y) < 0.5) {
            let t = normalize(vec2<f32>(-n.z, n.x));
            let cell = vec2<f32>(dot(i.wp.xz, t) / 3.0, i.wp.y / 3.5);
            let f = fract(cell);
            let win = step(0.25, f.x) * step(f.x, 0.75) * step(0.3, f.y) * step(f.y, 0.75);
            albedo = mix(albedo, vec3<f32>(0.50, 0.68, 0.80), win * 0.85);
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
    let t = clamp(dir.y * 1.6, 0.0, 1.0);
    var c = mix(u.sky_hor.rgb, u.sky_top.rgb, sqrt(t));
    c = mix(c, u.sky_hor.rgb * 1.05, (1.0 - smoothstep(0.0, 0.12, dir.y)) * 0.6);
    let s = max(dot(dir, normalize(u.sun.xyz)), 0.0);
    c = c + vec3<f32>(1.0, 0.85, 0.6) * (pow(s, 600.0) * 8.0 + pow(s, 12.0) * 0.18);
    return vec4<f32>(finish(c), 1.0);
}
