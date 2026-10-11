struct U {
    vp: mat4x4<f32>, light_vp: mat4x4<f32>, inv_vp: mat4x4<f32>,
    cam: vec4<f32>, sun: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, sky_hor: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var shadow_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_smp: sampler_comparison;

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
    let sun = normalize(u.sun.xyz);
    let diff = max(dot(n, sun), 0.0) * shadow_at(wp, n);
    let amb = mix(vec3<f32>(0.20, 0.22, 0.26), u.sky_top.rgb * 0.55, n.y * 0.5 + 0.5);
    var c = albedo * (vec3<f32>(1.0, 0.93, 0.80) * 2.6 * diff + amb);
    let d = length(wp - u.cam.xyz);
    let f = 1.0 - exp(-d * u.fog.x);
    c = mix(c, u.sky_hor.rgb, clamp(f, 0.0, 1.0));
    return finish(c);
}

struct TOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32> };

@vertex fn vs_terrain(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> TOut {
    var o: TOut; o.pos = u.vp * vec4<f32>(p, 1.0); o.wp = p; o.n = n; return o;
}
@vertex fn vs_terrain_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.light_vp * vec4<f32>(p, 1.0);
}

fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

@fragment fn fs_terrain(i: TOut) -> @location(0) vec4<f32> {
    let n = normalize(i.n);
    let slope = 1.0 - n.y;
    let grass = vec3<f32>(0.20, 0.34, 0.12);
    let dirt = vec3<f32>(0.40, 0.31, 0.20);
    let rock = vec3<f32>(0.42, 0.40, 0.38);
    var a = mix(grass, dirt, smoothstep(6.0, 22.0, i.wp.y));
    a = mix(a, rock, smoothstep(0.12, 0.30, slope));
    a = mix(a, vec3<f32>(0.85, 0.86, 0.90), smoothstep(34.0, 44.0, i.wp.y) * (1.0 - smoothstep(0.1, 0.3, slope)));
    a = a * (0.88 + 0.24 * hash2(floor(i.wp.xz * 0.7)));
    return vec4<f32>(lit(a, i.wp, n), 1.0);
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
    let s = max(dot(dir, normalize(u.sun.xyz)), 0.0);
    c = c + vec3<f32>(1.0, 0.85, 0.6) * (pow(s, 600.0) * 8.0 + pow(s, 12.0) * 0.18);
    return vec4<f32>(finish(c), 1.0);
}
