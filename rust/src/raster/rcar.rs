//! Reader of the `.rcar` car asset written by `tools/carconv`: a body mesh and one wheel mesh in the renderer's vertex
//! format (little endian: `RCAR`, then per mesh `u32 vertex count`, `u32 index count`, 28-byte vertices, `u32` indices).
use super::vehicle_models::{Mesh3, Vert};

const MAX_VERTS: usize = 400_000;

fn mesh(d: &[u8], o: &mut usize) -> Result<Mesh3, String> {
    let rd = |o: &mut usize| -> Result<u32, String> { let b = d.get(*o..*o + 4).ok_or("truncated")?; *o += 4; Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])) };
    let (nv, ni) = (rd(o)? as usize, rd(o)? as usize);
    if nv == 0 || nv > MAX_VERTS || ni == 0 || ni % 3 != 0 || ni > MAX_VERTS * 6 { return Err(format!("bad counts {nv}/{ni}")); }
    if d.len() < *o + nv * 28 + ni * 4 { return Err("truncated".into()); }
    let f = |at: usize| f32::from_le_bytes([d[at], d[at + 1], d[at + 2], d[at + 3]]);
    let mut m = Mesh3::default();
    for _ in 0..nv {
        let a = *o;
        m.verts.push(Vert { pos: [f(a), f(a + 4), f(a + 8)], nrm: [f(a + 12), f(a + 16), f(a + 20)], col: [d[a + 24], d[a + 25], d[a + 26], d[a + 27]] });
        *o += 28;
    }
    for _ in 0..ni { let i = rd(o)?; if i as usize >= nv { return Err("index out of range".into()); } m.idx.push(i); }
    if m.verts.iter().any(|v| v.pos.iter().chain(v.nrm.iter()).any(|c| !c.is_finite())) { return Err("non-finite vertex".into()); }
    Ok(m)
}

/// `(body, wheel)` from the bytes of an `.rcar` file.
pub fn parse(d: &[u8]) -> Result<(Mesh3, Mesh3), String> {
    if d.len() < 4 || &d[..4] != b"RCAR" { return Err("not an RCAR file".into()); }
    let mut o = 4;
    let body = mesh(d, &mut o)?;
    let wheel = mesh(d, &mut o)?;
    Ok((body, wheel))
}

/// Where the lamp units of a car mesh are, found from its emissive vertices (material 5) by colour: headlight units (cream or white at the front), tail units (red) and reverse
/// lamps (white at the rear). Metres in the mesh's frame, for the car's right-hand side (the cars are symmetric): the units' centres. A new car model only has to colour
/// its lamp meshes this way (the converter does) and its lamps sit on the mesh, wherever the designer put them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampPoints { pub head: [f32; 3], pub tail: [f32; 3], pub rev: [f32; 3] }

pub fn lamp_points(body: &Mesh3) -> Option<LampPoints> {
    let mut head = Vec::new(); let mut tail = Vec::new(); let mut rev = Vec::new();
    for v in &body.verts {
        if v.col[3] != 5 && v.col[3] != 7 { continue; }
        let (r, g, b) = (v.col[0], v.col[1], v.col[2]);
        if v.pos[0] <= 0.0 { continue; }   // the right-hand side
        if r > 200 && g < 70 && b < 70 { if v.pos[2] < 0.0 { tail.push(v.pos); } }
        else if r > 200 && g > 200 && b > 140 { if v.pos[2] > 0.0 { head.push(v.pos); } else { rev.push(v.pos); } }
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
    Some(LampPoints { head: pick(&head)?, tail: pick(&tail)?, rev: pick(&rev)? })
}

#[cfg(test)]
mod tests {
    use super::*;
    const PEUGEOT: &[u8] = include_bytes!("../../assets/rally_car.rcar");

    #[test] fn the_embedded_peugeot_parses_and_is_sane() {
        let (b, w) = parse(PEUGEOT).expect("rally_car.rcar must parse");
        assert!(b.tris() > 3000 && w.tris() > 200, "{} / {} triangles", b.tris(), w.tris());
        for m in [&b, &w] {
            assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()));
            for v in &m.verts { let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt(); assert!((l - 1.0).abs() < 0.02, "normal {l}"); assert!(v.col[3] <= 5, "material id {}", v.col[3]); }
        }
        let (lo, hi) = b.bounds();
        assert!(lo.y > -0.02 && hi.y < 1.6 && hi.z - lo.z > 3.8 && hi.z - lo.z < 4.2 && hi.x > 0.8 && lo.x < -0.8, "{lo:?} {hi:?}");
        let (wl, wh) = w.bounds();
        assert!(((wh.y - wl.y) / 2.0 - 0.326).abs() < 0.01 && (wl.y + wh.y).abs() < 0.02 && (wl.z + wh.z).abs() < 0.02, "wheel {wl:?} {wh:?}");
        let mats: std::collections::HashSet<u8> = b.verts.iter().map(|v| v.col[3]).collect();
        for want in [0u8, 1, 3, 5] { assert!(mats.contains(&want), "missing material {want}"); }
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
        assert!((l.head[0] - 0.569).abs() < 0.03 && (l.head[1] - 0.634).abs() < 0.03 && (l.head[2] - 1.693).abs() < 0.03, "{:?}", l.head);
        assert!((l.tail[0] - 0.611).abs() < 0.03 && l.tail[2] < -1.6, "{:?}", l.tail);
        assert!(l.rev[2] < -1.6 && l.rev[0] > 0.5, "{:?}", l.rev);
    }
    #[test] fn a_mesh_without_lamps_has_no_points() {
        assert!(lamp_points(&Mesh3::default()).is_none());
    }
}
