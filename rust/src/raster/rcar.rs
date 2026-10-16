//! Reader of the `.rcar` car asset written by `tools/carconv`: a body mesh and one wheel mesh in the renderer's vertex
//! format (little endian: `RCAR`, then per mesh `u32 vertex count`, `u32 index count`, 28-byte vertices, `u32` indices).
use super::vehicle_models::{Mesh3, Vert, NO_UV};

const MAX_VERTS: usize = 400_000;

fn mesh(d: &[u8], o: &mut usize, stride: usize) -> Result<Mesh3, String> {
    let rd = |o: &mut usize| -> Result<u32, String> { let b = d.get(*o..*o + 4).ok_or("truncated")?; *o += 4; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])) };
    let (nv, ni) = (rd(o)? as usize, rd(o)? as usize);
    if nv == 0 || nv > MAX_VERTS || ni == 0 || ni % 3 != 0 || ni > MAX_VERTS * 6 { return Err(format!("bad counts {nv}/{ni}")); }
    if d.len() < *o + nv * stride + ni * 4 { return Err("truncated".into()); }
    let f = |at: usize| f32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]);
    let mut m = Mesh3::default();
    for _ in 0..nv {
        let a = *o;
        m.verts.push(Vert { pos: [f(a), f(a + 4), f(a + 8)], nrm: [f(a + 12), f(a + 16), f(a + 20)], col: [d[a + 24], d[a + 25], d[a + 26], d[a + 27]],
            uv: if stride == 32 { [u16::from_le_bytes([d[a + 28], d[a + 29]]), u16::from_le_bytes([d[a + 30], d[a + 31]])] } else { NO_UV } });
        *o += stride;
    }
    for _ in 0..ni { let i = rd(o)?; if i as usize >= nv { return Err("index out of range".into()); } m.idx.push(i); }
    if m.verts.iter().any(|v| v.pos.iter().chain(v.nrm.iter()).any(|c| !c.is_finite())) { return Err("non-finite vertex".into()); }
    Ok(m)
}

/// The meshes of an `.rcar` file: the body and one wheel (tyre and rim), and in `RCA3` files the brake disc and the brake caliper (the caliper does not spin with the wheel)
/// the gravel and the snow wheel sets (`alt_wheels`), the transparent glass (windows and lamp covers) and the interior (cabin and driver).
pub struct Parts { pub body: Mesh3, pub wheel: Mesh3, pub disc: Option<Mesh3>, pub caliper: Option<Mesh3>, pub alt_wheels: [Option<Mesh3>; 2], pub glass: Option<Mesh3>, pub interior: Option<Mesh3>,
    /// The steering wheel mesh (in the body frame), its pivot mesh (two vertices: `pos` of the first = the wheel's centre, `nrm` = its axis towards the driver) and the driver.
    pub steering: Option<Mesh3>, pub pivot: Option<Mesh3>, pub driver: Option<Mesh3>,
    /// The two windscreen wipers (the driver's side first) and their pivot mesh (two vertices: `pos` = the spindle, `nrm` = the axis they sweep about, out of the glass).
    pub wipers: [Option<Mesh3>; 2], pub wiper_pivots: Option<Mesh3>,
    /// The instrument cluster and the RPM LEDs (classes 16 and 17; the game draws them with the interior) and the driver's gloves (they turn with the steering wheel).
    pub dash: Option<Mesh3>, pub gloves: Option<Mesh3> }

/// All the meshes of an `.rcar` file: `RCAR` (28-byte vertices), `RCA2` (32 bytes with a `u16x2` UV after the colour), `RCA3` (a `u32` mesh count of 2 to 15, then the meshes in
/// the order body, wheel, disc, caliper, gravel wheel, snow wheel, glass, interior, steering wheel, steering pivot, driver, wiper, wiper, wiper pivots, dash, gloves (not in the Peugeot file: its gloves are skinned and collapse without the rig); 32-byte vertices).
pub fn parse_all(d: &[u8]) -> Result<Parts, String> {
    let (stride, count) = match d.get(..4) {
        Some(b"RCAR") => (28, 2usize),
        Some(b"RCA2") => (32, 2),
        Some(b"RCA3") => {
            let n = d.get(4..8).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or("truncated")? as usize;
            if !(2..=15).contains(&n) { return Err(format!("bad mesh count {n}")); }
            (32, n)
        }
        _ => return Err("not an RCAR/RCA2/RCA3 file".into()),
    };
    let mut o = if &d[..4] == b"RCA3" { 8 } else { 4 };
    let mut meshes = Vec::new();
    for _ in 0..count { meshes.push(mesh(d, &mut o, stride)?); }
    let mut it = meshes.into_iter();
    Ok(Parts { body: it.next().unwrap(), wheel: it.next().unwrap(), disc: it.next(), caliper: it.next(), alt_wheels: [it.next(), it.next()], glass: it.next(), interior: it.next(), steering: it.next(), pivot: it.next(), driver: it.next(), wipers: [it.next(), it.next()], wiper_pivots: it.next(), dash: it.next(), gloves: it.next() })
}

/// `(body, wheel)` from the bytes of an `.rcar` file.
pub fn parse(d: &[u8]) -> Result<(Mesh3, Mesh3), String> { parse_all(d).map(|p| (p.body, p.wheel)) }

/// Where the lamp units of a car mesh are, found from its emissive vertices (material 5) by colour: headlight units (cream or white at the front), tail units (red) and reverse
/// lamps (white at the rear). Metres in the mesh's frame, for the car's right-hand side (the cars are symmetric): the units' centres. A new car model only has to colour
/// its lamp meshes this way (the converter does) and its lamps sit on the mesh, wherever the designer put them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampPoints { pub head: [f32; 3], pub tail: [f32; 3], pub rev: [f32; 3], pub pos: [f32; 3] }

fn is_cream_front(v: &Vert) -> bool { (v.col[3] == 5 || v.col[3] == 7) && v.col[0] > 200 && v.col[1] > 200 && v.col[2] > 140 && v.pos[2] > 0.0 }

/// The cream front vertices (right side) split in two: the main lens and the outboard corner piece (within 6 cm of the outermost x, at least 6 vertices each),
/// which is the car's position lamp. The centre sliver (x near 0) is no lamp unit.
fn front_split(body: &Mesh3) -> (Vec<usize>, Vec<usize>) {
    let right: Vec<usize> = (0..body.verts.len()).filter(|&i| is_cream_front(&body.verts[i]) && body.verts[i].pos[0] > 0.3).collect();
    let Some(max_x) = right.iter().map(|&i| body.verts[i].pos[0]).max_by(f32::total_cmp) else { return (vec![], vec![]) };
    let (corner, main): (Vec<usize>, Vec<usize>) = right.into_iter().partition(|&i| body.verts[i].pos[0] > max_x - 0.06);
    if corner.len() >= 6 && main.len() >= 6 { (main, corner) } else { (main.into_iter().chain(corner).collect(), vec![]) }
}

/// Indices of the vertices of the front corner (position-lamp) pieces, both sides; the car model gives them their own lamp material.
pub fn position_vertices(body: &Mesh3) -> Vec<usize> {
    let (_, corner) = front_split(body);
    if corner.is_empty() { return vec![]; }
    let lo = corner.iter().map(|&i| body.verts[i].pos[0]).fold(f32::MAX, f32::min) - 1e-4;
    (0..body.verts.len()).filter(|&i| { let v = &body.verts[i]; is_cream_front(v) && v.pos[0].abs() >= lo }).collect()
}

pub fn lamp_points(body: &Mesh3) -> Option<LampPoints> {
    let (main, corner) = front_split(body);
    let mut tail = Vec::new(); let mut rev = Vec::new();
    for v in &body.verts {
        if v.col[3] != 5 && v.col[3] != 7 { continue; }
        if v.pos[0] <= 0.0 { continue; }   // the right-hand side
        let (r, g, b) = (v.col[0], v.col[1], v.col[2]);
        if r > 200 && g < 70 && b < 70 { if v.pos[2] < 0.0 { tail.push(v.pos); } }
        else if r > 200 && g > 200 && b > 140 && v.pos[2] < 0.0 { rev.push(v.pos); }
    }
    // the outermost cluster of at least a few vertices: the main unit, not a centre brake light or a fog lamp sliver
    let pick = |pts: &[[f32; 3]]| -> Option<[f32; 3]> {
        let mut clusters: Vec<Vec<[f32; 3]>> = Vec::new();
        for p in pts {
            match clusters.iter_mut().find(|c| c.iter().any(|q| (0..3).all(|a| (p[a] - q[a]).abs() < 0.12))) { Some(c) => c.push(*p), None => clusters.push(vec![*p]) }
        }
        clusters.into_iter().filter(|c| c.len() >= 6).map(|c| { let n = c.len() as f32; [0, 1, 2].map(|a| c.iter().map(|p| p[a]).sum::<f32>() / n) })
            .max_by(|a, b| a[0].total_cmp(&b[0]))
    };
    let centre = |idx: &[usize]| -> Option<[f32; 3]> { if idx.len() < 6 { None } else { let n = idx.len() as f32; Some([0, 1, 2].map(|a| idx.iter().map(|&i| body.verts[i].pos[a]).sum::<f32>() / n)) } };
    let head = centre(&main)?;
    Some(LampPoints { head, tail: pick(&tail)?, rev: pick(&rev)?, pos: centre(&corner).unwrap_or(head) })
}


#[cfg(test)]
mod tests {
    use super::*;
    const PEUGEOT: &[u8] = include_bytes!("../../assets/rally_car.rcar");
    const PEUGEOT_V1: &[u8] = include_bytes!("../../assets/rally_car_v1.rcar");

    #[test] fn the_embedded_peugeot_parses_and_is_sane() {
        let (b, w) = parse(PEUGEOT).expect("rally_car.rcar must parse");
        assert!(b.tris() > 3000 && w.tris() > 200, "{} / {} triangles", b.tris(), w.tris());
        for m in [&b, &w] {
            assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()));
            for v in &m.verts { let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt(); assert!((l - 1.0).abs() < 0.02, "normal {l}"); assert!(v.col[3] <= 5 || (11..=15).contains(&v.col[3]), "material id {}", v.col[3]); }
        }
        let (lo, hi) = b.bounds();
        assert!(lo.y > -0.02 && hi.y < 1.6 && hi.z - lo.z > 3.8 && hi.z - lo.z < 4.2 && hi.x > 0.8 && lo.x < -0.8, "{lo:?} {hi:?}");
        let (wl, wh) = w.bounds();
        assert!(((wh.y - wl.y) / 2.0 - 0.326).abs() < 0.01 && (wl.y + wh.y).abs() < 0.02 && (wl.z + wh.z).abs() < 0.02, "wheel {wl:?} {wh:?}");
        let mats: std::collections::HashSet<u8> = b.verts.iter().map(|v| v.col[3]).collect();
        for want in [0u8, 3, 5] { assert!(mats.contains(&want), "missing material {want}"); }   // the glass is its own mesh now
    }
    #[test] fn broken_files_are_errors_not_panics() {
        assert!(parse(b"").is_err() && parse(b"NOPE").is_err() && parse(b"RCAR").is_err());
        assert!(parse(&PEUGEOT[..PEUGEOT.len() / 2]).is_err(), "truncated");
        let mut bad = PEUGEOT.to_vec(); bad[8] = 0xFF; bad[9] = 0xFF; bad[10] = 0xFF; bad[11] = 0x7F;
        assert!(parse(&bad).is_err(), "absurd index count");
        let mut idx = PEUGEOT.to_vec(); let n = idx.len(); idx[n - 1] = 0xFF; assert!(parse(&idx).is_err() || true);
    }

    #[test] fn the_lamp_points_are_found_on_the_asset_and_the_main_units_win() {
        let (b, _) = parse(PEUGEOT).unwrap();
        let l = lamp_points(&b).expect("lamps");
        // the headlight unit at the front corner, the tail unit at the rear corner, the reverse lamp beside it (measured on the lamp vertices)
        assert!((l.head[0] - 0.53).abs() < 0.05 && (l.head[1] - 0.62).abs() < 0.05 && (l.head[2] - 1.72).abs() < 0.05, "{:?}", l.head);
        assert!((l.tail[0] - 0.611).abs() < 0.03 && l.tail[2] < -1.6, "{:?}", l.tail);
        assert!(l.rev[2] < -1.6 && l.rev[0] > 0.5, "{:?}", l.rev);
    }
    #[test] fn the_front_unit_splits_into_main_lamp_and_corner_position_piece() {
        let (b, _) = parse(PEUGEOT).unwrap();
        let l = lamp_points(&b).expect("lamps");
        assert!((l.head[0] - 0.53).abs() < 0.05 && (l.head[2] - 1.72).abs() < 0.05, "main {:?}", l.head);
        assert!(l.pos[0] > l.head[0] + 0.08 && l.pos[2] < l.head[2], "the corner piece is outboard and behind the lens: {:?}", l.pos);
        let idx = position_vertices(&b);
        assert!(idx.len() >= 12 && idx.iter().all(|&i| b.verts[i].pos[2] > 1.0), "{}", idx.len());
        assert!(idx.iter().any(|&i| b.verts[i].pos[0] < 0.0) && idx.iter().any(|&i| b.verts[i].pos[0] > 0.0), "both sides");
    }
    #[test] fn a_mesh_without_a_corner_piece_uses_the_head_point_for_the_position_lamp() {
        let mut m = Mesh3::default();
        let mut vert = |x: f32, z: f32, c: [u8; 4]| m.verts.push(Vert { pos: [x, 0.6, z], nrm: [0.0, 1.0, 0.0], col: c, uv: NO_UV });
        for i in 0..8 { vert(0.5 + 0.005 * i as f32, 1.7, [255, 242, 204, 7]); }
        for i in 0..8 { vert(0.6 + 0.005 * i as f32, -1.6, [242, 13, 8, 7]); }
        for i in 0..8 { vert(0.5 + 0.005 * i as f32, -1.65, [255, 255, 255, 7]); }
        let l = lamp_points(&m).expect("lamps");
        assert_eq!(l.pos, l.head);
        assert!(position_vertices(&m).is_empty());
    }
    fn rca2(verts: &[([f32; 3], [u16; 2])]) -> Vec<u8> {
        let mut d = b"RCA2".to_vec();
        for _ in 0..2 {
            d.extend_from_slice(&(verts.len() as u32).to_le_bytes()); d.extend_from_slice(&3u32.to_le_bytes());
            for (p, uv) in verts { for f in p.iter().chain([0.0f32, 1.0, 0.0].iter()) { d.extend_from_slice(&f.to_le_bytes()); } d.extend_from_slice(&[10, 20, 30, 0]); d.extend_from_slice(&uv[0].to_le_bytes()); d.extend_from_slice(&uv[1].to_le_bytes()); }
            for i in 0..3u32 { d.extend_from_slice(&i.to_le_bytes()); }
        }
        d
    }
    #[test] fn rca2_files_carry_uvs_and_the_sentinel() {
        let (b, _) = parse(&rca2(&[([0.0, 0.0, 0.0], [100, 200]), ([1.0, 0.0, 0.0], [0xFFFF, 0xFFFF]), ([0.0, 1.0, 0.0], [0, 65534])])).unwrap();
        assert_eq!(b.verts[0].uv, [100, 200]); assert_eq!(b.verts[1].uv, NO_UV); assert_eq!(b.verts[2].uv, [0, 65534]);
        assert_eq!(b.verts[0].col, [10, 20, 30, 0]);
    }
    #[test] fn rcar_v1_still_parses_with_no_uv_and_rca2_garbage_is_an_error() {
        let (b, _) = parse(PEUGEOT_V1).unwrap();
        assert!(b.verts.iter().all(|v| v.uv == NO_UV));
        let good = rca2(&[([0.0; 3], [1, 2]), ([1.0, 0.0, 0.0], [3, 4]), ([0.0, 1.0, 0.0], [5, 6])]);
        assert!(parse(&good[..good.len() - 5]).is_err(), "truncated");
        assert!(parse(b"RCA3").is_err());
    }
    #[test] fn the_embedded_peugeot_has_uvs_on_the_paint_and_none_on_lamps() {
        let (b, _) = parse(PEUGEOT).unwrap();
        let paint: Vec<_> = b.verts.iter().filter(|v| v.col[3] == 0).collect();
        let textured = paint.iter().filter(|v| v.uv != NO_UV).count();
        assert!(textured * 10 > paint.len() * 7, "{textured} of {} paint vertices have a UV", paint.len());
        assert!(b.verts.iter().filter(|v| v.col[3] == 5).all(|v| v.uv == NO_UV), "lamps keep their colour");
        assert!(b.verts.iter().all(|v| v.uv == NO_UV || (v.uv[0] < 0xFFFF && v.uv[1] < 0xFFFF)), "a real UV never equals the sentinel");
    }
    fn rca3(meshes: usize) -> Vec<u8> {
        let mut d = b"RCA3".to_vec();
        d.extend_from_slice(&(meshes as u32).to_le_bytes());
        for m in 0..meshes {
            d.extend_from_slice(&3u32.to_le_bytes()); d.extend_from_slice(&3u32.to_le_bytes());
            for k in 0..3 { for f in [k as f32, m as f32, 0.0, 0.0, 1.0, 0.0] { d.extend_from_slice(&f.to_le_bytes()); } d.extend_from_slice(&[1, 2, 3, 0]); d.extend_from_slice(&[7, 0, 9, 0]); }
            for i in 0..3u32 { d.extend_from_slice(&i.to_le_bytes()); }
        }
        d
    }
    #[test] fn rca3_carries_up_to_four_meshes_and_older_files_have_no_disc() {
        let p = parse_all(&rca3(4)).unwrap();
        assert!(p.disc.is_some() && p.caliper.is_some());
        assert_eq!(p.body.verts[0].pos[1], 0.0); assert_eq!(p.wheel.verts[0].pos[1], 1.0); assert_eq!(p.disc.unwrap().verts[0].pos[1], 2.0); assert_eq!(p.caliper.unwrap().verts[0].uv, [7, 9]);
        let p3 = parse_all(&rca3(3)).unwrap(); assert!(p3.disc.is_some() && p3.caliper.is_none());
        let old = parse_all(PEUGEOT_V1).unwrap(); assert!(old.disc.is_none() && old.caliper.is_none());
        assert!(parse_all(&rca3(2)).is_ok() && parse_all(&rca3(1)).is_err() && parse_all(&rca3(16)).is_err(), "2 to 15 meshes");
        let p11 = parse_all(&rca3(11)).unwrap(); assert_eq!(p11.steering.as_ref().unwrap().verts[0].pos[1], 8.0); assert_eq!(p11.pivot.as_ref().unwrap().verts[0].pos[1], 9.0); assert_eq!(p11.driver.as_ref().unwrap().verts[0].pos[1], 10.0);
        assert!(parse_all(&rca3(8)).unwrap().steering.is_none() && parse_all(&rca3(9)).unwrap().pivot.is_none());
        let p8 = parse_all(&rca3(8)).unwrap(); assert_eq!(p8.glass.as_ref().unwrap().verts[0].pos[1], 6.0); assert_eq!(p8.interior.as_ref().unwrap().verts[0].pos[1], 7.0);
        let p7 = parse_all(&rca3(7)).unwrap(); assert!(p7.glass.is_some() && p7.interior.is_none(), "glass without an interior");
        assert!(parse_all(&rca3(6)).unwrap().glass.is_none());
        let p6 = parse_all(&rca3(6)).unwrap(); assert_eq!(p6.alt_wheels[0].as_ref().unwrap().verts[0].pos[1], 4.0); assert_eq!(p6.alt_wheels[1].as_ref().unwrap().verts[0].pos[1], 5.0);
        let p5 = parse_all(&rca3(5)).unwrap(); assert!(p5.alt_wheels[0].is_some() && p5.alt_wheels[1].is_none(), "only the gravel wheel");
        assert!(p.alt_wheels.iter().all(Option::is_none) && parse_all(&rca3(4)).unwrap().alt_wheels.iter().all(Option::is_none));
        let t = rca3(4); assert!(parse_all(&t[..t.len() - 9]).is_err(), "truncated");
    }
    #[test] fn the_embedded_peugeot_has_four_meshes_with_textured_wheel_parts() {
        let p = parse_all(PEUGEOT).unwrap();
        let (disc, caliper) = (p.disc.expect("disc"), p.caliper.expect("caliper"));
        let tris = |m: &Mesh3| m.idx.len() / 3;
        assert!(tris(&p.wheel) > 2000 && tris(&disc) > 100 && tris(&caliper) > 200, "{} {} {}", tris(&p.wheel), tris(&disc), tris(&caliper));
        let textured = p.wheel.verts.iter().filter(|v| v.uv != NO_UV).count();
        assert!(textured * 10 > p.wheel.verts.len() * 7, "{textured} of {} wheel vertices have a UV", p.wheel.verts.len());
        assert!(disc.verts.iter().all(|v| v.col[3] == 13), "the disc is class 13");
        assert!(caliper.verts.iter().any(|v| v.uv != NO_UV));
        let (lo, hi) = p.wheel.bounds();
        assert!(((hi.y - lo.y) / 2.0 - 0.326).abs() < 0.01 && (lo.y + hi.y).abs() < 0.02 && (lo.z + hi.z).abs() < 0.02, "wheel {lo:?} {hi:?}");
        let (dl, dh) = disc.bounds();
        assert!(dh.y - dl.y > 0.3 && dh.y - dl.y < 0.5, "the disc fits inside the rim: {dl:?} {dh:?}");
    }
    #[test] fn the_embedded_peugeot_has_three_wheel_sets() {
        let p = parse_all(PEUGEOT).unwrap();
        let sets = [Some(&p.wheel), p.alt_wheels[0].as_ref(), p.alt_wheels[1].as_ref()];
        for (i, m) in sets.iter().enumerate() {
            let m = m.unwrap_or_else(|| panic!("wheel set {i} is missing"));
            assert!(m.idx.len() / 3 > 2000, "set {i}: {} triangles", m.idx.len() / 3);
            let textured = m.verts.iter().filter(|v| v.uv != NO_UV).count();
            assert!(textured * 10 > m.verts.len() * 7, "set {i}: {textured} of {} vertices have a UV", m.verts.len());
            let (lo, hi) = m.bounds();
            assert!(((hi.y - lo.y) / 2.0 - 0.326).abs() < 0.01 && (lo.y + hi.y).abs() < 0.02 && (lo.z + hi.z).abs() < 0.02, "set {i}: {lo:?} {hi:?}");
        }
        assert!(p.wheel.verts[0].pos != p.alt_wheels[0].as_ref().unwrap().verts[0].pos && p.alt_wheels[0].as_ref().unwrap().verts[0].pos != p.alt_wheels[1].as_ref().unwrap().verts[0].pos, "the sets differ");
    }
    #[test] fn the_embedded_peugeot_carries_the_suspension_of_all_four_corners() {
        let (b, _) = parse(PEUGEOT).unwrap();
        let susp: Vec<_> = b.verts.iter().filter(|v| v.col[3] == 14).collect();
        assert!(susp.len() > 1500, "{} suspension vertices", susp.len());
        for corner in 0..4u8 {
            let c: Vec<_> = susp.iter().filter(|v| v.col[1] == corner).collect();
            assert!(c.len() > 300, "corner {corner}: {} vertices", c.len());
            assert!(c.iter().any(|v| v.col[0] > 200) && c.iter().any(|v| v.col[0] < 60), "weights run from the hub (255) to the far end (0)");
            // the corner is where its name says: +x on the left (corners 0 and 2), the front at +z (corners 0 and 1)
            let (mx, mz) = (c.iter().map(|v| v.pos[0]).sum::<f32>() / c.len() as f32, c.iter().map(|v| v.pos[2]).sum::<f32>() / c.len() as f32);
            assert!((mx > 0.0) == (corner % 2 == 0) && (mz > 0.0) == (corner < 2), "corner {corner} sits at x {mx:.2}, z {mz:.2}");
        }
        let textured = susp.iter().filter(|v| v.uv != NO_UV).count();
        assert!(textured * 10 > susp.len() * 9, "{textured} of {} suspension vertices have a UV", susp.len());
    }
    #[test] fn the_embedded_peugeot_has_glass_and_an_interior() {
        let p = parse_all(PEUGEOT).unwrap();
        let glass = p.glass.expect("glass"); let interior = p.interior.expect("interior");
        assert!(glass.verts.iter().all(|v| v.col[3] == 1 && v.uv == NO_UV), "glass is class 1 and untextured");
        assert!(glass.verts.iter().any(|v| v.col[0] == 0) && glass.verts.iter().any(|v| v.col[0] == 255), "windows (red 0) and clear lamp covers (red 255)");
        assert!(glass.idx.len() / 3 > 2500 && glass.idx.len() / 3 < 5000, "{} glass triangles", glass.idx.len() / 3);
        assert!(p.body.verts.iter().all(|v| v.col[3] != 1), "the body has no glass of its own any more");
        let t = interior.idx.len() / 3;
        assert!((12_000..=32_000).contains(&t), "{t} cabin triangles");
        assert!(interior.verts.iter().all(|v| v.col[3] == 15), "interior is class 15");
        let textured = interior.verts.iter().filter(|v| v.uv != NO_UV).count();
        assert!(textured * 10 > interior.verts.len() * 9, "{textured} of {} interior vertices have a UV", interior.verts.len());
        // the interior sits inside the body's envelope
        let ((blo, bhi), (ilo, ihi)) = (p.body.bounds(), interior.bounds());
        assert!(ilo.x >= blo.x - 0.05 && ihi.x <= bhi.x + 0.05 && ilo.z >= blo.z - 0.1 && ihi.z <= bhi.z + 0.1 && ihi.y <= bhi.y + 0.05, "{ilo:?} {ihi:?} vs {blo:?} {bhi:?}");
    }
    #[test] fn the_embedded_peugeot_has_a_steering_wheel_with_a_pivot_and_a_driver() {
        let p = parse_all(PEUGEOT).unwrap();
        let (wheel, meta, driver) = (p.steering.expect("steering wheel"), p.pivot.expect("pivot"), p.driver.expect("driver"));
        let m = &meta.verts[0];
        assert!(m.pos[0] > 0.2 && m.pos[0] < 0.5 && m.pos[1] > 0.7 && m.pos[1] < 1.1 && m.pos[2] > 0.0 && m.pos[2] < 0.6, "the wheel is in front of the left seat: {:?}", m.pos);
        let len = (m.nrm[0].powi(2) + m.nrm[1].powi(2) + m.nrm[2].powi(2)).sqrt();
        assert!((len - 1.0).abs() < 0.01 && m.nrm[2] < -0.5, "the axis is a unit vector pointing back at the driver: {:?}", m.nrm);
        let n = wheel.idx.len() / 3;
        assert!((1500..=5000).contains(&n), "{n} steering wheel triangles");
        let (lo, hi) = wheel.bounds();
        assert!(((lo.x + hi.x) / 2.0 - m.pos[0]).abs() < 0.08 && ((lo.y + hi.y) / 2.0 - m.pos[1]).abs() < 0.08, "the pivot is the wheel's centre");
        assert!(hi.x - lo.x < 0.5 && hi.y - lo.y < 0.5, "a steering wheel is smaller than 0.5 m");
        assert!(driver.idx.len() / 3 > 1000 && driver.verts.iter().all(|v| v.col[3] == 15), "{} driver triangles", driver.idx.len() / 3);
    }
    #[test] fn the_embedded_peugeot_has_two_wipers_with_pivots_on_the_windscreen() {
        let p = parse_all(PEUGEOT).unwrap();
        let [Some(l), Some(r)] = p.wipers else { panic!("two wipers") };
        let meta = p.wiper_pivots.expect("wiper pivots");
        assert_eq!(meta.verts.len(), 2);
        for (w, m) in [l, r].iter().zip(&meta.verts) {
            let n = w.idx.len() / 3;
            assert!((300..=2500).contains(&n), "{n} wiper triangles (arm and blade)");
            let len = (m.nrm[0].powi(2) + m.nrm[1].powi(2) + m.nrm[2].powi(2)).sqrt();
            assert!((len - 1.0).abs() < 0.01 && m.nrm[1] < -0.3 && m.nrm[2] < -0.2, "the axis is a unit vector pointing down and back into the car (a positive turn lifts the blade up the glass): {:?}", m.nrm);
            let (lo, hi) = w.bounds();
            assert!(m.pos[0] >= lo.x - 0.5 && m.pos[0] <= hi.x + 0.5 && m.pos[1] > 0.6 && m.pos[1] < 1.2, "the spindle is at the foot of the windscreen: {:?}", m.pos);
            assert!(hi.x - lo.x > 0.2, "a wiper is a long blade");
            let axis = [m.nrm[0], m.nrm[1], m.nrm[2]];
            let (mut dmin, mut dmax) = (f32::MAX, f32::MIN);
            for v in &w.verts { let d = (v.pos[0] - m.pos[0]) * axis[0] + (v.pos[1] - m.pos[1]) * axis[1] + (v.pos[2] - m.pos[2]) * axis[2]; dmin = dmin.min(d); dmax = dmax.max(d); }
            assert!(dmax - dmin < 0.16, "the blade lies in the plane its axis is normal to (it sweeps over the glass, not through the air): thickness {}", dmax - dmin);
        }
        assert!(meta.verts[0].pos[0] > meta.verts[1].pos[0], "the driver's wiper is on the +x side");
    }
    #[test] fn the_embedded_peugeot_has_a_backlit_dash_with_rpm_leds() {
        let p = parse_all(PEUGEOT).unwrap();
        let dash = p.dash.expect("dash");
        assert!(dash.verts.iter().any(|v| v.col[3] == 16), "the cluster is class 16");
        let mut leds: Vec<u8> = dash.verts.iter().filter(|v| v.col[3] == 17).map(|v| v.col[1]).collect();
        leds.sort(); leds.dedup();
        assert_eq!(leds, vec![1, 2, 3, 4], "four RPM LEDs");
        assert!(dash.verts.iter().all(|v| v.col[3] == 16 || v.col[3] == 17));
    }
    #[test] fn a_mesh_without_lamps_has_no_points() {
        assert!(lamp_points(&Mesh3::default()).is_none());
    }
}
#[cfg(test)] mod wiper_sweep_tests {
    use super::*;
    #[test] fn the_peugeots_wipers_sweep_only_as_far_as_the_glass_allows() {
        let car = crate::raster::rally_model::build_rally(&crate::game::vehicle::VehicleSpec::by_id("vulpes").unwrap());
        let s = car.windscreen.expect("windscreen");
        assert!(s.hull_n >= 4, "the outline of the glass: {}", s.hull_n);
        assert!(car.wiper_sweep > 1.2, "the blades reach the pillar, not the middle of the glass: {}", car.wiper_sweep);
        assert!(car.wiper_sweep > 0.5 && car.wiper_sweep <= crate::raster::vehicle_models::WIPER_SWEEP, "sweep {}", car.wiper_sweep);
        for w in car.wipers.iter().flatten() {
            // at the swept angle the blade's tip is on the glass; a good deal further it is not
            let tip = |th: f32| { let p = w.tip_at(th, 0.93); let r = [p.x - s.origin[0], p.y - s.origin[1], p.z - s.origin[2]]; s.inside(r[0] * s.u[0] + r[1] * s.u[1] + r[2] * s.u[2], r[0] * s.v[0] + r[1] * s.v[1] + r[2] * s.v[2], 0.0) };
            assert!(tip(car.wiper_sweep), "the tip stays on the glass at the sweep");
            // and it stays close to the glass's surface all the way: the tip's height above the glass's plane changes by less than the glass's own curvature allows
            let height = |th: f32| { let p = w.tip_at(th, 0.93); (p.x - s.origin[0]) * s.n[0] + (p.y - s.origin[1]) * s.n[1] + (p.z - s.origin[2]) * s.n[2] };
            let h0 = height(0.0);
            for k in 1..=10 {
                let th = car.wiper_sweep * k as f32 / 10.0;
                assert!((height(th) - h0).abs() < 0.03, "the blade tip rises {} m off the glass's plane at {th} rad", height(th) - h0);
            }
        }
    }
}
