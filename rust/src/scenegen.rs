//! Turns a parsed scene into WGSL: constants, `material`, `map`, `map_gi`, `occ_all`, `occ_cheap`.
//! The numbers are written as literals (the scene is compiled into the shader): a data-driven uniform version was ~15x
//! slower on the target GPU, because the driver could no longer fold constants or drop unused shapes.
use std::fmt::Write;

use crate::scene::{Material, Obj, SceneData, Shape};

fn f(x: f32) -> String {
    let s = format!("{x:?}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") { s } else { format!("{s}.0") }
}

fn v3(v: [f32; 3]) -> String { format!("vec3<f32>({}, {}, {})", f(v[0]), f(v[1]), f(v[2])) }

fn vec2(a: f32, b: f32) -> String { format!("vec2<f32>({}, {})", f(a), f(b)) }

/// Distance of one object (`fast`: the cheap GI version).
fn dist(o: &Obj, fast: bool) -> String {
    let c = v3(o.pos);
    match &o.shape {
        Shape::Sphere { r } => format!("d_sphere(p, {c}, {})", f(*r)),
        Shape::Box { half, round } => format!("d_box(p, {c}, {}, {}, {})", v3(*half), f(*round), f(o.spin)),
        Shape::Torus { big, small } => format!("d_torus(p, {c}, {}, {}, {})", vec2(*big, *small), f(o.spin), f(o.bound)),
        Shape::Menger { s } => format!("d_menger(p, {c}, {}, {}, {}, {fast})", f(*s), f(o.spin), f(o.bound)),
        Shape::Cylinder { r, h } => format!("d_cyl(p, {c}, {}, {}, {}, {})", f(*r), f(*h), f(o.spin), f(o.bound)),
        Shape::Prism { side, len, yaw } => format!("d_prism(p, {c}, {}, {}, {}, {}, {})", f(*side), f(*len), f(*yaw), f(o.spin), f(o.bound)),
        Shape::Blobs { pos2, r1, r2, k } => format!("d_pair(p, {c}, {}, {}, {}, {}, {})", v3(*pos2), f(*r1), f(*r2), f(*k), f(o.bound)),
    }
}

/// Shadow-ray test of one object: true when it blocks (`ro`, `dir`, `tmax` are the function's parameters).
fn occ(o: &Obj) -> String {
    let c = v3(o.pos);
    match &o.shape {
        Shape::Sphere { r } => format!("sph_occ(ro, dir, {c}, {}, tmax)", f(*r)),
        Shape::Box { half, .. } => format!("box_occ(ro, dir, tmax, {c}, {}, {})", v3(*half), f(o.spin)),
        Shape::Torus { big, small } => format!("occ_torus(ro, dir, tmax, {c}, {}, {}, {})", vec2(*big, *small), f(o.spin), f(o.bound)),
        Shape::Menger { s } => format!("occ_menger(ro, dir, tmax, {c}, {}, {}, {})", f(*s), f(o.spin), f(o.bound)),
        Shape::Cylinder { r, h } => format!("occ_cyl(ro, dir, tmax, {c}, {}, {}, {}, {})", f(*r), f(*h), f(o.spin), f(o.bound)),
        Shape::Prism { side, len, yaw } => format!("occ_prism(ro, dir, tmax, {c}, {}, {}, {}, {})", f(*side), f(*len), f(*yaw), f(o.spin)),
        Shape::Blobs { pos2, r1, r2, .. } => format!("occ_pair(ro, dir, tmax, {c}, {}, {}, {})", v3(*pos2), f(*r1), f(*r2)),
    }
}

fn material_case(m: &Material) -> String {
    let mut s = format!("m.albedo = {}; m.kind = {}u; m.rough = {}; m.ior = {}; m.disp = {};", v3(m.color), m.kind, f(m.rough), f(m.ior), f(m.dispersion));
    if let Some((c2, k)) = m.checker {
        let _ = write!(s, " if (((i32(floor(p.x * {k})) + i32(floor(p.z * {k}))) & 1) == 1) {{ m.albedo = {c2}; }}", c2 = v3(c2), k = f(k));
    }
    if m.kind == 3 {
        let e = match m.slot { 1 => "P.col_a".to_string(), 2 => "P.col_b".to_string(), _ => format!("{} * P.lk", v3(m.emit)) };
        let _ = write!(s, " m.emit = {e};");
    }
    s
}

pub fn scene_wgsl(sc: &SceneData) -> String {
    let mut s = String::new();
    match &sc.sun {
        Some(u) => {
            let _ = writeln!(s, "const SUN_ON: bool = true;\nconst SUN_DIR: vec3<f32> = {};\nconst SUN_COS: f32 = {};\nconst SUN_COL: vec3<f32> = {};", v3(u.dir), f(u.cos), v3(u.col));
        }
        None => {
            let _ = writeln!(s, "const SUN_ON: bool = false;\nconst SUN_DIR: vec3<f32> = vec3<f32>(0.0, 1.0, 0.0);\nconst SUN_COS: f32 = 1.0;\nconst SUN_COL: vec3<f32> = vec3<f32>(0.0);");
        }
    }
    for i in 0..2 {
        match sc.lamps.get(i) {
            Some(l) => { let _ = writeln!(s, "const L{i}_R: f32 = {};\nconst L{i}_C: vec3<f32> = {};\nconst L{i}_SLOT: i32 = {};\nconst L{i}_COL: vec3<f32> = {};", f(l.r), v3(l.pos), l.slot, v3(l.col)); }
            None => { let _ = writeln!(s, "const L{i}_R: f32 = 0.0;\nconst L{i}_C: vec3<f32> = vec3<f32>(0.0);\nconst L{i}_SLOT: i32 = 0;\nconst L{i}_COL: vec3<f32> = vec3<f32>(0.0);"); }
        }
    }
    match sc.caustic {
        Some((c, r)) => { let _ = writeln!(s, "const CAU_R: f32 = {};\nconst CAU_C: vec3<f32> = {};", f(r), v3(c)); }
        None => { let _ = writeln!(s, "const CAU_R: f32 = 0.0;\nconst CAU_C: vec3<f32> = vec3<f32>(0.0);"); }
    }
    for (name, fast) in [("map", false), ("map_gi", true)] {
        let _ = writeln!(s, "fn {name}(p: vec3<f32>) -> vec2<f32> {{\n  var r = vec2<f32>(1e9, -1.0);");
        for (i, o) in sc.objs.iter().enumerate() {
            let _ = writeln!(s, "  r = opU(r, {}, {i}.0);", dist(o, fast));
        }
        s.push_str("  return r;\n}\n");
    }
    for cheap in [false, true] {
        let name = if cheap { "occ_cheap" } else { "occ_all" };
        let _ = writeln!(s, "fn {name}(ro: vec3<f32>, dir: vec3<f32>, tmax: f32, id: i32) -> vec3<f32> {{\n  var tr = vec3<f32>(1.0);");
        for (i, o) in sc.objs.iter().enumerate() {
            if cheap && o.shape.complex() && !matches!(o.shape, Shape::Prism { .. }) { continue; }
            let blob = matches!(o.shape, Shape::Blobs { .. });
            let test = if blob { format!("id != {i} && {}", occ(o)) } else { occ(o) };
            if o.filter {
                let _ = writeln!(s, "  if ({test}) {{ tr = tr * {} * 0.92; }}", v3(sc.mats[o.mat].color));
            } else {
                let _ = writeln!(s, "  if ({test}) {{ return vec3<f32>(0.0); }}");
            }
        }
        s.push_str("  return tr;\n}\n");
    }
    s.push_str("fn material(p: vec3<f32>, id: f32) -> Mat {\n  var m: Mat;\n  m.albedo = vec3<f32>(0.8);\n  m.emit = vec3<f32>(0.0);\n  m.kind = 0u;\n  m.rough = 0.0;\n  m.ior = 1.5;\n  m.disp = 0.0;\n  switch (i32(id + 0.5)) {\n");
    for (i, o) in sc.objs.iter().enumerate() {
        let _ = writeln!(s, "    case {i}: {{ {} }}", material_case(&sc.mats[o.mat]));
    }
    s.push_str("    default: {}\n  }\n  return m;\n}\n");
    s
}

pub const PHOTONS: u32 = 65536;
/// Fixed-point scale of the photon counters (u32): power * FIXED.
pub const FIXED: f32 = 1_048_576.0;

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn unit(a: [f32; 3]) -> [f32; 3] { let l = dot(a, a).sqrt(); [a[0] / l, a[1] / l, a[2] / l] }

pub fn total_cells(sc: &SceneData) -> u32 { sc.receivers.iter().map(|r| r.cells[0] * r.cells[1]).sum() }
pub fn receiver_offset(sc: &SceneData, i: usize) -> u32 { sc.receivers.iter().take(i).map(|r| r.cells[0] * r.cells[1]).sum() }

/// `caustic_at(p)`: the photon map's irradiance / pi at world point p (zero off the receivers). with_map: the module declares CMAP.
pub fn caustic_at_wgsl(sc: &SceneData, with_map: bool) -> String {
    let mut s = String::new();
    if with_map && sc.photons() {
        s.push_str("@group(0) @binding(4) var<storage, read> CMAP: array<vec4<f32>>;\n");
        s.push_str("fn cmap_cell(off: u32, nu: u32, nv: u32, a: f32, b: f32) -> vec3<f32> {\n  let x = clamp(a * f32(nu) - 0.5, 0.0, f32(nu) - 1.0);\n  let y = clamp(b * f32(nv) - 0.5, 0.0, f32(nv) - 1.0);\n  let x0 = u32(floor(x));\n  let y0 = u32(floor(y));\n  let x1 = min(x0 + 1u, nu - 1u);\n  let y1 = min(y0 + 1u, nv - 1u);\n  let fx = x - floor(x);\n  let fy = y - floor(y);\n  let c00 = CMAP[off + y0 * nu + x0].xyz;\n  let c10 = CMAP[off + y0 * nu + x1].xyz;\n  let c01 = CMAP[off + y1 * nu + x0].xyz;\n  let c11 = CMAP[off + y1 * nu + x1].xyz;\n  return mix(mix(c00, c10, fx), mix(c01, c11, fx), fy);\n}\n");
        s.push_str("fn caustic_at(p: vec3<f32>) -> vec3<f32> {\n");
        for (i, r) in sc.receivers.iter().enumerate() {
            let n = unit(cross(r.u, r.v));
            let _ = writeln!(s, "  {{\n    let q = p - {o};\n    if (abs(dot(q, {n})) < 0.004) {{\n      let a = dot(q, {u}) / {uu};\n      let b = dot(q, {v}) / {vv};\n      if (a >= 0.0 && a < 1.0 && b >= 0.0 && b < 1.0) {{ return cmap_cell({off}u, {nu}u, {nv}u, a, b); }}\n    }}\n  }}",
                o = v3(r.origin), n = v3(n), u = v3(r.u), v = v3(r.v), uu = f(dot(r.u, r.u)), vv = f(dot(r.v, r.v)),
                off = receiver_offset(sc, i), nu = r.cells[0], nv = r.cells[1]);
        }
        s.push_str("  return vec3<f32>(0.0);\n}\n");
    } else {
        s.push_str("fn caustic_at(p: vec3<f32>) -> vec3<f32> {\n  return vec3<f32>(0.0);\n}\n");
    }
    s
}

/// Constants and receiver helpers of the photon and resolve passes (the scene must have a prism and receivers).
pub fn photon_wgsl(sc: &SceneData) -> String {
    let p = sc.prism().expect("photon_wgsl needs a prism");
    let m = &sc.mats[p.mat];
    let Shape::Prism { side, len, yaw } = p.shape else { unreachable!() };
    let mut s = format!(
        "const PR_C: vec3<f32> = {};\nconst PR_SIDE: f32 = {};\nconst PR_LEN: f32 = {};\nconst PR_YAW: f32 = {};\nconst PR_SPIN: f32 = {};\nconst PR_BOUND: f32 = {};\nconst PR_IOR: f32 = {};\nconst PR_DISP: f32 = {};\nconst PHOTONS: f32 = {};\nconst FIXED: f32 = {};\nconst TOTAL_CELLS: u32 = {}u;\n",
        v3(p.pos), f(side), f(len), f(yaw), f(p.spin), f(p.bound), f(m.ior), f(m.dispersion), f(PHOTONS as f32), f(FIXED), total_cells(sc));
    // rx_hit: nearest receiver hit by the ray (p, d): (counter cell index, t); index -1 on a miss
    s.push_str("fn rx_hit(p: vec3<f32>, d: vec3<f32>) -> vec2<f32> {\n  var best = vec2<f32>(-1.0, 1e9);\n");
    for (i, r) in sc.receivers.iter().enumerate() {
        let n = unit(cross(r.u, r.v));
        let _ = writeln!(s, "  {{\n    let den = dot({n}, d);\n    if (abs(den) > 1e-6) {{\n      let t = dot({o} - p, {n}) / den;\n      if (t > 1e-3 && t < best.y) {{\n        let q = p + d * t - {o};\n        let a = dot(q, {u}) / {uu};\n        let b = dot(q, {v}) / {vv};\n        if (a >= 0.0 && a < 1.0 && b >= 0.0 && b < 1.0) {{\n          let cx = min(u32(a * {nu}.0), {nu1}u);\n          let cy = min(u32(b * {nv}.0), {nv1}u);\n          best = vec2<f32>(f32({off}u + cy * {nu}u + cx), t);\n        }}\n      }}\n    }}\n  }}",
            n = v3(n), o = v3(r.origin), u = v3(r.u), v = v3(r.v), uu = f(dot(r.u, r.u)), vv = f(dot(r.v, r.v)),
            nu = r.cells[0], nv = r.cells[1], nu1 = r.cells[0] - 1, nv1 = r.cells[1] - 1, off = receiver_offset(sc, i));
    }
    s.push_str("  return best;\n}\n");
    // cell_area: area of one cell of the receiver that owns counter cell i
    s.push_str("fn cell_area(i: u32) -> f32 {\n");
    for (i, r) in sc.receivers.iter().enumerate() {
        let area = dot(cross(r.u, r.v), cross(r.u, r.v)).sqrt() / (r.cells[0] * r.cells[1]) as f32;
        let _ = writeln!(s, "  if (i < {}u) {{ return {}; }}", receiver_offset(sc, i) + r.cells[0] * r.cells[1], f(area));
    }
    s.push_str("  return 1.0;\n}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_are_valid_wgsl_literals() {
        assert_eq!(f(1.0), "1.0");
        assert_eq!(f(0.25), "0.25");
        assert_eq!(f(-3.0), "-3.0");
        assert!(f(1e-7).contains('e'));
        assert!(f(60.0).ends_with(".0"));
    }

    #[test]
    fn generated_code_mentions_every_object() {
        for i in 0..crate::scene::BUILTIN.len() {
            let sc = crate::scene::builtin(i);
            let g = scene_wgsl(&sc);
            assert!(g.contains(&format!("{}.0);", sc.objs.len() - 1)), "{}", sc.name);
            assert!(g.contains("fn occ_all") && g.contains("fn material"));
        }
    }

    #[test]
    fn prism_scene_generates_prism_code_and_validates() {
        let sc = crate::scene::parse(r#"{"materials":[{"name":"g","kind":"glass","dispersion":2.0}],
            "objects":[{"type":"prism","pos":[0,1,0],"size":0.9,"length":1.0,"yaw":0.3,"material":"g"}]}"#).unwrap();
        let g = scene_wgsl(&sc);
        assert!(g.contains("d_prism(p,") && g.contains("occ_prism(ro,"), "{g}");
        crate::shaders::validate_for_tests("trace/prism", crate::shaders::trace(&sc));
    }

    #[test]
    fn material_code_carries_ior_and_dispersion() {
        let sc = crate::scene::parse(r#"{"materials":[{"name":"g","kind":"glass","ior":1.62,"dispersion":2.0},{"name":"w"}],
            "objects":[{"type":"sphere","pos":[0,1,0],"material":"g"},{"type":"sphere","pos":[3,1,0],"material":"w"}]}"#).unwrap();
        let g = scene_wgsl(&sc);
        assert!(g.contains("m.ior = 1.62; m.disp = 2.0;"), "{g}");
        assert!(g.contains("m.ior = 1.5; m.disp = 0.0;"));
        crate::shaders::validate_for_tests("trace/ior", crate::shaders::trace(&sc));
    }

    #[test]
    fn photon_shaders_generate_and_validate_for_the_prism_scene() {
        let sc = crate::scene::builtin(3);
        assert!(sc.photons());
        crate::shaders::validate_for_tests("photon", crate::shaders::photon(&sc));
        crate::shaders::validate_for_tests("caustic_resolve", crate::shaders::caustic_resolve(&sc));
        crate::shaders::validate_for_tests("trace/prism+map", crate::shaders::trace(&sc));
        crate::shaders::validate_for_tests("gi_trace/prism+map", crate::shaders::gi_trace(&sc));
    }

    #[test]
    fn scenes_without_photons_have_a_stub_lookup() {
        let sc = crate::scene::builtin(0);
        assert!(!sc.photons());
        assert!(caustic_at_wgsl(&sc, true).contains("return vec3<f32>(0.0);"));
        assert!(!caustic_at_wgsl(&sc, true).contains("CMAP"));
        crate::shaders::validate_for_tests("trace/no-photons", crate::shaders::trace(&sc));
    }

    #[test]
    fn photon_budget_and_fixed_point_cannot_overflow() {
        // worst case: sun colour 10 with the light setting at 2x, a prism bound radius of 1.5 m, every photon into one cell
        let e_perp = std::f32::consts::PI * 10.0 * 2.0;
        let beam_area = std::f32::consts::PI * 1.5 * 1.5;
        let per_photon = e_perp * beam_area / PHOTONS as f32;
        assert!((per_photon * FIXED) as f64 * PHOTONS as f64 * 2.0 <= u32::MAX as f64, "2x headroom lost");
    }

    #[test]
    fn receiver_offsets_are_packed_back_to_back() {
        let sc = crate::scene::builtin(3);
        let cells: Vec<u32> = sc.receivers.iter().map(|r| r.cells[0] * r.cells[1]).collect();
        assert_eq!(total_cells(&sc), cells.iter().sum::<u32>());
        assert_eq!(receiver_offset(&sc, 1), cells[0]);
        assert_eq!(receiver_offset(&sc, 0), 0);
    }
}
