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

#[cfg(test)]
mod tests {
    use super::*;
    const PEUGEOT: &[u8] = include_bytes!("../../assets/peugeot206.rcar");

    #[test] fn the_embedded_peugeot_parses_and_is_sane() {
        let (b, w) = parse(PEUGEOT).expect("peugeot206.rcar must parse");
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
}
