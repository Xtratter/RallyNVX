//! Offline converter: a Sketchfab glb of a car -> a compact `.rcar` (body and one wheel as simplified, vertex-coloured
//! meshes in the renderer's vertex format). Usage: carconv <in.glb> <out.rcar> <body_tris> <wheel_tris>
mod atlas;
use std::collections::HashMap;

type M4 = [[f32; 4]; 4];
fn mul(a: &M4, b: &M4) -> M4 { let mut r = [[0.0; 4]; 4]; for c in 0..4 { for rw in 0..4 { r[c][rw] = (0..4).map(|k| a[k][rw] * b[c][k]).sum(); } } r }
fn tp(m: &M4, v: [f32; 3]) -> [f32; 3] { [m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0], m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1], m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2]] }
fn tn(m: &M4, v: [f32; 3]) -> [f32; 3] {
    let r = [m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2], m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2], m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2]];
    let l = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt().max(1e-9); [r[0] / l, r[1] / l, r[2] / l]
}

const PAINT: u8 = 0; const GLASS: u8 = 1; const RUBBER: u8 = 2; const PLASTIC: u8 = 3; const CHROME: u8 = 4; const EMISSIVE: u8 = 5;
// the reflector interiors of the lamp units: lit by the car's state in the shader (8 = position lamp and 9, 10 = covers are made in the game); 11 = front, 12 = rear
const INT_FRONT: u8 = 11; const INT_REAR: u8 = 12;

#[derive(Clone, Copy)]
struct V { p: [f32; 3], n: [f32; 3], c: [u8; 4], uv: [f32; 2], tex: Option<usize> }

fn srgb(u: u8) -> f32 { let c = u as f32 / 255.0; if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } }
fn q(f: f32) -> u8 { (f.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 }

fn keep_body(n: &str) -> bool {
    // the underbody and inner arches close the shell, so nothing is seen through it
    let closers = ["x0_under_f", "x0_under_b", "x0_main_body_undercarriage", "x0_wing_fl_int", "x0_wing_fr_int", "x0_bumper_fm_int", "x0_bumper_bm_int"];
    if closers.iter().any(|c| n == *c) { return true; }
    // the lamp units: the reflector interiors and the plastics are kept; the clear glass covers are left out (a cover is opaque here and would hide the reflector)
    let lamp_unit = ["Light_front", "Ligth_front", "Ligth_rear", "Light_bar", "Light_2_brake", "Ligth_2_brake"].iter().any(|p| n.starts_with(p));
    if lamp_unit {
        let glass = n.ends_with("_glass") || (n.starts_with("Light_front") && n.ends_with("_turn")) || n.contains("Light_bar_glass");
        return !glass;
    }
    let bad = ["_int", "int_", "lock", "under", "engine", "snow", "gravel", "spare", "suspension", "wishbone", "disc", "caliper", "mud", "CO_DRIVER", "Wipers", "LED", "notes", "Sabo", "INT_", "boot_int", "undercarriage", "standard_pbr", "anagram", "exhaust_cardan", "_ints", "Light_bar_ints", "tow_hook"];
    if bad.iter().any(|b| n.contains(b)) { return false; }
    let good = ["x0_main_body", "x0_bonnet", "x0_boot", "x0_bumper_fm", "x0_bumper_bm", "x0_spoiler_b", "x0_door_f", "x0_wing_f", "x0_wing_mirror", "x0_window_", "Light_front", "Ligth_front", "Ligth_rear", "Light_bar", "Light_2_brake", "Ligth_2_brake", "x0_exhaust_bl", "x0_f_body_part", "x0_front_anagram"];
    good.iter().any(|g| n.starts_with(g))
}

fn lamp_colour(n: &str) -> [f32; 3] {
    let l = n.to_lowercase();
    if l.contains("brake") || l.contains("fog") { [0.95, 0.05, 0.03] } else if l.contains("turn") { [1.0, 0.55, 0.10] } else if l.contains("reverse") { [1.0, 1.0, 1.0] } else { [1.0, 0.95, 0.80] }
}

fn class_of(mat: &str, node: &str) -> (u8, bool) {
    let m = mat.to_lowercase();
    if m == "lamps" || node.ends_with("_lamp") || node.contains("_lamp.") || node == "Light_bar_lamps" { return (EMISSIVE, true); }
    if m.contains("glass") || m == "rearglass" { return (GLASS, true); }
    if m == "ligths_ints" { return (if node.contains("front") || node.starts_with("Light_bar") { INT_FRONT } else { INT_REAR }, false); }
    if m == "crapaint_ext" { return (PAINT, false); }
    if m.contains("chrome") || m == "mirrors" || m == "rims_paint" || m == "tarmac_wheel" || m == "rim_gravel" { return (CHROME, false); }
    if m == "rubbers" || m.starts_with("mud") || m.starts_with("ext_tyre") { return (RUBBER, false); }
    (PLASTIC, false)
}

struct Textures<'a> { doc: &'a gltf::Document, buffers: &'a [gltf::buffer::Data], cache: HashMap<usize, Option<image::RgbaImage>> }
impl<'a> Textures<'a> {
    fn get(&mut self, idx: usize) -> Option<&image::RgbaImage> {
        if !self.cache.contains_key(&idx) {
            let img = self.doc.images().nth(idx).and_then(|im| match im.source() {
                gltf::image::Source::View { view, .. } => {
                    let b = &self.buffers[view.buffer().index()].0;
                    image::load_from_memory(&b[view.offset()..view.offset() + view.length()]).ok().map(|d| d.to_rgba8())
                }
                _ => None,
            });
            eprintln!("  texture {idx}: {:?}", img.as_ref().map(|i| i.dimensions()));
            self.cache.insert(idx, img);
        }
        self.cache.get(&idx).and_then(|o| o.as_ref())
    }
}
fn sample(img: &image::RgbaImage, uv: [f32; 2]) -> [f32; 3] {
    let (w, h) = img.dimensions();
    let (u, v) = (uv[0] - uv[0].floor(), uv[1] - uv[1].floor());
    let (x, y) = (((u * w as f32) as u32).min(w - 1), ((v * h as f32) as u32).min(h - 1));
    // a 5x5 box average takes the grain out of baked textures
    let mut acc = [0.0f32; 3]; let mut n = 0.0;
    for dy in -2i32..=2 { for dx in -2i32..=2 {
        let (sx, sy) = (((x as i32 + dx * 3).clamp(0, w as i32 - 1)) as u32, ((y as i32 + dy * 3).clamp(0, h as i32 - 1)) as u32);
        let p = img.get_pixel(sx, sy).0; acc[0] += srgb(p[0]); acc[1] += srgb(p[1]); acc[2] += srgb(p[2]); n += 1.0;
    } }
    [acc[0] / n, acc[1] / n, acc[2] / n]
}

fn walk(doc: &gltf::Document, buffers: &[gltf::buffer::Data], tex: &mut Textures, node: gltf::Node, parent: &M4, want: &dyn Fn(&str) -> bool, atlas: bool, out: &mut Vec<(String, Vec<V>, Vec<u32>)>) {
    let m = mul(parent, &node.transform().matrix());
    let name = node.name().unwrap_or("").replace(".001", "");
    if let (Some(mesh), true) = (node.mesh(), want(&name)) {
        let mut verts: Vec<V> = Vec::new(); let mut idx: Vec<u32> = Vec::new();
        for prim in mesh.primitives() {
            let r = prim.reader(|b| Some(&buffers[b.index()].0));
            let pos: Vec<[f32; 3]> = r.read_positions().map(|i| i.collect()).unwrap_or_default();
            let nrm: Vec<[f32; 3]> = r.read_normals().map(|i| i.collect()).unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; pos.len()]);
            let uvs: Vec<[f32; 2]> = r.read_tex_coords(0).map(|i| i.into_f32().collect()).unwrap_or_else(|| vec![[0.0, 0.0]; pos.len()]);
            let mat = prim.material(); let pbr = mat.pbr_metallic_roughness();
            let (class, flat) = class_of(mat.name().unwrap_or(""), &name);
            let factor = pbr.base_color_factor();
            let tex_idx = pbr.base_color_texture().map(|t| t.texture().source().index());
            let base = verts.len() as u32;
            for i in 0..pos.len() {
                // body paint, plastic, chrome and rubber keep their texture (the atlas); lamps, glass and the wheels get a colour
                let textured = atlas && class != EMISSIVE && class != GLASS && tex_idx.is_some();
                let col = if class == EMISSIVE { lamp_colour(&name) } else if class == GLASS { [0.03, 0.05, 0.08] } else if textured { [factor[0], factor[1], factor[2]] } else if let Some(ti) = tex_idx {
                    match tex.get(ti) { Some(img) => { let s = sample(img, uvs[i]); [s[0] * factor[0], s[1] * factor[1], s[2] * factor[2]] } None => [factor[0], factor[1], factor[2]] }
                } else { [factor[0], factor[1], factor[2]] };
                let _ = flat;
                let uv = [uvs[i][0] - uvs[i][0].floor(), uvs[i][1] - uvs[i][1].floor()];
                verts.push(V { p: tp(&m, pos[i]), n: tn(&m, nrm[i]), c: [q(col[0]), q(col[1]), q(col[2]), class], uv, tex: if textured { tex_idx } else { None } });
            }
            if let Some(ind) = r.read_indices() { idx.extend(ind.into_u32().map(|i| i + base)); }
        }
        out.push((name.clone(), verts, idx));
    }
    for c in node.children() { walk(doc, buffers, tex, c, &m, want, atlas, out); }
}

fn collect(doc: &gltf::Document, buffers: &[gltf::buffer::Data], tex: &mut Textures, want: &dyn Fn(&str) -> bool, atlas: bool) -> Vec<(String, Vec<V>, Vec<u32>)> {
    let id: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
    let mut out = Vec::new();
    for s in doc.scenes() { for n in s.nodes() { walk(doc, buffers, tex, n, &id, want, atlas, &mut out); } }
    out
}

fn merge(parts: &[(String, Vec<V>, Vec<u32>)]) -> (Vec<V>, Vec<u32>) {
    let (mut v, mut i) = (Vec::new(), Vec::new());
    for (_, pv, pi) in parts { let b = v.len() as u32; v.extend_from_slice(pv); i.extend(pi.iter().map(|x| x + b)); }
    (v, i)
}

fn simplify(v: &[V], idx: &[u32], target_tris: usize) -> (Vec<V>, Vec<u32>) {
    if idx.len() / 3 <= target_tris { return compact(v, idx); }
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.p.iter().flat_map(|f| f.to_le_bytes())).collect();
    let adapter = meshopt::VertexDataAdapter::new(&bytes, 12, 0).unwrap();
    let attrs: Vec<f32> = v.iter().flat_map(|x| [x.n[0], x.n[1], x.n[2], x.c[0] as f32 / 255.0, x.c[1] as f32 / 255.0, x.c[2] as f32 / 255.0, x.uv[0], x.uv[1]]).collect();
    let mut err = 0.0f32;
    let lock = vec![false; v.len()];
    let out = meshopt::simplify_with_attributes_and_locks(idx, &adapter, &attrs, &[0.6, 0.6, 0.6, 0.4, 0.4, 0.4, 1.0, 1.0], 32, &lock, target_tris * 3, 0.5, meshopt::SimplifyOptions::None, Some(&mut err));
    eprintln!("  simplified {} -> {} tris (error {:.4})", idx.len() / 3, out.len() / 3, err);
    compact(v, &out)
}

fn compact(v: &[V], idx: &[u32]) -> (Vec<V>, Vec<u32>) {
    let mut map: HashMap<u32, u32> = HashMap::new(); let mut nv = Vec::new(); let mut ni = Vec::new();
    for &i in idx { let n = *map.entry(i).or_insert_with(|| { nv.push(v[i as usize]); (nv.len() - 1) as u32 }); ni.push(n); }
    (nv, ni)
}

fn bounds(v: &[V]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3]; let mut hi = [f32::MIN; 3];
    for x in v { for k in 0..3 { lo[k] = lo[k].min(x.p[k]); hi[k] = hi[k].max(x.p[k]); } }
    (lo, hi)
}

fn write(out: &mut Vec<u8>, v: &[V], idx: &[u32]) {
    out.extend_from_slice(&(v.len() as u32).to_le_bytes()); out.extend_from_slice(&(idx.len() as u32).to_le_bytes());
    for x in v {
        for f in x.p.iter().chain(x.n.iter()) { out.extend_from_slice(&f.to_le_bytes()); }
        out.extend_from_slice(&x.c);
        let uv: [u16; 2] = if x.tex.is_some() { [(x.uv[0].clamp(0.0, 1.0) * 65534.0).round() as u16, (x.uv[1].clamp(0.0, 1.0) * 65534.0).round() as u16] } else { [0xFFFF, 0xFFFF] };
        out.extend_from_slice(&uv[0].to_le_bytes()); out.extend_from_slice(&uv[1].to_le_bytes());
    }
    for i in idx { out.extend_from_slice(&i.to_le_bytes()); }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 6 { eprintln!("usage: carconv <in.glb> <out.rcar> <body_tris> <wheel_tris> <livery.jpg>"); std::process::exit(2); }
    let (body_t, wheel_t): (usize, usize) = (a[3].parse().unwrap(), a[4].parse().unwrap());
    let gltf = gltf::Gltf::open(&a[1]).expect("open glb");
    let buffers = gltf::import_buffers(&gltf.document, None, gltf.blob.clone()).expect("buffers");
    let doc = &gltf.document;
    let mut tex = Textures { doc, buffers: &buffers, cache: HashMap::new() };
    // wheel centres and radii from the tarmac tyres
    let tyres = collect(doc, &buffers, &mut tex, &|n| n.starts_with("tyre_tarmac_") && !n.contains("spare"), false);
    for (n, v, _) in &tyres { let (lo, hi) = bounds(v); eprintln!("{n}: centre ({:.3}, {:.3}, {:.3}) radius {:.3} width {:.3}", (lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0, (hi[1] - lo[1]) / 2.0, hi[0] - lo[0]); }
    let ground = tyres.iter().map(|(_, v, _)| bounds(v).0[1]).fold(f32::MAX, f32::min);
    eprintln!("ground y = {ground:.3}");
    // body
    let parts = collect(doc, &buffers, &mut tex, &keep_body, true);
    let (mut bv, bi) = merge(&parts);
    for v in &mut bv { v.p[1] -= ground; }
    // the model sits nose-high by the difference of its wheel centres: level it about the rear axle
    let cen = |name: &str| tyres.iter().find(|t| t.0 == name).map(|t| { let (lo, hi) = bounds(&t.1); [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0 - ground, (lo[2] + hi[2]) / 2.0] }).unwrap();
    let (cf, cr) = (cen("tyre_tarmac_lf"), cen("tyre_tarmac_lr"));
    let th = ((cf[1] - cr[1]) / (cf[2] - cr[2])).atan();
    let (st, ct) = (th.sin(), th.cos());
    for v in &mut bv {
        let (y, z) = (v.p[1] - cr[1], v.p[2] - cr[2]);
        v.p[1] = y * ct - z * st + cr[1]; v.p[2] = y * st + z * ct + cr[2];
        let (ny, nz) = (v.n[1], v.n[2]); v.n[1] = ny * ct - nz * st; v.n[2] = ny * st + nz * ct;
    }
    eprintln!("levelled by {:.4} rad; wheel centres after levelling: front y {:.3}, rear y {:.3} (radius {:.3})", th, cr[1], cr[1], 0.326);
    eprintln!("body parts {} with {} vertices, {} triangles", parts.len(), bv.len(), bi.len() / 3);
    let (lo, hi) = bounds(&bv); eprintln!("body bounds: x {:.3}..{:.3} y {:.3}..{:.3} z {:.3}..{:.3}", lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]);
    // the livery atlas: the body's textures packed into one square, the vertices' UVs remapped into their tiles
    const ATLAS: u32 = 2048;
    // textures used by only a few vertices (a roof light bar, small details) are not worth an atlas tile: they keep a baked colour
    {
        let mut count: HashMap<usize, usize> = HashMap::new();
        for v in &bv { if let Some(t) = v.tex { *count.entry(t).or_insert(0) += 1; } }
        let total: usize = count.values().sum();
        let small: HashMap<usize, bool> = count.keys().map(|&t| (t, tex.get(t).map_or(true, |im| im.width().max(im.height()) <= 512))).collect();
        for v in &mut bv {
            if let Some(t) = v.tex {
                // a big texture needs at least a tenth of the textured vertices to earn its tile
                if count[&t] < if small[&t] { 300 } else { total / 10 } {
                    if let Some(img) = tex.get(t) { let c = sample(img, v.uv); v.c = [q(c[0]), q(c[1]), q(c[2]), v.c[3]]; }
                    v.tex = None;
                }
            }
        }
    }
    // textures with identical pixels (the livery is there twice) are packed once
    let mut first: HashMap<u64, usize> = HashMap::new(); let mut alias: HashMap<usize, usize> = HashMap::new();
    for i in { let mut u: Vec<usize> = bv.iter().filter_map(|v| v.tex).collect(); u.sort(); u.dedup(); u } {
        if let Some(im) = tex.get(i) {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new(); im.dimensions().hash(&mut h); im.as_raw().hash(&mut h);
            alias.insert(i, *first.entry(h.finish()).or_insert(i));
        }
    }
    for v in &mut bv { if let Some(t) = v.tex { v.tex = Some(*alias.get(&t).unwrap_or(&t)); } }
    let mut used: Vec<usize> = bv.iter().filter_map(|v| v.tex).collect(); used.sort(); used.dedup();
    let sizes: Vec<(usize, u32, u32)> = used.iter().filter_map(|&i| tex.get(i).map(|im| (i, im.width(), im.height()))).collect();
    let rects = atlas::pack(&sizes, ATLAS);
    let mut atlas_img = image::RgbImage::from_pixel(ATLAS, ATLAS, image::Rgb([128, 128, 128]));
    for r in &rects {
        let src = tex.get(r.id).unwrap();
        let small = image::imageops::resize(src, r.w, r.h, image::imageops::FilterType::Lanczos3);
        for y in 0..r.h { for x in 0..r.w { let p = small.get_pixel(x, y).0; atlas_img.put_pixel(r.x + x, r.y + y, image::Rgb([p[0], p[1], p[2]])); } }
        eprintln!("  atlas: image {} -> {}x{} at ({}, {})", r.id, r.w, r.h, r.x, r.y);
    }
    for v in &mut bv {
        if let Some(r) = v.tex.and_then(|t| rects.iter().find(|r| r.id == t)) {
            let (iw, ih) = (r.w.saturating_sub(2).max(1) as f32, r.h.saturating_sub(2).max(1) as f32);
            v.uv = [(r.x as f32 + 1.0 + v.uv[0] * iw) / ATLAS as f32, (r.y as f32 + 1.0 + v.uv[1] * ih) / ATLAS as f32];
        }
    }
    {
        let f = std::fs::File::create(&a[5]).expect("create livery jpg");
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), 90).encode_image(&atlas_img).expect("write jpg");
    }
    let (sv, si) = simplify(&bv, &bi, body_t);
    // one wheel (left front): tyre + rim, centred
    let wparts = collect(doc, &buffers, &mut tex, &|n| n == "tyre_tarmac_lf" || n == "x0_wheel_fl_5radios" || n == "x0_disc_fl" || n == "x0_hub_caliper_fl", false);
    let (mut wv, wi) = merge(&wparts);
    let (wl, wh) = bounds(&wv); let c = [(wl[0] + wh[0]) / 2.0, (wl[1] + wh[1]) / 2.0, (wl[2] + wh[2]) / 2.0];
    for v in &mut wv { for k in 0..3 { v.p[k] -= c[k]; } }
    eprintln!("wheel: {} parts, {} tris, centre ({:.3}, {:.3}, {:.3}) radius {:.3}", wparts.len(), wi.len() / 3, c[0], c[1] - ground, c[2], (wh[1] - wl[1]) / 2.0);
    let (swv, swi) = simplify(&wv, &wi, wheel_t);
    let mut out = b"RCA2".to_vec();
    write(&mut out, &sv, &si); write(&mut out, &swv, &swi);
    std::fs::write(&a[2], &out).unwrap();
    eprintln!("wrote {} bytes: body {} tris / {} verts, wheel {} tris / {} verts", out.len(), si.len() / 3, sv.len(), swi.len() / 3, swv.len());
}
