use super::*;
fn check(m: &Mesh, name: &str) {
    assert!(!m.verts.is_empty() && m.idx.len() % 3 == 0, "{name}");
    assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()), "{name} idx");
    for v in &m.verts {
        assert!(v[..3].iter().all(|c| c.abs() <= 0.5001), "{name} extent {v:?}");
        let l = (v[3] * v[3] + v[4] * v[4] + v[5] * v[5]).sqrt();
        assert!((l - 1.0).abs() < 1e-3, "{name} normal {l}");
    }
}
#[test] fn all_primitives_are_valid() { check(&cube(), "cube"); check(&cylinder(), "cyl"); check(&cone(), "cone"); check(&sphere(), "sphere"); }
#[test] fn cube_has_24_verts_and_12_tris() { let c = cube(); assert_eq!((c.verts.len(), c.idx.len()), (24, 36)); }
