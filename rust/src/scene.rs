//! Scene description: JSON -> `SceneData`; scenegen.rs turns it into shader code. Built-in scenes are JSON files too (rust/scenes/).
//! Format reference: docs/scenes.md.
use serde_json::Value;

use crate::camera::SceneCam;

/// Limits keep the generated shader small enough for mobile GPU compilers.
pub const MAX_OBJECTS: usize = 32;
pub const MAX_MATS: usize = 24;
pub const MAX_COMPLEX: usize = 6;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Sphere { r: f32 },
    Box { half: [f32; 3], round: f32 },
    Torus { big: f32, small: f32 },
    Menger { s: f32 },
    Cylinder { r: f32, h: f32 },
    Blobs { pos2: [f32; 3], r1: f32, r2: f32, k: f32 },
    Prism { side: f32, len: f32, yaw: f32 },
}

impl Shape {
    pub fn complex(&self) -> bool { !matches!(self, Shape::Sphere { .. } | Shape::Box { .. }) }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Obj {
    pub pos: [f32; 3],
    pub shape: Shape,
    pub spin: f32,
    pub mat: usize,
    /// glass box: tints the light passing through (shadow rays) instead of blocking it
    pub filter: bool,
    /// radius of a sphere around `pos` that contains the shape
    pub bound: f32,
}

/// kind: 0 diffuse, 1 metal, 2 glass (colour = tint), 3 emissive, 4 glossy (diffuse under a specular coat).
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: [f32; 3],
    pub kind: u8,
    pub rough: f32,
    pub emit: [f32; 3],
    /// (second colour, tile size)
    pub checker: Option<([f32; 3], f32)>,
    /// emissive: 0 fixed colour, 1 / 2 = the "Light A / B colour" setting
    pub slot: u8,
    /// glass: refractive index at 589 nm and dispersion strength (0 = none, 1 = crown glass)
    pub ior: f32,
    pub dispersion: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sun { pub dir: [f32; 3], /** radiance */ pub col: [f32; 3], pub cos: f32 }

#[derive(Debug, Clone, PartialEq)]
pub struct Lamp { pub pos: [f32; 3], pub r: f32, pub col: [f32; 3], pub slot: u8 }

pub const MAX_RECEIVERS: usize = 2;
pub const MAX_CELLS: u32 = 256;

/// A parallelogram (origin + u + v) that collects the photon map; the map has `cells` = [nu, nv] cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Receiver { pub origin: [f32; 3], pub u: [f32; 3], pub v: [f32; 3], pub cells: [u32; 2] }

#[derive(Debug)]
pub struct SceneData {
    pub name: String,
    pub cam: SceneCam,
    pub sun: Option<Sun>,
    pub lamps: Vec<Lamp>,
    pub mats: Vec<Material>,
    pub objs: Vec<Obj>,
    /// first glass sphere: gets the analytic caustic
    pub caustic: Option<([f32; 3], f32)>,
    pub receivers: Vec<Receiver>,
}

impl SceneData {
    pub fn prism(&self) -> Option<&Obj> { self.objs.iter().find(|o| matches!(o.shape, Shape::Prism { .. })) }

    /// Whether the photon passes are needed: a sun, a prism and somewhere for the light to land.
    pub fn photons(&self) -> bool { self.sun.is_some() && self.prism().is_some() && !self.receivers.is_empty() }
}

/// `scene` setting value for a user-loaded file (built-in scenes are 0..BUILTIN.len()).
pub const CUSTOM_SCENE: usize = 4;

/// Scene used until a settings push selects another (matches the `scene` default in params.rs).
pub const DEFAULT_SCENE: usize = 1;

pub const BUILTIN: [(&str, &str); 4] = [
    ("Classic", include_str!("../scenes/classic.json")),
    ("Sun room", include_str!("../scenes/sunroom.json")),
    ("Materials", include_str!("../scenes/materials.json")),
    ("Prism", include_str!("../scenes/prism.json")),
];

pub fn builtin(i: usize) -> SceneData {
    parse(BUILTIN[i.min(BUILTIN.len() - 1)].1).expect("built-in scene is valid")
}

fn vec3(v: &Value, what: &str) -> Result<[f32; 3], String> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| format!("{what}: expected [x, y, z]"))?;
    let mut o = [0.0; 3];
    for (i, x) in a.iter().enumerate() {
        let f = x.as_f64().ok_or_else(|| format!("{what}: not a number"))? as f32;
        if !f.is_finite() { return Err(format!("{what}: not finite")); }
        o[i] = f;
    }
    Ok(o)
}

fn opt3(o: &Value, key: &str, d: [f32; 3]) -> Result<[f32; 3], String> {
    match o.get(key) { Some(v) => vec3(v, key), None => Ok(d) }
}

fn num(o: &Value, key: &str, d: f32) -> Result<f32, String> {
    match o.get(key) {
        None => Ok(d),
        Some(v) => v.as_f64().map(|f| f as f32).filter(|f| f.is_finite()).ok_or_else(|| format!("{key}: expected a number")),
    }
}

fn norm(v: [f32; 3]) -> Result<[f32; 3], String> {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l < 1e-6 { return Err("zero-length direction".into()); }
    Ok([v[0] / l, v[1] / l, v[2] / l])
}

fn user_slot(o: &Value) -> Result<u8, String> {
    match o.get("user").and_then(|v| v.as_str()) {
        None => Ok(0),
        Some("a") => Ok(1),
        Some("b") => Ok(2),
        Some(s) => Err(format!("user: expected \"a\" or \"b\", got \"{s}\"")),
    }
}

/// Parses a scene; errors are human-readable (shown in the app).
pub fn parse(json: &str) -> Result<SceneData, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("JSON: {e}"))?;
    let name = root.get("name").and_then(|v| v.as_str()).unwrap_or("Scene").to_string();

    // camera
    let mut cam = SceneCam::default();
    if let Some(c) = root.get("camera") {
        cam.target = opt3(c, "target", cam.target)?;
        cam.yaw = num(c, "yaw", cam.yaw)?;
        cam.pitch = num(c, "pitch", cam.pitch)?.clamp(0.02, 1.45);
        cam.dist = num(c, "dist", cam.dist)?;
        if let Some(r) = c.get("dist_range") {
            let a = r.as_array().filter(|a| a.len() == 2).ok_or("dist_range: expected [min, max]")?;
            cam.dist_range = [a[0].as_f64().ok_or("dist_range: not a number")? as f32, a[1].as_f64().ok_or("dist_range: not a number")? as f32];
        }
        cam.fly_min = opt3(c, "fly_min", cam.fly_min)?;
        cam.fly_max = opt3(c, "fly_max", cam.fly_max)?;
        cam.orbit_min = opt3(c, "orbit_min", cam.fly_min)?;
        cam.orbit_max = opt3(c, "orbit_max", cam.fly_max)?;
        if cam.dist_range[0] <= 0.0 || cam.dist_range[0] > cam.dist_range[1] { return Err("dist_range: need 0 < min <= max".into()); }
        cam.dist = cam.dist.clamp(cam.dist_range[0], cam.dist_range[1]);
        for i in 0..3 {
            if cam.fly_min[i] > cam.fly_max[i] || cam.orbit_min[i] > cam.orbit_max[i] { return Err("camera bounds: min must not exceed max".into()); }
        }
    }

    // sun
    let mut sun = None;
    if let Some(sj) = root.get("sun") {
        if !sj.is_null() {
            let d = norm(vec3(sj.get("dir").ok_or("sun: missing dir")?, "sun.dir")?)?;
            let col = opt3(sj, "color", [1.0, 0.95, 0.85])?;
            let ang = num(sj, "angle_deg", 1.5)?.clamp(0.05, 20.0);
            let cos = ang.to_radians().cos();
            // `color` = radiance of a white diffuse surface facing the sun; the shader needs the sun's own radiance
            let k = std::f32::consts::PI / (2.0 * std::f32::consts::PI * (1.0 - cos));
            sun = Some(Sun { dir: d, col: [col[0] * k, col[1] * k, col[2] * k], cos });
        }
    }

    // lamps
    let mut lamps = vec![];
    if let Some(ls) = root.get("lamps") {
        let ls = ls.as_array().ok_or("lamps: expected a list")?;
        if ls.len() > 2 { return Err("lamps: at most 2".into()); }
        for l in ls {
            let pos = vec3(l.get("pos").ok_or("lamp: missing pos")?, "lamp.pos")?;
            let r = num(l, "radius", 0.5)?;
            if r <= 0.0 { return Err("lamp: radius must be positive".into()); }
            lamps.push(Lamp { pos, r, col: opt3(l, "color", [30.0, 30.0, 30.0])?, slot: user_slot(l)? });
        }
    }

    // materials
    let mj = root.get("materials").and_then(|v| v.as_array()).ok_or("materials: expected a list")?;
    if mj.is_empty() || mj.len() > MAX_MATS { return Err(format!("materials: 1..{MAX_MATS} required")); }
    let mut names: Vec<String> = vec![];
    let mut mats = vec![];
    for m in mj {
        let n = m.get("name").and_then(|v| v.as_str()).ok_or("material: missing name")?;
        if names.iter().any(|x| x == n) { return Err(format!("material \"{n}\" defined twice")); }
        names.push(n.to_string());
        let kind = match m.get("kind").and_then(|v| v.as_str()).unwrap_or("diffuse") {
            "diffuse" => 0, "metal" => 1, "glass" => 2, "emissive" => 3, "glossy" => 4,
            k => return Err(format!("material \"{n}\": unknown kind \"{k}\"")),
        };
        let checker = if m.get("checker").is_some() { Some((opt3(m, "checker", [0.0; 3])?, num(m, "checker_scale", 1.0)?.max(0.01))) } else { None };
        let ior = num(m, "ior", 1.5)?;
        if !(1.0..=3.0).contains(&ior) { return Err(format!("material \"{n}\": ior must be between 1.0 and 3.0")); }
        mats.push(Material {
            color: opt3(m, "color", [0.8, 0.8, 0.8])?, kind, rough: num(m, "rough", 0.0)?.clamp(0.0, 1.0),
            emit: opt3(m, "emit", [0.0; 3])?, checker, slot: user_slot(m)?, ior, dispersion: num(m, "dispersion", 0.0)?.clamp(0.0, 10.0),
        });
    }

    // objects
    let objs_j = root.get("objects").and_then(|v| v.as_array()).ok_or("objects: expected a list")?;
    if objs_j.is_empty() { return Err("objects: at least one required".into()); }
    if objs_j.len() > MAX_OBJECTS { return Err(format!("objects: at most {MAX_OBJECTS}")); }
    let mut objs = vec![];
    let mut caustic = None;
    for (i, o) in objs_j.iter().enumerate() {
        let ty = o.get("type").and_then(|v| v.as_str()).ok_or("object: missing type")?;
        let mname = o.get("material").and_then(|v| v.as_str()).ok_or_else(|| format!("object {i}: missing material"))?;
        let mi = names.iter().position(|x| x == mname).ok_or_else(|| format!("object {i}: unknown material \"{mname}\""))?;
        let pos = vec3(o.get("pos").ok_or_else(|| format!("object {i}: missing pos"))?, "pos")?;
        let spin = num(o, "spin", 0.0)?;
        let pair = |key: &str| -> Result<(f32, f32), String> {
            let r = o.get(key).and_then(|v| v.as_array()).filter(|a| a.len() == 2).ok_or_else(|| format!("object {i}: {ty} needs {key} [a, b]"))?;
            let (a, b) = (r[0].as_f64().unwrap_or(0.0) as f32, r[1].as_f64().unwrap_or(0.0) as f32);
            if a <= 0.0 || b <= 0.0 { return Err(format!("object {i}: {key} must be positive")); }
            Ok((a, b))
        };
        let (shape, bound) = match ty {
            "sphere" => {
                let r = num(o, "radius", 1.0)?;
                if r <= 0.0 { return Err(format!("object {i}: radius must be positive")); }
                (Shape::Sphere { r }, r)
            }
            "box" => {
                let h = vec3(o.get("size").ok_or_else(|| format!("object {i}: box needs size [half-x, half-y, half-z]"))?, "size")?;
                if h.iter().any(|v| *v <= 0.0) { return Err(format!("object {i}: size must be positive")); }
                let round = num(o, "round", 0.0)?.clamp(0.0, h[0].min(h[1]).min(h[2]));
                (Shape::Box { half: h, round }, (h[0] * h[0] + h[1] * h[1] + h[2] * h[2]).sqrt())
            }
            "torus" => { let (a, b) = pair("radii")?; (Shape::Torus { big: a, small: b }, a + b + 0.05) }
            "menger" => {
                let s = num(o, "size", 1.0)?;
                if s <= 0.0 { return Err(format!("object {i}: size must be positive")); }
                (Shape::Menger { s }, s * 1.8)
            }
            "prism" => {
                let side = num(o, "size", 1.0)?;
                let len = num(o, "length", 1.0)?;
                if side <= 0.0 || len <= 0.0 { return Err(format!("object {i}: prism size and length must be positive")); }
                let yaw = num(o, "yaw", 0.0)?;
                // circumradius of the triangle plus half the length: a sphere that contains the prism
                let r = side / 3f32.sqrt();
                (Shape::Prism { side, len, yaw }, (r * r + len * len * 0.25).sqrt() + 0.02)
            }
            "cylinder" => {
                let r = num(o, "radius", 0.5)?;
                let h = num(o, "half_height", 0.5)?;
                if r <= 0.0 || h <= 0.0 { return Err(format!("object {i}: radius and half_height must be positive")); }
                (Shape::Cylinder { r, h }, (r * r + h * h).sqrt() + 0.02)
            }
            "blobs" => {
                let pos2 = vec3(o.get("pos2").ok_or_else(|| format!("object {i}: blobs need pos2"))?, "pos2")?;
                let (a, b) = pair("radii")?;
                let k = num(o, "smooth", 0.35)?.max(0.01);
                let d = ((pos2[0] - pos[0]).powi(2) + (pos2[1] - pos[1]).powi(2) + (pos2[2] - pos[2]).powi(2)).sqrt();
                (Shape::Blobs { pos2, r1: a, r2: b, k }, d + a.max(b) + k)
            }
            t => return Err(format!("object {i}: unknown type \"{t}\"")),
        };
        // glass boxes are tinted-transparent for shadow rays; glass spheres block them (their light comes from the caustic term)
        let glass = mats[mi].kind == 2;
        let filter = glass && matches!(shape, Shape::Box { .. });
        if glass && caustic.is_none() { if let Shape::Sphere { r } = shape { caustic = Some((pos, r)); } }
        objs.push(Obj { pos, shape, spin, mat: mi, filter, bound });
    }
    if objs.iter().filter(|o| matches!(o.shape, Shape::Prism { .. })).count() > 1 { return Err("objects: only one prism is supported".into()); }
    if objs.iter().filter(|o| o.shape.complex()).count() > MAX_COMPLEX {
        return Err(format!("objects: at most {MAX_COMPLEX} of torus, menger, cylinder and blobs"));
    }
    let mut receivers = vec![];
    if let Some(rj) = root.get("caustic_receivers") {
        let rj = rj.as_array().ok_or("caustic_receivers: expected a list")?;
        if rj.len() > MAX_RECEIVERS { return Err(format!("caustic_receivers: at most {MAX_RECEIVERS}")); }
        for (i, r) in rj.iter().enumerate() {
            let origin = vec3(r.get("origin").ok_or_else(|| format!("receiver {i}: missing origin"))?, "origin")?;
            let u = vec3(r.get("u").ok_or_else(|| format!("receiver {i}: missing u"))?, "u")?;
            let v = vec3(r.get("v").ok_or_else(|| format!("receiver {i}: missing v"))?, "v")?;
            let cross = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            if cross.iter().map(|c| c * c).sum::<f32>().sqrt() < 1e-4 { return Err(format!("receiver {i}: u and v are parallel (zero area)")); }
            let c = r.get("cells").and_then(|c| c.as_array()).filter(|c| c.len() == 2).ok_or_else(|| format!("receiver {i}: cells must be [nu, nv]"))?;
            let cells = [c[0].as_u64().unwrap_or(0) as u32, c[1].as_u64().unwrap_or(0) as u32];
            if cells.iter().any(|n| *n == 0 || *n > MAX_CELLS) { return Err(format!("receiver {i}: cells must be 1..{MAX_CELLS}")); }
            receivers.push(Receiver { origin, u, v, cells });
        }
    }
    Ok(SceneData { name, cam, sun, lamps, mats, objs, caustic, receivers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse() {
        for (n, j) in BUILTIN {
            let s = parse(j).unwrap_or_else(|e| panic!("{n}: {e}"));
            assert!(!s.objs.is_empty(), "{n}");
        }
    }

    #[test]
    fn classic_matches_old_hardcoded_scene() {
        let s = builtin(0);
        assert_eq!(s.cam, SceneCam::default());
        assert_eq!((s.lamps[0].pos, s.lamps[0].r), ([-2.5, 5.0, 1.5], 0.8));
        assert_eq!((s.lamps[1].pos, s.lamps[1].r), ([4.0, 3.0, -3.5], 0.45));
        assert_eq!(s.caustic, Some(([1.2, 0.8, 1.0], 0.8)));
        assert!(s.sun.is_none());
    }

    #[test]
    fn sun_room_has_sun_and_tinted_glass() {
        let s = builtin(1);
        let sun = s.sun.as_ref().unwrap();
        let l = sun.dir.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((l - 1.0).abs() < 1e-4);
        assert!(s.objs.iter().any(|o| o.filter));
    }

    #[test]
    fn errors_are_readable() {
        let bad = |j: &str| parse(j).err().unwrap();
        assert!(bad("{").starts_with("JSON"));
        assert!(bad(r#"{"materials":[],"objects":[]}"#).contains("materials"));
        let m = r#""materials":[{"name":"a"}]"#;
        assert!(bad(&format!(r#"{{{m},"objects":[{{"type":"cone","pos":[0,0,0],"material":"a"}}]}}"#)).contains("cone"));
        assert!(bad(&format!(r#"{{{m},"objects":[{{"type":"sphere","pos":[0,0,0],"material":"zzz"}}]}}"#)).contains("zzz"));
        assert!(bad(&format!(r#"{{{m},"objects":[]}}"#)).contains("objects"));
        assert!(bad(r#"{"materials":[{"name":"a","kind":"plastic"}],"objects":[]}"#).contains("plastic"));
    }

    #[test]
    fn limits() {
        let one = |t: &str| format!(r#"{{"type":"{t}","pos":[0,0,0],"size":{},"material":"a"}}"#, if t == "menger" { "1" } else { "[1,1,1]" });
        let mk = |n: usize, t: &str| format!(r#"{{"materials":[{{"name":"a"}}],"objects":[{}]}}"#, (0..n).map(|_| one(t)).collect::<Vec<_>>().join(","));
        assert!(parse(&mk(MAX_OBJECTS + 1, "sphere")).unwrap_err().contains("at most"));
        assert!(parse(&mk(MAX_COMPLEX + 1, "menger")).unwrap_err().contains("at most"));
        assert!(parse(&mk(MAX_COMPLEX, "menger")).is_ok());
    }

    const PRISM_BASE: &str = r#"{"materials":[{"name":"g","kind":"glass","ior":1.6,"dispersion":2.5},{"name":"w"}],
        "sun":{"dir":[1,0.2,0],"color":[3,3,3]},
        "caustic_receivers":[{"origin":[-4,0,-4],"u":[8,0,0],"v":[0,0,8],"cells":[128,128]}],
        "objects":[{"type":"prism","pos":[0,1,0],"size":0.9,"length":1.0,"yaw":0.3,"material":"g"}]}"#;

    #[test]
    fn parses_prism_dispersion_and_receivers() {
        let s = parse(PRISM_BASE).unwrap();
        assert_eq!(s.mats[0].ior, 1.6);
        assert_eq!(s.mats[0].dispersion, 2.5);
        assert_eq!(s.mats[1].ior, 1.5);
        assert_eq!(s.mats[1].dispersion, 0.0);
        assert_eq!(s.receivers.len(), 1);
        assert_eq!(s.receivers[0].cells, [128, 128]);
        assert!(matches!(s.objs[0].shape, Shape::Prism { side, len, yaw } if side == 0.9 && len == 1.0 && yaw == 0.3));
        assert!(s.objs[0].bound > 0.72 && s.objs[0].bound < 0.8, "bound {}", s.objs[0].bound);
        assert!(s.prism().is_some() && s.photons());
        assert!(s.caustic.is_none(), "a prism does not use the analytic sphere caustic");
    }

    #[test]
    fn photons_need_sun_prism_and_receivers() {
        let no_sun = PRISM_BASE.replace(r#""sun":{"dir":[1,0.2,0],"color":[3,3,3]},"#, "");
        assert!(!parse(&no_sun).unwrap().photons());
        let no_rx = PRISM_BASE.replace(r#""caustic_receivers":[{"origin":[-4,0,-4],"u":[8,0,0],"v":[0,0,8],"cells":[128,128]}],"#, "");
        assert!(!parse(&no_rx).unwrap().photons());
    }

    #[test]
    fn bad_receivers_and_prisms_are_rejected_readably() {
        let rx = |s: &str| PRISM_BASE.replace(r#"{"origin":[-4,0,-4],"u":[8,0,0],"v":[0,0,8],"cells":[128,128]}"#, s);
        let bad = |j: String| parse(&j).err().unwrap();
        assert!(bad(rx(r#"{"origin":[0,0,0],"u":[1,0,0],"v":[2,0,0],"cells":[8,8]}"#)).contains("parallel"));
        assert!(bad(rx(r#"{"origin":[0,0,0],"u":[1,0,0],"v":[0,0,1],"cells":[0,8]}"#)).contains("cells"));
        assert!(bad(rx(r#"{"origin":[0,0,0],"u":[1,0,0],"v":[0,0,1],"cells":[512,8]}"#)).contains("cells"));
        assert!(bad(PRISM_BASE.replace(r#""size":0.9"#, r#""size":-1"#)).contains("size"));
        assert!(bad(PRISM_BASE.replace(r#""ior":1.6"#, r#""ior":0.5"#)).contains("ior"));
        let three = PRISM_BASE.replace(r#""caustic_receivers":["#, r#""caustic_receivers":[{"origin":[0,0,0],"u":[1,0,0],"v":[0,0,1],"cells":[8,8]},{"origin":[0,0,0],"u":[1,0,0],"v":[0,0,1],"cells":[8,8]},"#);
        assert!(bad(three).contains("at most"));
    }

    #[test]
    fn only_one_prism() {
        let two = PRISM_BASE.replace(r#""objects":["#, r#""objects":[{"type":"prism","pos":[2,1,0],"size":0.5,"length":1,"material":"g"},"#);
        assert!(parse(&two).unwrap_err().contains("one prism"));
    }

    fn rot_y(v: [f32; 3], a: f32) -> [f32; 3] { let (s, c) = a.sin_cos(); [c * v[0] + s * v[2], v[1], -s * v[0] + c * v[2]] }
    fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

    /// Same algorithm as photon.wgsl, on the CPU, for one wavelength: returns the exit (position, direction) in world space.
    fn trace_prism(sc: &SceneData, lambda: f32) -> Option<([f32; 3], [f32; 3])> {
        let p = sc.prism()?;
        let Shape::Prism { side, len, yaw } = p.shape else { return None };
        let m = &sc.mats[p.mat];
        let n = crate::dispersion::ior(lambda, m.ior, m.dispersion);
        let sun = sc.sun.as_ref()?;
        let d_world = [-sun.dir[0], -sun.dir[1], -sun.dir[2]];
        let start = [p.pos[0] - 3.0 * d_world[0], p.pos[1] - 3.0 * d_world[1], p.pos[2] - 3.0 * d_world[2]];
        let r = side * 0.28867513;
        let planes: [([f32; 3], f32); 5] = [([-0.8660254, 0.5, 0.0], r), ([0.8660254, 0.5, 0.0], r), ([0.0, -1.0, 0.0], r), ([0.0, 0.0, -1.0], len / 2.0), ([0.0, 0.0, 1.0], len / 2.0)];
        let clip = |o: [f32; 3], d: [f32; 3]| -> Option<(f32, f32, usize, usize)> {
            let (mut tin, mut tout, mut fin, mut fout) = (-1e9f32, 1e9f32, 0, 0);
            for (k, (nrm, off)) in planes.iter().enumerate() {
                let den = dot3(*nrm, d);
                let num = off - dot3(*nrm, o);
                if den.abs() < 1e-8 { if num < 0.0 { return None; } continue; }
                let t = num / den;
                if den < 0.0 { if t > tin { tin = t; fin = k; } } else if t < tout { tout = t; fout = k; }
            }
            if tout > tin { Some((tin, tout, fin, fout)) } else { None }
        };
        let refract = |i: [f32; 3], nn: [f32; 3], eta: f32| -> [f32; 3] {
            let cosi = -dot3(nn, i);
            let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
            [eta * i[0] + (eta * cosi - k.sqrt()) * nn[0], eta * i[1] + (eta * cosi - k.sqrt()) * nn[1], eta * i[2] + (eta * cosi - k.sqrt()) * nn[2]]
        };
        let mut o = rot_y([start[0] - p.pos[0], start[1] - p.pos[1], start[2] - p.pos[2]], yaw);
        let mut d = rot_y(d_world, yaw);
        let (tin, _, fin, _) = clip(o, d)?;
        if tin < 0.0 { return None; }
        let nin = planes[fin].0;
        o = [o[0] + d[0] * tin - nin[0] * 1e-4, o[1] + d[1] * tin - nin[1] * 1e-4, o[2] + d[2] * tin - nin[2] * 1e-4];
        d = refract(d, nin, 1.0 / n);
        for _ in 0..5 {
            let (_, tout, _, fout) = clip(o, d)?;
            let nout = planes[fout].0;
            let pos = [o[0] + d[0] * tout, o[1] + d[1] * tout, o[2] + d[2] * tout];
            let cosi = dot3(d, nout);
            let sint = n * (1.0 - cosi * cosi).max(0.0).sqrt();
            if sint > 1.0 {
                let k = 2.0 * cosi;
                d = [d[0] - k * nout[0], d[1] - k * nout[1], d[2] - k * nout[2]];
                o = [pos[0] - nout[0] * 1e-4, pos[1] - nout[1] * 1e-4, pos[2] - nout[2] * 1e-4];
            } else {
                let dd = refract(d, [-nout[0], -nout[1], -nout[2]], n);
                let ow = rot_y([pos[0] + nout[0] * 2e-3, pos[1] + nout[1] * 2e-3, pos[2] + nout[2] * 2e-3], -yaw);
                return Some(([ow[0] + p.pos[0], ow[1] + p.pos[1], ow[2] + p.pos[2]], rot_y(dd, -yaw)));
            }
        }
        None
    }

    fn receiver_hit(sc: &SceneData, o: [f32; 3], d: [f32; 3]) -> Option<usize> {
        for (i, r) in sc.receivers.iter().enumerate() {
            let n = [r.u[1] * r.v[2] - r.u[2] * r.v[1], r.u[2] * r.v[0] - r.u[0] * r.v[2], r.u[0] * r.v[1] - r.u[1] * r.v[0]];
            let den = dot3(n, d);
            if den.abs() < 1e-6 { continue; }
            let t = dot3(n, [r.origin[0] - o[0], r.origin[1] - o[1], r.origin[2] - o[2]]) / den;
            if t < 1e-3 { continue; }
            let q = [o[0] + d[0] * t - r.origin[0], o[1] + d[1] * t - r.origin[1], o[2] + d[2] * t - r.origin[2]];
            let (a, b) = (dot3(q, r.u) / dot3(r.u, r.u), dot3(q, r.v) / dot3(r.v, r.v));
            if (0.0..1.0).contains(&a) && (0.0..1.0).contains(&b) { return Some(i); }
        }
        None
    }

    #[test]
    fn prism_is_builtin_three_and_files_are_four() {
        assert_eq!(BUILTIN.len(), 4);
        assert_eq!(BUILTIN[3].0, "Prism");
        assert_eq!(CUSTOM_SCENE, 4);
        let s = builtin(3);
        assert!(s.photons(), "the Prism scene has sun, prism and receivers");
        let m = &s.mats[s.prism().unwrap().mat];
        assert!(m.dispersion >= 2.0 && m.kind == 2);
    }

    #[test]
    fn prism_scene_sends_a_visible_fan_onto_a_receiver() {
        let s = builtin(3);
        let hits: Vec<_> = [400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0].iter().map(|l| {
            let (o, d) = trace_prism(&s, *l).unwrap_or_else(|| panic!("{l} nm does not leave the prism"));
            let rx = receiver_hit(&s, o, d).unwrap_or_else(|| panic!("{l} nm misses every receiver"));
            let t = {
                let r = &s.receivers[rx];
                let n = [r.u[1] * r.v[2] - r.u[2] * r.v[1], r.u[2] * r.v[0] - r.u[0] * r.v[2], r.u[0] * r.v[1] - r.u[1] * r.v[0]];
                dot3(n, [r.origin[0] - o[0], r.origin[1] - o[1], r.origin[2] - o[2]]) / dot3(n, d)
            };
            [o[0] + d[0] * t, o[1] + d[1] * t, o[2] + d[2] * t]
        }).collect();
        let spread = ((hits[0][0] - hits[6][0]).powi(2) + (hits[0][2] - hits[6][2]).powi(2)).sqrt();
        assert!(spread > 0.15, "violet and red land only {spread} m apart: {hits:?}");
    }
}
