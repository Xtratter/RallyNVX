struct U {
    vp: mat4x4<f32>, light_vp: mat4x4<f32>, inv_vp: mat4x4<f32>,
    cam: vec4<f32>, sun: vec4<f32>, fog: vec4<f32>, sky_top: vec4<f32>, sky_hor: vec4<f32>, sun_col: vec4<f32>,
    mode: vec4<f32>, cam_tile: vec4<f32>, moon: vec4<f32>, disc: vec4<f32>, hl_pos: vec4<f32>, hl_dir: vec4<f32>, tl_pos: vec4<f32>, tl_col: vec4<f32>, wind: vec4<f32>, lamps: vec4<f32>, rt: vec4<f32>, susp: vec4<f32>, wx: vec4<f32>, cr: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var shadow_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_smp: sampler_comparison;
@group(0) @binding(5) var atlas: texture_2d<f32>;
@group(0) @binding(6) var atlas_smp: sampler;   // height atlas (R16Float, 8x8 tiles of 64x64 texels, 2 m per texel)
@group(0) @binding(8) var livery_tex: texture_2d<f32>;   // the rally car's livery atlas (sRGB), sampled with the vertex's UV
@group(0) @binding(9) var livery_smp: sampler;
@group(0) @binding(10) var interior_tex: texture_2d<f32>;   // the interior atlas (the cabin and the driver, sRGB)
@group(0) @binding(11) var interior_smp: sampler;
@group(0) @binding(12) var normal_tex: texture_2d<f32>;   // the normal-map atlas (linear, half the livery's size, the same layout)
@group(0) @binding(7) var dep_tex: texture_2d<f32>;   // the settled dust layer of the ground window (R = amount), 64 x 64 cells of 1 m
@group(0) @binding(3) var mat_tex: texture_2d_array<f32>;
@group(0) @binding(4) var mat_smp: sampler;

// integer hash (PCG3D): no sin() precision problems on mobile GPUs
fn hash3(p: vec3<f32>) -> f32 {
    var v = bitcast<vec3<u32>>(vec3<i32>(floor(p))) * 1664525u + vec3<u32>(1013904223u);
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    v = v ^ (v >> vec3<u32>(16u));
    v.x += v.y * v.z; v.y += v.z * v.x; v.z += v.x * v.y;
    return f32(v.x >> 8u) / 16777216.0;
}
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453); }

fn sky_gradient(dir: vec3<f32>) -> vec3<f32> {
    let t = clamp(dir.y * 1.6, 0.0, 1.0);
    let c = mix(u.sky_hor.rgb, u.sky_top.rgb, sqrt(t));
    return mix(c, u.sky_hor.rgb * 1.05, (1.0 - smoothstep(0.0, 0.12, dir.y)) * 0.6);
}

// Henyey-Greenstein phase function (normalised to 1/4pi * 4pi = 1 over the sphere average ~ 1/(4 pi)): `g` the anisotropy
fn hg(cs: f32, g: f32) -> f32 { let g2 = g * g; return (1.0 - g2) / pow(max(1.0 + g2 - 2.0 * g * cs, 1e-4), 1.5) * 0.0795775; }

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

const POISSON = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216), vec2<f32>(0.94558609, -0.76890725), vec2<f32>(-0.09418410, -0.92938870), vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432), vec2<f32>(-0.81544232, -0.87912464), vec2<f32>(-0.38277543, 0.27676845), vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554), vec2<f32>(0.53742981, -0.47373420), vec2<f32>(-0.26496911, -0.41893023), vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507), vec2<f32>(-0.81409955, 0.91437590), vec2<f32>(0.19984126, 0.78641367), vec2<f32>(0.14383161, -0.14100790));

fn shadow_basic(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    let p = u.light_vp * vec4<f32>(wp + n * 0.2, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let inside = select(0.0, 1.0, uv.x > 0.0 && uv.x < 1.0 && uv.y > 0.0 && uv.y < 1.0 && p.z < 1.0);
    var s = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            s += textureSampleCompareLevel(shadow_tex, shadow_smp, uv + vec2<f32>(f32(i), f32(j)) * (1.0 / 2048.0), p.z - 0.002);
        }
    }
    return mix(1.0, s / 9.0, inside);
}

fn shadow_at(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    let p = u.light_vp * vec4<f32>(wp + n * 0.2, 1.0);
    let uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    let inside = select(0.0, 1.0, uv.x > 0.0 && uv.x < 1.0 && uv.y > 0.0 && uv.y < 1.0 && p.z < 1.0);
    let texel = 1.0 / 2048.0;
    if (u.mode.x < 0.5) {
        var s = 0.0;
        for (var j = -1; j <= 1; j++) {
            for (var i = -1; i <= 1; i++) {
                s += textureSampleCompareLevel(shadow_tex, shadow_smp, uv + vec2<f32>(f32(i), f32(j)) * texel, p.z - 0.002);
            }
        }
        return mix(1.0, s / 9.0, inside);
    }
    let q = i32(u.cam_tile.w + 0.5);
    if (q >= 3) { return 1.0; }                              // shadows off
    if (q == 2) { return textureSampleCompareLevel(shadow_tex, shadow_smp, uv, p.z - 0.002); }   // low: one hardware-filtered tap
    if (q == 1) {   // medium: six taps of the Poisson disc, turned per pixel (interleaved gradient noise), a fixed soft edge
        let ang = 6.2831853 * fract(52.9829189 * fract(dot(vec2<f32>(0.06711056, 0.00583715), floor(uv * 2048.0))));
        let cs = vec2<f32>(cos(ang), sin(ang));
        var m = textureSampleCompareLevel(shadow_tex, shadow_smp, uv, p.z - 0.003);
        for (var k = 0; k < 5; k++) {
            let o = POISSON[k * 3];
            m += textureSampleCompareLevel(shadow_tex, shadow_smp, uv + vec2<f32>(o.x * cs.x - o.y * cs.y, o.x * cs.y + o.y * cs.x) * (3.0 * texel), p.z - 0.003);
        }
        return m / 6.0;
    }
    // hybrid: percentage-closer soft shadows (blocker search, penumbra from the blocker distance, 12-tap PCF)
    if (inside < 0.5) { return 1.0; }
    let sun_n = normalize(u.sun.xyz);
    let zr = p.z - (0.0015 + 0.006 * (1.0 - clamp(dot(n, sun_n), 0.0, 1.0)));   // slope-scaled bias
    if (dot(n, sun_n) <= 0.0) { return 1.0; }                // facing away from the sun: the diffuse term is zero whatever the shadow says
    // beyond 45 m the filter is cut to a little over half the taps (the penumbra is hardly resolved there anyway)
    let far = distance(wp, u.cam.xyz) > 45.0;
    var bsum = 0.0; var bcnt = 0.0;
    let maxpix = 12.0;                                       // search radius in texels = the largest filter radius
    for (var k = 0; k < select(9, 5, far); k++) {
        let i = select(k, select(k * 2, 8, k == 4), far);
        let o = select(POISSON[i] * (maxpix * texel), vec2<f32>(0.0), i == 8);   // 8 spread taps plus the centre
        let q = clamp(uv + o, vec2<f32>(0.0), vec2<f32>(0.9995));
        let d = textureLoad(shadow_tex, vec2<i32>(q * 2048.0), 0);
        if (d < zr) { bsum += d; bcnt += 1.0; }
    }
    if (bcnt < 0.5) { return 1.0; }
    let dist_m = (zr - bsum / bcnt) * 299.0;                 // ortho light, near 1 far 300
    let pen_m = clamp(dist_m * 0.028, 0.04, 0.8);
    let rad = pen_m / 0.0684 * texel;                        // shadow map texel = 140 / 2048 m (at most 12 texels)
    var s = textureSampleCompareLevel(shadow_tex, shadow_smp, uv, zr);
    let np = select(11, 5, far);
    for (var k = 0; k < np; k++) { s += textureSampleCompareLevel(shadow_tex, shadow_smp, uv + POISSON[select(k, k * 2, far)] * rad, zr); }
    return s / (1.0 + f32(np));
}

// terrain height from the atlas (metres); far below everything where no tile is loaded
fn terrain_h(x: f32, z: f32) -> f32 {
    let tx = floor(x / 128.0); let tz = floor(z / 128.0);
    if (abs(tx - u.cam_tile.x) > 3.0 || abs(tz - u.cam_tile.y) > 3.0) { return -10000.0; }
    return textureSampleLevel(atlas, atlas_smp, vec2<f32>(x + 1.0, z + 1.0) / 1024.0, 0.0).r;
}

// soft ray shadow of the terrain towards the sun (20 growing steps, penumbra widens with distance)
fn terrain_shadow(wp: vec3<f32>, sun: vec3<f32>) -> f32 {
    let l = length(sun.xz);
    if (l < 1.0e-3) { return 1.0; }
    let dir = sun.xz / l;
    let slope = sun.y / l;
    // a high sun or a far, hazy fragment: the hills cannot throw a visible shadow here
    let far_fade = 1.0 - smoothstep(200.0, 280.0, distance(wp, u.cam.xyz));
    if (slope > 1.4 || far_fade <= 0.0) { return 1.0; }
    var t = 2.0;
    var vis = 1.0;
    for (var i = 0; i < 14; i++) {
        let ray_h = wp.y + 0.4 + slope * t;
        if (ray_h > 160.0) { break; }
        let th = terrain_h(wp.x + dir.x * t, wp.z + dir.y * t);
        vis = min(vis, clamp((ray_h - th) / (0.06 * t + 0.5), 0.0, 1.0));
        t = t * 1.34 + 1.5;
    }
    return mix(1.0, vis, far_fade);
}

fn lit(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return shade(albedo, wp, n, 0.92, 0.0);
}

// Screen-space bump mapping (Mikkelsen): tilts `n` by the screen-space gradient of a height `h` (metres of relief), no tangents needed. Must run in uniform control flow.
fn bump(n: vec3<f32>, wp: vec3<f32>, h: f32) -> vec3<f32> {
    let dpx = dpdx(wp); let dpy = dpdy(wp);
    let r1 = cross(dpy, n); let r2 = cross(n, dpx);
    let det = dot(dpx, r1);
    let grad = sign(det) * (dpdx(h) * r1 + dpdy(h) * r2);
    return normalize(abs(det) * n - grad);
}

// roughness 0 (mirror) .. 1 (matte), metalness 0 (dielectric) .. 1 (metal)
fn shade(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>, rough: f32, metal: f32) -> vec3<f32> {
    let sun = normalize(u.sun.xyz);
    var sh = shadow_at(wp, n);
    if (u.mode.x > 0.5) { sh = min(sh, mix(1.0, terrain_shadow(wp, sun), smoothstep(0.0, 0.2, dot(n, sun)))); }
    return shade_with(albedo, wp, n, rough, metal, sh);
}

// Paint under a clear coat: the base (`n`, perhaps bump-mapped, a satin `rough`) lit as usual and dimmed by what the coat reflects, plus the coat's own thin glossy layer
// (dielectric F0 0.04, roughness 0.07) on the smooth surface normal `nc`: a sharp sun glint and a crisp reflection of the sky over the paint's softer one.
fn shade_coat(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>, nc: vec3<f32>, rough: f32) -> vec3<f32> {
    let sun = normalize(u.sun.xyz);
    var sh = shadow_at(wp, n);
    if (u.mode.x > 0.5) { sh = min(sh, mix(1.0, terrain_shadow(wp, sun), smoothstep(0.0, 0.2, dot(n, sun)))); }
    let v = normalize(u.cam.xyz - wp);
    let nv = max(dot(nc, v), 0.02);
    let f0 = vec3<f32>(0.04);
    let fc = fresnel(f0, nv);                               // what the coat reflects at this angle: the base below gets the rest
    var c = shade_with(albedo, wp, n, rough, 0.0, sh) * (vec3<f32>(1.0) - fc);
    c = c + u.sun_col.rgb * ggx_spec(nc, v, sun, f0, 0.005) * sh;
    c = c + sky_env(reflect(-v, nc), 0.07) * env_brdf(f0, 0.07, nv) * mix(0.35, 1.0, nc.y * 0.5 + 0.5);
    return c;
}

// cheap variant for foliage tufts (lots of overdraw): the plain 3x3 shadow filter in every mode
fn shade_lite(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return shade_with(albedo, wp, n, 0.95, 0.0, shadow_basic(wp, n));
}

// how much the tail lights light a point (0..1): only what lies behind the rear bumper, never under the car; mirrored in `carlights::tail_gain`
fn taillight(wp: vec3<f32>) -> f32 {
    if (u.tl_col.w <= 0.0) { return 0.0; }
    let dv = wp - u.tl_pos.xyz;
    let behind = smoothstep(0.3, 1.0, dot(dv, -normalize(u.hl_dir.xyz)));
    let att = clamp(1.0 - length(dv) / u.tl_col.w, 0.0, 1.0);
    return behind * att * att;
}

// set by the car model's fragment shader: the car is not lit by its own lamps (the beam on its bumper, the red light under it)
var<private> own_surface: f32 = 0.0;

fn beam_one(lamp: vec3<f32>, fwd: vec3<f32>, right: vec3<f32>, p: vec3<f32>) -> f32 {
    let d = p - lamp;
    let dist = length(d);
    if (dist < 0.05) { return 0.0; }
    let l = d / dist;
    let z = dot(l, fwd);
    if (z <= 0.05) { return 0.0; }
    // cheap rejection before the arctangents: outside about 40 degrees sideways or above the highest cut-off (about 8 degrees up) nothing is lit
    let lx = dot(l, right);
    let ly = dot(l, cross(right, fwd));
    if (abs(lx) > 0.85 * z || ly > 0.16 * z) { return 0.0; }
    let h = atan(lx / z);
    let v = atan(ly / z);
    let cut = 0.07 * smoothstep(0.0, 0.25, h);
    let gv = (1.0 - smoothstep(cut, cut + 0.02, v)) * smoothstep(-0.28, -0.10, v);   // and below the axis the light fades out: nothing under the bumper
    let gh = 1.0 - smoothstep(0.35, 0.65, abs(h));
    let hot = 0.6 + 0.4 * exp(-(h / 0.2) * (h / 0.2));
    let att = 1.0 / (1.0 + (dist / 18.0) * (dist / 18.0));
    return gv * gh * hot * att * clamp(1.0 - dist / 90.0, 0.0, 1.0) * smoothstep(0.6, 2.0, dist);
}
// whether the light of the headlights reaches a world point: the terrain between the lamps and the point does not block the ray (4 samples of the height atlas along it); a hill or a
// crest between the car and the point keeps the beam off what lies behind it, so only what is really in front of the lamps is lit
fn beam_visible(wp: vec3<f32>) -> f32 {
    let src = u.hl_pos.xyz;
    var vis = 1.0;
    for (var k = 1; k <= 4; k++) {
        let q = mix(src, wp, f32(k) / 5.0);
        let h = terrain_h(q.x, q.z);
        vis = min(vis, smoothstep(-0.25, 0.05, q.y - h));   // under the ground by more than 25 cm: blocked; above it: clear
    }
    return vis;
}

// the low beam of both headlights at a world point (0..1), mirrored in `carlights::beam`; the lamps sit `lamps.w` either side of `hl_pos` across the axis
fn headbeam_raw(wp: vec3<f32>) -> f32 {
    if (u.hl_pos.w <= 0.0) { return 0.0; }
    let f = normalize(u.hl_dir.xyz);
    let r = normalize(u.rt.xyz);   // the car's own right: the beam rolls and pitches with the body
    let s = r * u.lamps.w;
    return 0.5 * (beam_one(u.hl_pos.xyz - s, f, r, wp) + beam_one(u.hl_pos.xyz + s, f, r, wp));
}
// the same, blocked by the terrain
fn headbeam(wp: vec3<f32>) -> f32 {
    let g = headbeam_raw(wp);
    if (g < 0.01) { return 0.0; }
    return g * beam_visible(wp);
}

// the light of the lit lamps on the car's own body: a thin soft rim of light on the bodywork right around each lamp unit (small: the car must not look lit by its own lamps) (no billboard, so nothing shows through the car)
fn lamp_aureole(wp: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    let f = normalize(u.hl_dir.xyz);
    let r = normalize(u.rt.xyz);
    var g = vec3<f32>(0.0);
    if (u.hl_pos.w > 0.0) {
        for (var k = 0; k < 2; k++) {
            let c = u.hl_pos.xyz + r * (u.lamps.w * select(-1.0, 1.0, k == 1));
            let d = length(wp - c);
            g = g + vec3<f32>(1.0, 0.93, 0.80) * (0.22 * exp(-d * d / 0.016)) * (0.4 + 0.6 * max(dot(n, f), 0.0));
        }
    }
    if (u.tl_col.w > 0.0) {
        for (var k = 0; k < 2; k++) {
            let c = u.tl_pos.xyz + vec3<f32>(0.0, 0.3, 0.0) + r * (u.tl_pos.w * select(-1.0, 1.0, k == 1));
            let d = length(wp - c);
            g = g + u.tl_col.rgb * (0.35 * exp(-d * d / 0.016)) * (0.4 + 0.6 * max(dot(n, -f), 0.0));
        }
    }
    return g;
}

// ---- physically based shading (Cook-Torrance: GGX distribution, Smith visibility, Schlick Fresnel; the host-tested twins are in game/pbr.rs) ----
fn ggx_d(nh: f32, a: f32) -> f32 {
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / max(3.14159265 * d * d, 1e-9);
}
// Smith-GGX height-correlated visibility, already divided by 4 n.l n.v
fn smith_vis(nl: f32, nv: f32, a: f32) -> f32 {
    let a2 = a * a;
    let gv = nl * sqrt(nv * nv * (1.0 - a2) + a2);
    let gl = nv * sqrt(nl * nl * (1.0 - a2) + a2);
    return 0.5 / (gv + gl + 1e-5);
}
fn fresnel(f0: vec3<f32>, vh: f32) -> vec3<f32> { return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vh, 5.0); }
// the split-sum environment term (Karis' analytic fit): scale and bias of F0 for the ambient specular
fn env_brdf(f0: vec3<f32>, rough: f32, nv: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = rough * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * nv)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + vec3<f32>(ab.y);
}
// the GGX specular lobe of one light: `l` towards the light, radiance in the file's usual units (diffuse is albedo * radiance * n.l)
fn ggx_spec(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, f0: vec3<f32>, a: f32) -> vec3<f32> {
    let nl = dot(n, l);
    if (nl <= 0.0) { return vec3<f32>(0.0); }
    let h = normalize(l + v);
    let nv = max(dot(n, v), 0.02);
    return fresnel(f0, max(dot(v, h), 0.0)) * (ggx_d(max(dot(n, h), 0.0), a) * smith_vis(nl, nv, a) * nl * 3.14159265);
}
// the light of the sky on a surface of a given roughness seen along `r`: the gradient, blurred towards the sky's mean for a rough surface; below the horizon the dim ground
fn sky_env(r: vec3<f32>, rough: f32) -> vec3<f32> {
    let sharp_c = sky_gradient(r);
    let mean_c = mix(u.sky_hor.rgb, u.sky_top.rgb, 0.45);
    var c = mix(sharp_c, mean_c, clamp(rough * rough * 1.6, 0.0, 1.0));
    let ground = vec3<f32>(0.20, 0.22, 0.26) * mix(1.0, 0.12, u.disc.w) + u.sky_hor.rgb * 0.12;
    return mix(c, ground, smoothstep(0.0, -0.25, r.y));
}

// The one lighting function of the world: albedo (linear), the point, the normal, the surface's roughness (0 mirror .. 1 matte) and metalness, the sun's shadow factor.
fn shade_with(albedo: vec3<f32>, wp: vec3<f32>, n: vec3<f32>, rough_in: f32, metal: f32, sh: f32) -> vec3<f32> {
    let sun = normalize(u.sun.xyz);
    let v = normalize(u.cam.xyz - wp);
    let nv = max(dot(n, v), 0.02);
    // wet ground (upward-facing surfaces, not the car): darker and smooth, the water film shows the sky and the lamps
    let wetk = u.wx.y * smoothstep(0.55, 0.9, n.y) * (1.0 - own_surface);
    let rough = clamp(mix(rough_in, 0.22, wetk), 0.05, 1.0);
    let al = albedo * (1.0 - 0.32 * wetk);
    let f0 = mix(vec3<f32>(0.04), al, metal);
    let dcol = al * (1.0 - metal);
    let a = rough * rough;
    let nl = max(dot(n, sun), 0.0);
    let amb = mix(vec3<f32>(0.20, 0.22, 0.26) * mix(1.0, 0.12, u.disc.w), u.sky_top.rgb * 0.55, n.y * 0.5 + 0.5);
    var c = dcol * (u.sun_col.rgb * (nl * sh) + amb);
    let glossy = rough < 0.88 || metal > 0.0;
    if (glossy) {
        // the sun's highlight and the sky's reflection (the ambient specular: the environment times the split-sum term; the downward-facing parts see less of the sky)
        c = c + u.sun_col.rgb * ggx_spec(n, v, sun, f0, max(a, 0.002)) * sh;
        c = c + sky_env(reflect(-v, n), rough) * env_brdf(f0, rough, nv) * mix(0.35, 1.0, n.y * 0.5 + 0.5);
    }
    if (u.hl_pos.w > 0.0 && own_surface < 0.5) {
        let l = normalize(wp - u.hl_pos.xyz);
        let g = headbeam(wp);
        if (g > 0.0) {
            // a beam on a grazing surface (the ground far ahead) is weak by n.l alone: keep a floor so the beam reaches far
            let lamp_rad = vec3<f32>(1.0, 0.93, 0.80) * (14.0 * g);
            c = c + dcol * lamp_rad * (0.25 + 0.75 * max(dot(n, -l), 0.0));
            if (glossy) { c = c + lamp_rad * ggx_spec(n, v, -l, f0, max(a, 0.01)) * 0.5; }
        }
    }
    if (u.tl_col.w > 0.0 && own_surface < 0.5) {
        let dv = wp - u.tl_pos.xyz;
        let dist = length(dv);
        // only what lies behind the rear bumper is lit by the tail lights, never the ground under the car (mirrored in `carlights::tail_gain`)
        let behind = smoothstep(0.3, 1.0, dot(dv, -normalize(u.hl_dir.xyz)));
        let att = clamp(1.0 - dist / u.tl_col.w, 0.0, 1.0);
        c = c + dcol * u.tl_col.rgb * (behind * att * att * (0.35 + 0.65 * max(dot(n, -dv / max(dist, 0.001)), 0.0)) * 4.0);
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

// how much dust has settled at a ground position: bilinear over the window's cells (the window's first cell is in mode.yz; 0 when the option is off)
fn dust_layer(wp: vec3<f32>) -> f32 {
    if (u.mode.w < 0.5) { return 0.0; }
    let g = wp.xz - vec2<f32>(0.5);
    let i = floor(g);
    let f = g - i;
    let b = u.mode.yz;
    var s = 0.0;
    for (var k = 0; k < 4; k++) {
        let o = vec2<f32>(f32(k & 1), f32(k >> 1));
        let c = i + o;
        if (any(c < b) || any(c >= b + vec2<f32>(64.0))) { continue; }
        let ci = vec2<i32>(c);
        let t = textureLoad(dep_tex, vec2<i32>(((ci.x % 64) + 64) % 64, ((ci.y % 64) + 64) % 64), 0).x;
        s = s + t * select(1.0 - f.x, f.x, (k & 1) == 1) * select(1.0 - f.y, f.y, (k >> 1) == 1);
    }
    return s;
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
    let conc_fade = 1.0 - smoothstep(180.0, 260.0, distance(i.wp, u.cam.xyz));
    if (u.mode.x > 0.5 && conc_fade > 0.0) {
        // hybrid: hollows and valleys are darker (concavity from the height atlas, 4 taps at 3 m)
        let hn = (terrain_h(i.wp.x + 3.0, i.wp.z) + terrain_h(i.wp.x - 3.0, i.wp.z) + terrain_h(i.wp.x, i.wp.z + 3.0) + terrain_h(i.wp.x, i.wp.z - 3.0)) * 0.25;
        a = a * (1.0 - clamp((hn - i.wp.y) * 0.22, 0.0, 0.45) * conc_fade);
    }
    a = mix(a, vec3<f32>(0.72, 0.58, 0.42), 0.5 * dust_layer(i.wp));   // settled dust
    // micro-relief: the ground texture's brightness read as a height (3 cm swing), fading out with distance (it would shimmer) and under snow
    let near = 1.0 - smoothstep(25.0, 70.0, distance(i.wp, u.cam.xyz));
    let nb = bump(n, i.wp, dot(a, vec3<f32>(0.33)) * 0.03 * near * (1.0 - snow));
    return vec4<f32>(lit(a, i.wp, nb), 1.0);
}

// foliage (material 6) bends downwind, more towards the top, with gust waves that travel with the wind and a small flutter
fn sway(w: vec3<f32>, k_world: f32) -> vec3<f32> {
    let wd = u.wind.xy;
    let sp = length(wd);
    let dir = wd / max(sp, 0.001);
    let side = vec2<f32>(-dir.y, dir.x);
    let strength = 0.4 + 0.3 * sp;
    let ph = u.fog.y * 1.7 - dot(w.xz, dir) * 0.16;
    let lean = strength * (0.04 * k_world + 0.004 * k_world * k_world);
    let wave = (0.55 * sin(ph) + 0.30 * sin(ph * 2.3 + w.z * 0.2) + 0.15 * sin(ph * 4.1)) * 0.35;
    let flutter = sin(ph * 1.9 + dot(w.xz, side) * 0.5) * 0.12;
    let off = dir * (lean * (1.0 + wave)) + side * (lean * flutter);
    return vec3<f32>(w.x + off.x, w.y, w.z + off.y);
}

struct MOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) col: vec3<f32>, @location(3) mat: f32, @location(4) ao: f32, @location(5) own: f32, @location(6) uv: vec2<f32>, @location(7) heat: f32, @location(8) lp: vec3<f32> };

fn model_world(p: vec3<f32>, m0: vec4<f32>, m1: vec4<f32>, m2: vec4<f32>, m3: vec4<f32>) -> vec3<f32> {
    return (mat4x4<f32>(m0, m1, m2, m3) * vec4<f32>(p, 1.0)).xyz;
}

@vertex fn vs_model(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>,
    @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
    @location(7) tint: vec4<f32>, @location(8) uv: vec2<f32>) -> MOut {
    var o: MOut;
    var pm = p;
    // a suspension part (material 14): lifted with its hub, by the corner's offset times the vertex's weight (red = weight, green = corner)
    if (i32(c.a * 255.0 + 0.5) == 14) {
        let corner = clamp(i32(c.g * 255.0 + 0.5), 0, 3);
        pm.y = pm.y + u.susp[corner] * c.r;
    }
    var w = model_world(pm, m0, m1, m2, m3);
    // foliage (material 6) sways in the wind, more towards the top of the model
    if (i32(c.a * 255.0 + 0.5) == 6) {
        w = sway(w, max(p.y, 0.0) * length(m1.xyz));
    }
    o.pos = u.vp * vec4<f32>(w, 1.0);
    o.wp = w;
    o.n = normalize(m0.xyz * n.x + m1.xyz * n.y + m2.xyz * n.z);   // uniform scale + rotation
    o.col = c.rgb * tint.rgb;
    o.mat = c.a * 255.0;
    // hybrid occlusion near the ground: tint.a 2 = plants (base at the origin), 3 = rocks (base 0.7 below the origin)
    let base = select(0.0, -0.4, tint.a > 2.5);
    let hgt = (p.y - base) * length(m1.xyz);
    o.ao = select(1.0, mix(0.55, 1.0, smoothstep(0.0, 1.8, hgt)), tint.a > 1.5);
    o.uv = uv;
    o.lp = p;
    o.heat = clamp((tint.a - 0.1) / 0.4, 0.0, 1.0);   // a brake disc's tint alpha carries its heat (0.1 .. 0.5)
    o.own = select(0.0, 1.0, tint.a < 0.75);   // tint.a 0.5 = the player's car: its lamps do not light it
    return o;
}
@vertex fn vs_model_shadow(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) c: vec4<f32>,
    @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
    @location(7) tint: vec4<f32>) -> @builtin(position) vec4<f32> {
    var w = model_world(p, m0, m1, m2, m3);
    if (i32(c.a * 255.0 + 0.5) == 6) {   // same sway as vs_model
        w = sway(w, max(p.y, 0.0) * length(m1.xyz));
    }
    return u.light_vp * vec4<f32>(w, 1.0);
}
// The normal `n` tilted by a tangent-space normal map sampled at `uv` (cotangent frame from screen-space derivatives, so the mesh needs no tangents; Schueler's formulation).
// Must run in uniform control flow. `k` scales the relief (0 = flat).
fn map_normal(n: vec3<f32>, wp: vec3<f32>, uv: vec2<f32>, k: f32) -> vec3<f32> {
    let tn = textureSample(normal_tex, livery_smp, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0))).xyz * 2.0 - 1.0;
    let dp1 = dpdx(wp); let dp2 = dpdy(wp);
    let du1 = dpdx(uv); let du2 = dpdy(uv);
    let dp2perp = cross(dp2, n); let dp1perp = cross(n, dp1);
    let t = dp2perp * du1.x + dp1perp * du2.x;
    let b = dp2perp * du1.y + dp1perp * du2.y;
    let inv = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-20));
    // glTF normal maps have green up; the atlas's v runs down the image
    let m = vec3<f32>(tn.x * k, -tn.y * k, max(tn.z, 0.2));
    return normalize(t * inv * m.x + b * inv * m.y + n * m.z);
}

@fragment fn fs_model(i: MOut) -> @location(0) vec4<f32> {
    let has_uv0 = i.uv.x < 0.9999;
    let m0 = i32(i.mat + 0.5);
    // the textured exterior (paint, plastic, chrome, rubber and the wheels) is bump-mapped from the normal atlas, fading out with distance
    let nk = select(0.0, 1.0 - smoothstep(14.0, 40.0, distance(i.wp, u.cam.xyz)), has_uv0 && (m0 == 0 || m0 == 2 || m0 == 3 || m0 == 4));
    let n = map_normal(normalize(i.n), i.wp, i.uv, nk);
    let occl = mix(1.0, i.ao, u.mode.x);
    let m = i32(i.mat + 0.5);
    own_surface = i.own;
    // the livery (uniform control flow for the sample): a vertex with the sentinel UV (1, 1) has no texture and keeps its colour
    let lv = textureSample(livery_tex, livery_smp, clamp(i.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let has_uv = i.uv.x < 0.9999;
    let iv = textureSample(interior_tex, interior_smp, clamp(i.uv, vec2<f32>(0.0), vec2<f32>(1.0)));
    let albedo = select(i.col, i.col * lv.rgb, has_uv);
    if (m == 5) { return vec4<f32>(finish(i.col * (2.2 + 2.0 * u.disc.w)), 1.0); }       // lamps
    if (m == 7 || m == 8) {   // the car's own lamps: the shape is the mesh's, the state comes from the car; the class from the colour (8: a front position lamp)
        let c = i.col;
        let v = normalize(u.cam.xyz - i.wp);
        let facing = clamp(abs(dot(n, v)), 0.0, 1.0);
        let fr = 1.0 - facing;                                     // 0 looking straight into the lens, 1 at the rim
        var k = 0.4;
        var on = 0.0;
        if (m == 8) { on = select(0.0, 1.0, u.lamps.x > 0.5); k = 1.6 + 1.2 * u.disc.w; }                                  // position lamp: burns with the low beam
        else if (c.r > 0.8 && c.g < 0.3) { on = max(select(0.0, 0.3, u.lamps.x > 0.5), select(0.0, 1.0, u.lamps.y > 0.5)); k = 3.2; }   // tail unit: brake bright, headlights dim red
        else if (c.r > 0.8 && c.g < 0.75 && c.b < 0.4) { k = 0.5; }                                                         // indicator: not lit yet
        else if (c.b > 0.93) { on = select(0.0, 1.0, u.lamps.z > 0.5); k = 3.0; }                                          // reverse lamp
        else { on = select(0.0, 1.0, u.lamps.x > 0.5); k = 2.8 + 2.0 * u.disc.w; }                                          // headlight unit
        // a lit lens: a hot core looking straight into it, a darker rim, a stepped reflector (rings in the angle to the view)
        let rings = 0.86 + 0.14 * step(0.5, fract(fr * 6.0));
        let core = mix(0.45, 1.0, pow(facing, 3.0)) * rings;
        let lit = c * k * core * on;
        // an unlit lens: dark glass with a rim reflection of the sky and the sun
        let sunr = pow(max(dot(reflect(-v, n), normalize(u.sun.xyz)), 0.0), 40.0);
        let dark = (c * 0.10 + u.sky_hor.rgb * 0.45 * fr * fr * fr + vec3<f32>(sunr) * 0.6) * (1.0 - on);
        return vec4<f32>(finish(lit + dark), 1.0);
    }
    if (m == 11 || m == 12) {   // the reflector interior of a lamp unit: its texture, glowing when the lamp is on (front: the headlights; rear: tail light and brake, reverse)
        var on = 0.0;
        if (m == 11) { on = select(0.0, 1.0, u.lamps.x > 0.5); }
        else { on = max(select(0.0, 0.35, u.lamps.x > 0.5), max(select(0.0, 1.0, u.lamps.y > 0.5), select(0.0, 0.6, u.lamps.z > 0.5))); }
        let tint = select(vec3<f32>(1.0, 0.45, 0.35), vec3<f32>(1.0, 0.95, 0.85), m == 11);
        return vec4<f32>(shade(albedo * occl, i.wp, n, 0.3, 0.7) + finish(albedo * tint * on * 1.8), 1.0);
    }
    if (m == 15) {   // the interior and the driver: the interior atlas, in the shade of the roof (a cabin factor: little sun, no shadow map inside)
        let ia = select(vec3<f32>(0.12), iv.rgb, has_uv);
        return vec4<f32>(shade_with(ia * occl, i.wp, n, 0.7, 0.0, 0.3) * 0.7, 1.0);
    }
    if (m == 16) {   // the instrument cluster: like the interior, backlit when it is dark (u.disc.w)
        let ia = select(vec3<f32>(0.12), iv.rgb, has_uv);
        return vec4<f32>(shade_with(ia * occl, i.wp, n, 0.7, 0.0, 0.3) * 0.7 + finish(ia * (0.05 + 1.3 * u.disc.w)), 1.0);
    }
    if (m == 17) {   // an RPM LED (the number 1..4 in the green channel of the colour): lit when the revs reach its threshold
        let k = floor(i.col.g * 255.0 + 0.5);
        let on = select(0.0, 1.0, u.rt.w > 0.52 + 0.12 * k);
        let tone = select(select(vec3<f32>(1.0, 0.15, 0.1), vec3<f32>(1.0, 0.65, 0.1), k < 3.5), vec3<f32>(0.2, 1.0, 0.3), k < 2.5);
        return vec4<f32>(0.03 + finish(tone * (0.04 + on * 3.0)), 1.0);
    }
    if (m == 14) {   // a suspension part: its texture only (the vertex colour carries the weight and the corner)
        let sc = select(vec3<f32>(0.22), lv.rgb, has_uv);
        return vec4<f32>(shade(sc * occl, i.wp, n, 0.55, 0.5), 1.0);
    }
    if (m == 13) {   // a brake disc: textured metal that glows orange-red when hot (stronger in the dark)
        let hot = i.heat * i.heat;
        let glow = vec3<f32>(1.0, 0.32, 0.08) * hot * (1.2 + 3.0 * u.disc.w);
        return vec4<f32>(shade(albedo * occl, i.wp, n, 0.35, 1.0) + finish(glow), 1.0);
    }
    if (m == 1) { return vec4<f32>(shade(i.col * occl, i.wp, n, 0.06, 0.0), 1.0); }   // glass
    if (m == 4) { return vec4<f32>(shade(albedo * occl, i.wp, n, 0.22, 1.0) + own_surface * finish(lamp_aureole(i.wp, n)), 1.0); }   // chrome
    if (m == 6) { return vec4<f32>(shade_lite(i.col * occl, i.wp, n), 1.0); }   // foliage
    if (m == 0 && has_uv) { return vec4<f32>(shade_coat(albedo * occl, i.wp, n, normalize(i.n), 0.30) + own_surface * finish(lamp_aureole(i.wp, n)), 1.0); }   // the rally car's paint: clear-coated
    if (m == 0) { return vec4<f32>(shade(albedo * occl, i.wp, n, 0.28, 0.0) + own_surface * finish(lamp_aureole(i.wp, n)), 1.0); }   // paint
    if (m == 2) { return vec4<f32>(shade(albedo * occl, i.wp, n, 0.88, 0.0), 1.0); }   // rubber
    return vec4<f32>(shade(albedo * occl, i.wp, n, 0.5, 0.0) + own_surface * finish(lamp_aureole(i.wp, n)), 1.0);   // plastic
}

// The car's glass, blended over the opaque scene: a dark tint, the sky reflected by Fresnel, a sun glint; see-through near the car (the interior shows), solid and dark from afar
// (mirrored in `vehicle_models::glass_alpha`). The vertex colour's red is 1 for a clear lamp cover (a quarter of the alpha).
@fragment fn fs_glass(i: MOut) -> @location(0) vec4<f32> {
    let n = normalize(i.n);
    let to_cam = u.cam.xyz - i.wp;
    let dist = length(to_cam);
    let v = to_cam / max(dist, 0.001);
    let fr = pow(1.0 - clamp(abs(dot(n, v)), 0.0, 1.0), 4.0);   // concentrated at grazing angles: the glass has few, large triangles and a softer curve hides their edges
    let clear = clamp(i.col.r, 0.0, 1.0);
    let r = reflect(-v, n);
    let sun = normalize(u.sun.xyz);
    var col = vec3<f32>(0.03, 0.05, 0.08) * (1.0 - clear) + sky_gradient(r) * fr * 0.6 + u.sun_col.rgb * pow(max(dot(r, sun), 0.0), 60.0) * 0.6;
    let f = clamp(1.0 - exp(-dist * u.fog.x), 0.0, 1.0);
    col = mix(col, u.sky_hor.rgb, f);
    let t = smoothstep(0.0, 1.0, clamp((dist - 4.0) / 4.0, 0.0, 1.0));
    let k = 1.0 - 0.75 * clear;
    let near = (0.2 + 0.3 * fr) * k;
    var a = clamp(near + (0.95 - near) * t * k, 0.0, 1.0);
    // raindrops on the panes: a lattice of cells on the glass (object space, so they stay on it), each holding a drop that grows in once the wipers have passed (the cycle matches
    // `vehicle_models::wiper_angle`) and is cleared when the sweep starts over; none on a lamp cover
    let rain = u.wx.x;
    if (rain > 0.05 && clear < 0.5 && dist < 12.0) {
        let period = select(2.6, 1.3, rain > 0.6);
        let cyc = fract(u.cr.w / period);
        let g = vec2<f32>(i.lp.x, i.lp.y * 0.8 + i.lp.z * 0.6) / 0.05;
        let cell = floor(g);
        let h = hash_u3(u32(cell.x + 2000.0) * 7919u + u32(cell.y + 2000.0));
        let c = cell + vec2<f32>(0.3 + 0.4 * h.x, 0.3 + 0.4 * h.y);
        let rad = 0.14 + 0.2 * h.z * h.z;
        let born = h.x * 0.9;                                   // when in the cycle this drop appears
        let grown = smoothstep(born, born + 0.08, cyc);
        let d = length(g - c) / rad;
        let drop = (1.0 - smoothstep(0.7, 1.0, d)) * grown * step(h.y, 0.25 + 0.7 * rain);
        let rim = smoothstep(0.35, 0.85, d) * (1.0 - smoothstep(0.85, 1.0, d));   // a drop is a small lens: bright rim, the sky seen through its middle
        col += drop * (sky_gradient(r) * 0.5 + vec3<f32>(0.6) * rim * 0.5);
        a = clamp(a + drop * 0.35, 0.0, 1.0);
    }
    return vec4<f32>(finish(col), a);
}


// Rain: streaks on a lattice that is fixed in the world round the camera (so they fall past a moving car), wind-slanted, not inside the car; stateless (the position is a hash of
// the instance and the clock). `wx.x` is the rain (0..1): that share of the instances is alive.
const RAIN_N: f32 = 6000.0;
struct ROut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) a: f32 };
fn hash_u3(i: u32) -> vec3<f32> {
    var s = i * 747796405u + 2891336453u;
    var r = vec3<f32>(0.0);
    for (var k = 0; k < 3; k++) {
        s = s * 747796405u + 2891336453u;
        let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
        r[k] = f32((w >> 22u) ^ w) / 4294967295.0;
    }
    return r;
}
fn wrap_to(v: vec3<f32>, p: vec3<f32>) -> vec3<f32> { return v - floor(v / p + vec3<f32>(0.5)) * p; }
@vertex fn vs_rain(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> ROut {
    var o: ROut;
    o.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0);   // off screen unless the drop is alive
    o.uv = vec2<f32>(0.0); o.a = 0.0;
    if (f32(ii) >= u.wx.x * RAIN_N) { return o; }
    let h = hash_u3(ii);
    let bx = vec3<f32>(26.0, 14.0, 26.0);
    let drift = vec3<f32>(u.wind.x, 0.0, u.wind.y) * 0.8;
    let t = u.cr.w;
    let start = h * bx + drift * t - vec3<f32>(0.0, 11.0 * t, 0.0);
    let p = u.cam.xyz + wrap_to(start - u.cam.xyz, bx);
    // not inside the car
    let dc = p - u.cr.xyz;
    if (abs(dc.x) < 2.6 && abs(dc.z) < 2.6 && dc.y > -1.2 && dc.y < 1.8) { return o; }
    let dir = normalize(vec3<f32>(drift.x, -11.0, drift.z));
    let to_cam = p - u.cam.xyz;
    let dist = length(to_cam);
    let w = 0.004 + 0.0018 * dist;
    let side = normalize(cross(dir, to_cam)) * w;
    var q = array<vec2<f32>, 6>(vec2<f32>(-1.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(-1.0, 1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0));
    let c = q[vi];
    let wp = p + dir * (0.5 * c.y) + side * c.x;
    o.pos = u.vp * vec4<f32>(wp, 1.0);
    o.uv = c;
    o.a = (0.22 + 0.2 * u.wx.x) * (1.0 - smoothstep(8.0, 13.0, dist)) * smoothstep(0.6, 2.0, dist);
    return o;
}
@fragment fn fs_rain(i: ROut) -> @location(0) vec4<f32> {
    let edge = 1.0 - abs(i.uv.x);
    let col = u.sky_hor.rgb * 0.7 + vec3<f32>(0.25);
    return vec4<f32>(col, i.a * edge);
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
        if (u.mode.x > 0.5) { albedo = albedo * mix(0.62, 1.0, smoothstep(0.0, 5.0, i.wp.y - (i.ctr.y - i.size.y * 0.5))); }   // hybrid: darker at the base
        var col = lit(albedo, i.wp, n);
        if (abs(n.y) < 0.5 && u.disc.w > 0.05) {
            // night: about 45 % of the windows are lit (a warm glow that fades with the fog)
            let t2 = normalize(vec2<f32>(-n.z, n.x));
            let wc = vec2<f32>(dot(i.wp.xz, t2) / 3.0, i.wp.y / 3.5);
            let f2 = fract(wc);
            let win2 = step(0.25, f2.x) * step(f2.x, 0.75) * step(0.3, f2.y) * step(f2.y, 0.75);
            let on = step(0.55, hash2(floor(wc) + i.ctr.xz * 0.173));
            let fogk = clamp(1.0 - exp(-length(i.wp - u.cam.xyz) * u.fog.x), 0.0, 1.0);
            col = col + vec3<f32>(1.0, 0.78, 0.45) * 0.75 * win2 * on * smoothstep(0.1, 0.7, u.disc.w) * (1.0 - fogk);
        }
        return vec4<f32>(col, 1.0);
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
    let ndc = vec4<f32>(i.uv.x * 2.0 - 1.0, i.uv.y * 2.0 - 1.0, 1.0, 1.0);   // uv.y = 1 at the top of the screen (the sky was mirrored vertically before)
    let w = u.inv_vp * ndc;
    let dir = normalize(w.xyz / w.w - u.cam.xyz);
    var c = sky_gradient(dir);
    let night = u.disc.w;
    let sd = normalize(u.disc.xyz);
    let md = normalize(u.moon.xyz);
    let horizon = smoothstep(-0.02, 0.03, dir.y);
    // clouds: a layer of noise on a plane above the viewer, drifting with the wind. Lit like a participating medium: the optical depth grows with the density (Beer-Lambert),
    // the sun's light is attenuated by the density towards the sun (a second tap offset along it) and scattered forward (Henyey-Greenstein), which gives the bright silver
    // edges and the dark, dense cores and bases; the sky lights them from all sides. A high thin cirrus veil lies above.
    var cov = 0.0;
    if (dir.y > 0.02) {
        let sdir = normalize(vec3<f32>(sd.x, max(sd.y, 0.05), sd.z));
        let p = dir.xz / (dir.y + 0.12) * 0.6 - u.wind.zw * 0.58;   // the pattern drifts downwind (the offset integrates the wind on the CPU)
        let lo = 0.50 - 0.45 * u.wx.z; let hi = lo + 0.17;           // the weather's cloud cover lowers the threshold: overcast; a crisp edge
        let n = fbm2(p) - 0.10 * vnoise2(p * 7.0);                   // billows and erosion at the edges
        let dens = smoothstep(lo, hi, n);
        let ns = fbm2(p + sdir.xz / sdir.y * 0.08) - 0.10 * vnoise2((p + sdir.xz / sdir.y * 0.08) * 7.0);   // the density a little way towards the sun
        let dens_s = smoothstep(lo, hi, ns);
        let tau = dens * 5.0;
        let fade = smoothstep(0.02, 0.25, dir.y);
        cov = (1.0 - exp(-tau)) * fade * mix(1.0, 0.4, night);
        let cs = dot(dir, sd);
        let phase = mix(hg(cs, 0.62), hg(cs, -0.25), 0.35);        // forward lobe with a little back scatter
        let sun_in = u.sun_col.rgb * exp(-dens_s * 3.0);           // what reaches this point of the cloud
        let multi = 0.3 + 0.7 * exp(-dens * 1.2);                   // multiple scattering lifts the thick parts
        let amb_c = mix(u.sky_hor.rgb, u.sky_top.rgb, 0.6) * 0.8 * (1.0 - 0.5 * dens);
        let cloud = (sun_in * phase * 4.2 * multi + amb_c) * (1.0 - 0.3 * u.wx.z) * mix(1.0, 0.06, night);
        c = mix(c, cloud, cov * 0.95);
        // cirrus: thin streaks high up, in wisps stretched along the wind; faint, in clear weather mostly
        let pc = vec2<f32>(p.x * 0.45, p.y * 1.9) + vec2<f32>(11.0, 3.0);
        let ci = smoothstep(0.55, 0.85, fbm2(pc)) * fade * (0.30 - 0.18 * u.wx.z) * mix(1.0, 0.1, night);
        c = mix(c, cloud * 0.85 + u.sky_hor.rgb * 0.15, ci * (1.0 - cov));
        cov = max(cov, ci);
    }
    // stars: a hashed point field on the sphere, twinkling, only in the night and in clear sky
    if (night > 0.2 && dir.y > 0.0) {
        let g = dir * 110.0;
        let q = floor(g);
        let h = hash3(q);
        if (h > 0.985) {
            let centre = vec3<f32>(hash3(q + 17.0), hash3(q + 31.0), hash3(q + 53.0)) * 0.6 + 0.2;
            let d = length(fract(g) - centre);
            let mag = 0.35 + 0.65 * (h - 0.985) / 0.015;
            let tw = 0.75 + 0.25 * sin(u.fog.y * 3.0 + h * 400.0);
            let star = smoothstep(0.17, 0.0, d) * mag * tw;
            c = c + vec3<f32>(0.85, 0.9, 1.0) * star * 1.8 * smoothstep(0.2, 0.9, night) * (1.0 - cov) * smoothstep(0.0, 0.18, dir.y) * u.wx.w;
        }
    }
    // the moon: a disc whose lit side faces the sun, with a soft glow
    let sm = dot(dir, md);
    let rm = 0.0175;                                       // 1 degree
    let rgt = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), md));
    let upv = cross(md, rgt);
    let pm = vec2<f32>(dot(dir, rgt), dot(dir, upv)) / rm;
    let r2 = dot(pm, pm);
    let shown = horizon * (1.0 - 0.9 * cov) * u.wx.w;
    if (sm > 0.0 && r2 < 1.0) {
        let nrm = rgt * pm.x + upv * pm.y - md * sqrt(max(1.0 - r2, 0.0));
        let lit_m = clamp(dot(nrm, sd) * 2.5, 0.0, 1.0);
        let maria = 0.85 + 0.15 * vnoise2(pm * 3.0 + 7.0);
        let disc_m = vec3<f32>(0.86, 0.88, 0.95) * (0.05 + 0.95 * lit_m) * maria * 1.8;
        c = mix(c, disc_m, smoothstep(1.0, 0.92, sqrt(r2)) * shown);
    }
    c = c + vec3<f32>(0.35, 0.45, 0.75) * pow(max(sm, 0.0), 250.0) * 0.5 * shown;
    // the sun: disc, glow, red near the horizon, hidden below it
    let s = max(dot(dir, sd), 0.0);
    let sun_c = mix(vec3<f32>(1.0, 0.95, 0.80), vec3<f32>(1.0, 0.42, 0.14), 1.0 - smoothstep(0.0, 0.35, sd.y)) * smoothstep(-0.14, 0.0, sd.y);
    let sun_disc = 1.0 - smoothstep(1.7e-4, 2.3e-4, 1.0 - s);
    c = c + sun_c * (sun_disc * 14.0 * horizon * u.wx.w + pow(s, 400.0) * 3.0 * u.wx.w + hg(s, 0.78) * 0.9 * u.wx.w * (1.0 - 0.6 * u.wx.z) + pow(s, 3.0) * 0.02);
    return vec4<f32>(finish(c), 1.0);
}

struct WOut { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) depth: f32 };
@vertex fn vs_water(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> WOut {
    var o: WOut; o.pos = u.vp * vec4<f32>(p, 1.0); o.wp = p; o.depth = n.x; return o;
}
// Rain on water: rings spreading from random points of a lattice, each ring a small wave packet; `k` is the rain (0..1). Returns the slope (d height / d xz).
fn ripples(p: vec2<f32>, t: f32, k: f32) -> vec2<f32> {
    var g = vec2<f32>(0.0);
    for (var layer = 0; layer < 2; layer++) {
        let sc = select(1.3, 2.1, layer == 1);
        let q = p * sc + f32(layer) * 7.7;
        let cell = floor(q);
        let h = hash2(cell + f32(layer) * 31.0);
        let ph = fract(t * 0.9 + h * 3.1);                         // age of this cell's ring, 0..1
        let ctr = cell + vec2<f32>(0.25 + 0.5 * hash2(cell + 5.0), 0.25 + 0.5 * hash2(cell + 9.0));
        let d = q - ctr;
        let r = length(d);
        let rr = ph * 0.55;
        let w = exp(-pow((r - rr) * 14.0, 2.0)) * (1.0 - ph) * select(0.0, 1.0, h < k);
        g += normalize(d + vec2<f32>(1e-4)) * w * cos((r - rr) * 40.0) * 0.5 * sc;
    }
    return g;
}

@fragment fn fs_water(i: WOut) -> @location(0) vec4<f32> {
    let t = u.fog.y;
    let dist = length(i.wp - u.cam.xyz);
    // wave slope from drifting noise layers; the fine ones fade out with distance (they would shimmer) and the surface grows rougher in the rain
    let e = 0.15;
    let p1 = i.wp.xz * 0.35 + vec2<f32>(t * 0.15, t * 0.08);
    let p2 = i.wp.xz * 0.9 + vec2<f32>(-t * 0.2, t * 0.11);
    let p3 = i.wp.xz * 2.7 + vec2<f32>(t * 0.33, -t * 0.27);
    let h0 = vnoise2(p1) * 0.6 + vnoise2(p2) * 0.4;
    let hx = vnoise2(p1 + vec2<f32>(e, 0.0)) * 0.6 + vnoise2(p2 + vec2<f32>(e, 0.0)) * 0.4;
    let hz = vnoise2(p1 + vec2<f32>(0.0, e)) * 0.6 + vnoise2(p2 + vec2<f32>(0.0, e)) * 0.4;
    var sl = vec2<f32>(hx - h0, hz - h0) * 1.6;
    let fine = 1.0 - smoothstep(25.0, 90.0, dist);
    let c0 = vnoise2(p3);
    sl += vec2<f32>(vnoise2(p3 + vec2<f32>(e, 0.0)) - c0, vnoise2(p3 + vec2<f32>(0.0, e)) - c0) * (0.55 + 0.7 * u.wx.x) * fine;
    if (u.wx.x > 0.05 && dist < 60.0) { sl += ripples(i.wp.xz, t, u.wx.x) * (1.0 - smoothstep(30.0, 60.0, dist)) * 0.35; }
    let n = normalize(vec3<f32>(-sl.x, 1.0, -sl.y));
    let v = normalize(u.cam.xyz - i.wp);
    let nv = max(dot(n, v), 0.02);
    let d = max(i.depth, 0.0);
    // the body: light scattered back out of the water, darker and bluer with depth, lit by the sun and the sky
    let day = 0.5 + 0.5 * length(u.sun_col.rgb) / 2.6;
    let body = mix(vec3<f32>(0.05, 0.20, 0.22), vec3<f32>(0.01, 0.06, 0.13), smoothstep(0.0, 1.2, d)) * day;
    // physically based surface: water's F0 is 0.02; the roughness is the waves' slope variance (a calm pond is nearly a mirror, the rain and the wind rough it up)
    let rough = clamp(0.06 + 0.10 * u.wx.x + 0.04 * length(u.wind.xy) * 0.2 + 0.0 * dist, 0.05, 0.35);
    let f0 = vec3<f32>(0.02);
    let spec_env = sky_env(reflect(-v, n), rough) * env_brdf(f0, rough, nv);
    var c = body * (1.0 - fresnel(f0, nv)) + spec_env;
    let sd = normalize(u.sun.xyz);
    c = c + u.sun_col.rgb * ggx_spec(n, v, sd, f0, rough * rough) * 0.9;
    if (u.hl_pos.w > 0.0) {   // the headlights' beam glitters on the water as it does on wet ground
        let l = normalize(i.wp - u.hl_pos.xyz);
        let g = headbeam(i.wp);
        if (g > 0.0) { c = c + vec3<f32>(1.0, 0.93, 0.80) * (14.0 * g) * (ggx_spec(n, v, -l, f0, max(rough * rough, 0.01)) * 0.5 + body * 0.4 * max(dot(n, -l), 0.0)); }
    }
    let f = clamp(1.0 - exp(-dist * u.fog.x), 0.0, 1.0);
    c = mix(c, u.sky_hor.rgb, f);
    // see-through in the shallows; a steep view into deep water is clearer than a grazing one (which is mostly mirror)
    let alpha = smoothstep(0.0, 0.5, i.depth) * mix(0.35, 0.85, smoothstep(0.0, 1.2, d));
    return vec4<f32>(finish(c), clamp(alpha + fresnel(f0, nv).x * 1.5, 0.0, 1.0));
}

struct RIn { @builtin(position) pos: vec4<f32>, @location(0) wp: vec3<f32>, @location(1) a: vec3<f32> };
@vertex fn vs_road(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>) -> RIn {
    var o: RIn; o.pos = u.vp * vec4<f32>(p, 1.0); o.wp = p; o.a = n; return o;
}
// a = (across -1..1, metres along the road, kind * 100 + width): gravel (kind 0) or tarmac (kind 1)
@fragment fn fs_road(i: RIn) -> @location(0) vec4<f32> {
    let nd = normalize(cross(dpdy(i.wp), dpdx(i.wp)));
    let up = select(-nd, nd, nd.y > 0.0);
    let kind = floor(i.a.z / 100.0);
    let asphalt = kind >= 1.0;
    let across = i.a.x; let s = i.a.y; let ac = abs(across);
    let grain = 0.80 + 0.40 * vnoise2(i.wp.xz * 1.7) * (0.6 + 0.4 * vnoise2(i.wp.xz * 0.23 + vec2<f32>(3.0, 1.0)));
    var col: vec3<f32>;
    var alpha = 1.0;
    if (kind >= 2.0) {
        // city loop lines: a checkered start/finish line (kind 2) and a plain check line (kind 3)
        let chk = (floor((across * 0.5 + 0.5) * 6.0) + floor(s)) % 2.0;
        col = select(vec3<f32>(0.92), vec3<f32>(0.06), chk < 0.5);
        if (kind >= 3.0) { col = vec3<f32>(0.95, 0.78, 0.15); }
    } else if (asphalt) {
        col = vec3<f32>(0.115, 0.115, 0.125) * grain;
        if (ac > 0.86 && ac < 0.92) { col = mix(col, vec3<f32>(0.62), 0.9); }
        if (ac < 0.025 && fract(s / 9.0) < 0.5) { col = mix(col, vec3<f32>(0.70, 0.66, 0.30), 0.9); }
    } else {
        col = vec3<f32>(0.47, 0.40, 0.31) * grain;
        let rut = smoothstep(0.12, 0.22, ac) * (1.0 - smoothstep(0.38, 0.50, ac));
        col = col * (1.0 - 0.18 * rut);
        col = col * (1.0 + 0.10 * (1.0 - smoothstep(0.0, 0.10, ac)) + 0.10 * smoothstep(0.72, 0.9, ac));
        alpha = 1.0 - smoothstep(0.86, 1.0, ac);
    }
    col = mix(col, vec3<f32>(0.72, 0.58, 0.42), 0.5 * dust_layer(i.wp));   // settled dust
    // micro-relief: aggregate in the asphalt (2 mm), loose gravel and ruts on the track (2 cm), fading out with distance
    let near = 1.0 - smoothstep(12.0, 45.0, distance(i.wp, u.cam.xyz));
    let relief = select(0.02 * vnoise2(i.wp.xz * 14.0) + 0.03 * vnoise2(i.wp.xz * 4.0), 0.002 * vnoise2(i.wp.xz * 90.0) + 0.002 * vnoise2(i.wp.xz * 30.0), asphalt);
    return vec4<f32>(lit(col, i.wp, bump(up, i.wp, relief * near)), alpha);
}

struct DOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) c: vec4<f32>, @location(2) wp: vec3<f32>, @location(3) e: vec4<f32> };
fn dust_vertex(vi: u32, ps: vec4<f32>, c: vec4<f32>, e: vec4<f32>, size: f32) -> DOut {
    var q = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0));
    let right = normalize(vec3<f32>(u.vp[0].x, u.vp[1].x, u.vp[2].x));
    let up = normalize(vec3<f32>(u.vp[0].y, u.vp[1].y, u.vp[2].y));
    let corner = q[vi];
    // every puff turns slowly about its own axis, so the noise of the sprite does not look the same twice
    let ang = e.x * 6.2831 + e.y * 1.5;
    let rc = vec2<f32>(corner.x * cos(ang) - corner.y * sin(ang), corner.x * sin(ang) + corner.y * cos(ang));
    let wp = ps.xyz + (right * rc.x + up * rc.y) * size;
    var o: DOut;
    o.pos = u.vp * vec4<f32>(wp, 1.0); o.uv = rc; o.c = c; o.wp = wp; o.e = vec4<f32>(e.x, e.y, e.z, e.w);
    return o;
}
@vertex fn vs_dust(@builtin(vertex_index) vi: u32, @location(0) ps: vec4<f32>, @location(1) c: vec4<f32>, @location(2) e: vec4<f32>) -> DOut {
    var o = dust_vertex(vi, ps, c, e, ps.w);
    if (e.z > 1.5) { o.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0); }   // the specks are drawn by their own pass at full resolution
    return o;
}
// the flying specks of grit: tiny, so never smaller than a pixel or two on the screen
@vertex fn vs_dust_specks(@builtin(vertex_index) vi: u32, @location(0) ps: vec4<f32>, @location(1) c: vec4<f32>, @location(2) e: vec4<f32>) -> DOut {
    let size = max(ps.w, 0.0013 * length(ps.xyz - u.cam.xyz));
    var o = dust_vertex(vi, ps, c, e, size);
    if (e.z < 1.5 || c.a < 0.02) { o.pos = vec4<f32>(2.0, 2.0, 2.0, 1.0); }
    return o;
}
@fragment fn fs_dust_speck(i: DOut) -> @location(0) vec4<f32> {
    let r = length(i.uv);
    let a = i.c.a * (1.0 - smoothstep(0.5, 1.0, r));
    if (a < 0.03) { discard; }
    let sun = clamp(length(u.sun_col.rgb) / 2.6, 0.06, 1.0);
    let amb = (u.sky_hor.rgb * 0.15 + vec3<f32>(0.62, 0.55, 0.45)) * 0.55 * mix(1.0, 0.2, u.disc.w);
    var col = i.c.rgb * (amb * 1.6 + vec3<f32>(sun) * 1.1);
    let dcam = length(i.wp - u.cam.xyz);
    let f = clamp(1.0 - exp(-dcam * u.fog.x), 0.0, 1.0) * 0.6;
    col = mix(col, u.sky_hor.rgb, f);
    return vec4<f32>(finish(col), a);
}
@fragment fn fs_dust(i: DOut) -> @location(0) vec4<f32> {
    let r = length(i.uv);
    // a ragged, noisy puff instead of a smooth disc: the edge radius wanders with a noise that is different for every puff
    let n = vnoise2(i.uv * 2.6 + vec2<f32>(i.e.x * 31.0, i.e.x * 17.0));
    let edge = mix(0.55, 1.05, n);
    let shape = 1.0 - smoothstep(0.1, edge, r);
    let wp = i.wp;
    let dcam = length(wp - u.cam.xyz);
    let near = smoothstep(2.0, 9.0, dcam);   // puffs close to the camera thin out instead of blinding the view
    if (i.c.a * shape * near < 0.004) { discard; }   // most of a sprite is empty: skip the lighting there
    // soft against the ground: a puff that meets the terrain fades out instead of cutting a hard line
    let above = wp.y - terrain_h(wp.x, wp.z);
    let soft = mix(0.4, 1.0, smoothstep(0.0, 0.7, above));
    let a = i.c.a * shape * near * soft;
    // light: the sun, the sky, the glow of a cloud seen against the sun, the headlights and the night
    let sun_dir = normalize(u.sun.xyz);
    let view = normalize(wp - u.cam.xyz);
    let sun = clamp(length(u.sun_col.rgb) / 2.6, 0.06, 1.0);
    let fwd = pow(max(dot(view, sun_dir), 0.0), 5.0);
    let amb = (u.sky_hor.rgb * 0.4 + vec3<f32>(0.45)) * 0.5 * mix(1.0, 0.2, u.disc.w);
    var col = i.c.rgb * (amb + vec3<f32>(sun) * (0.55 + 0.3 * shape + 1.2 * fwd * sun));
    col = col * mix(0.82, 1.0, smoothstep(0.0, 0.8, r));   // the thick middle of a puff is shaded by the rest of it
    col = col + i.c.rgb * (vec3<f32>(1.0, 0.93, 0.80) * (6.0 * headbeam(wp)) + u.tl_col.rgb * (5.0 * taillight(wp)));   // the lamps light the dust: the beam ahead, the red tail lights behind
    let f = clamp(1.0 - exp(-dcam * u.fog.x), 0.0, 1.0);
    col = mix(col, u.sky_hor.rgb, f);
    return vec4<f32>(finish(col), a);
}

// The GPU dust: tens of thousands of soft, rotating, noisy-edged particles whose overlap builds the cloud. `e.w` is the light of the particle
// (the dust above it towards the sun shades it, computed by the step pass); the puff is soft against the terrain and fades near the camera.
fn dust_pt(i: DOut) -> vec4<f32> {
    // a soft puff, not a disc and not grains: a smooth falloff whose radius wobbles slowly (low-frequency noise), so overlapping puffs blend into one smooth cloud
    let r = length(i.uv);
    let n = vnoise2(i.uv * 0.9 + vec2<f32>(i.e.x * 31.0, i.e.x * 17.0));
    let rr = r / (0.78 + 0.34 * n);
    let body = 1.0 - smoothstep(0.0, 1.0, clamp(rr, 0.0, 1.0));
    // inside the puff a second, finer layer of soft noise gives the cloud its billowing structure (smooth, never grainy)
    let n2 = vnoise2(i.uv * 2.3 + vec2<f32>(i.e.x * 7.0 + i.e.y * 3.0, i.e.x * 13.0));
    let shape = body * body * (0.55 + 0.45 * body) * (0.84 + 0.32 * n2);
    let wp = i.wp;
    let dcam = length(wp - u.cam.xyz);
    let near = smoothstep(3.0, 10.0, dcam);   // dust right at the camera fades, so the picture does not fill with a veil
    if (i.c.a * shape * near < 0.003) { return vec4<f32>(0.0); }
    let a = i.c.a * shape * near;   // softness against the terrain and the car comes from the sampled depth of the low-resolution pass
    let sun_dir = normalize(u.sun.xyz);
    let view = normalize(wp - u.cam.xyz);
    let sun = clamp(length(u.sun_col.rgb) / 2.6, 0.06, 1.0);
    let fwd = pow(max(dot(view, sun_dir), 0.0), 4.0);   // forward scattering: a cloud seen against the sun glows
    let amb = (u.sky_hor.rgb * 0.15 + vec3<f32>(0.62, 0.55, 0.45)) * 0.55 * mix(1.0, 0.2, u.disc.w);   // warm skylight: dust is tan, not grey
    let light = max(i.e.w, 0.02);
    var col = i.c.rgb * (amb * 2.5 * mix(0.55, 1.0, light) + vec3<f32>(sun) * (1.35 * light) + vec3<f32>(1.0, 0.72, 0.4) * (3.2 * fwd * sun * light));   // thin lit dust glows amber against the sun, the thick core is dark
    // every puff is shaded like a soft sphere (the sun in the view's frame): bright on the sunny side, a soft grey underside, so the cloud has a volume
    let vr = normalize(vec3<f32>(u.vp[0].x, u.vp[1].x, u.vp[2].x));
    let vu = normalize(vec3<f32>(u.vp[0].y, u.vp[1].y, u.vp[2].y));
    let vb = normalize(cross(vr, vu));
    let n3 = vec3<f32>(i.uv, sqrt(max(1.0 - r * r, 0.0)));
    let ld = vec3<f32>(dot(sun_dir, vr), dot(sun_dir, vu), dot(sun_dir, vb));
    let sh = clamp(dot(n3, ld) * 0.5 + 0.5, 0.0, 1.0);
    col = col * (0.34 + 1.2 * sh);
    col = col + i.c.rgb * (vec3<f32>(1.0, 0.93, 0.80) * (6.0 * headbeam(wp)) + u.tl_col.rgb * (5.0 * taillight(wp)));   // the lamps light the dust: the beam ahead, the red tail lights behind
    let f = clamp(1.0 - exp(-dcam * u.fog.x), 0.0, 1.0) * 0.6;
    col = mix(col, u.sky_hor.rgb, f);
    return vec4<f32>(finish(col), a);
}

@fragment fn fs_dust_pt(i: DOut) -> @location(0) vec4<f32> {
    let c = dust_pt(i);
    if (c.a < 0.003) { discard; }
    return c;
}

@group(1) @binding(3) var vol_depth: texture_depth_2d;   // the scene's depth, sampled by the low-resolution dust pass (group 1 of `raster/dust_gpu.rs`)
const DUST_DIV: f32 = 8.0;   // the low-resolution dust target is 1/DUST_DIV of the screen (= DUST_DIV in raster/dust_gpu.rs)
// the same particle drawn into the low-resolution target: premultiplied, hidden behind and soft against the scene (the depth buffer is sampled)
@fragment fn fs_dust_lo(i: DOut) -> @location(0) vec4<f32> {
    var c = dust_pt(i);
    if (c.a < 0.003) { discard; }
    c = vec4<f32>(c.rgb + vec3<f32>(fract(52.9829189 * fract(dot(i.pos.xy, vec2<f32>(0.06711056, 0.00583715)))) - 0.5) * (1.5 / 255.0), c.a);   // a little dither against the banding of soft gradients in the 8-bit target
    let dims = textureDimensions(vol_depth);
    let px = clamp(vec2<i32>(i.pos.xy * DUST_DIV), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
    let d = textureLoad(vol_depth, px, 0);
    if (d < 0.9999) {
        let ndc = vec2<f32>((f32(px.x) + 0.5) / f32(dims.x) * 2.0 - 1.0, 1.0 - (f32(px.y) + 0.5) / f32(dims.y) * 2.0);
        let ws = u.inv_vp * vec4<f32>(ndc, d, 1.0);
        let t_scene = length(ws.xyz / ws.w - u.cam.xyz);
        c.a = c.a * clamp((t_scene - length(i.wp - u.cam.xyz)) / 1.2, 0.0, 1.0);
    }
    if (c.a < 0.003) { discard; }
    return vec4<f32>(c.rgb * c.a, c.a);
}

// the half-resolution dust drawn over the frame (bilinear)
struct COut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@group(1) @binding(4) var comp_tex: texture_2d<f32>;
@group(1) @binding(5) var comp_smp: sampler;
@vertex fn vs_vcomp(@builtin(vertex_index) vi: u32) -> COut {
    var q = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var o: COut; o.pos = vec4<f32>(q[vi], 0.0, 1.0); o.uv = vec2<f32>(q[vi].x * 0.5 + 0.5, 0.5 - q[vi].y * 0.5); return o;
}
@fragment fn fs_vcomp(i: COut) -> @location(0) vec4<f32> { return textureSampleLevel(comp_tex, comp_smp, i.uv, 0.0); }

// The headlights' beam seen in the air (haze, rain, fog), drawn into the low-resolution dust target: per pixel a short march along the view ray up to the scene's depth; every
// sample adds the beam's light there (`headbeam_raw`) times the air's density, with a phase that is faint from behind the beam and strong from in front.
@fragment fn fs_beam(i: COut) -> @location(0) vec4<f32> {
    let ndc = vec4<f32>(i.uv.x * 2.0 - 1.0, 1.0 - i.uv.y * 2.0, 1.0, 1.0);
    let w = u.inv_vp * ndc;
    let dir = normalize(w.xyz / w.w - u.cam.xyz);
    let dims = textureDimensions(vol_depth);
    let px = clamp(vec2<i32>(i.pos.xy * DUST_DIV), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
    let d = textureLoad(vol_depth, px, 0);
    var t_end = 70.0;
    if (d < 0.9999) {
        let ndc2 = vec2<f32>((f32(px.x) + 0.5) / f32(dims.x) * 2.0 - 1.0, 1.0 - (f32(px.y) + 0.5) / f32(dims.y) * 2.0);
        let ws = u.inv_vp * vec4<f32>(ndc2, d, 1.0);
        t_end = min(t_end, length(ws.xyz / ws.w - u.cam.xyz));
    }
    let t0 = 0.8;
    if (t_end <= t0 + 0.2) { return vec4<f32>(0.0); }
    // the air: a faint haze always, thicker in fog and rain
    let sigma = 0.0025 + 1.4 * u.fog.x + 0.006 * u.wx.x;
    let n = 10;
    let ratio = t_end / t0;
    let ds_k = log(ratio) / f32(n);
    var acc = 0.0;
    for (var k = 0; k < n; k++) {
        let t = t0 * pow(ratio, (f32(k) + 0.5) / f32(n));
        let p = u.cam.xyz + dir * t;
        let g = headbeam_raw(p);   // no terrain test per sample (cost): the shaft is faint, the lit ground is tested
        if (g > 0.003) {
            let ds = t * ds_k;
            let l = normalize(p - u.hl_pos.xyz);
            let cs = dot(l, normalize(u.cam.xyz - p));   // 1: the camera looks into the lamps along the beam, -1: from behind them
            let phase = 0.30 + 0.70 * pow(max(0.5 + 0.5 * cs, 0.0), 3.0);
            acc = acc + g * ds * sigma * phase;
        }
    }
    let c = vec3<f32>(1.0, 0.93, 0.80) * acc * 9.0;
    return vec4<f32>(min(c, vec3<f32>(1.5)), 0.0);   // premultiplied with no alpha: added to the dust target
}
