use super::*;
use crate::game::math::v3;
fn check(m: &Mesh3, name: &str) {
    assert!(!m.verts.is_empty() && m.idx.len() % 3 == 0, "{name}");
    assert!(m.idx.iter().all(|&i| (i as usize) < m.verts.len()), "{name} indices");
    for v in &m.verts {
        assert!(v.pos.iter().all(|c| c.is_finite()), "{name}");
        let l = (v.nrm[0].powi(2) + v.nrm[1].powi(2) + v.nrm[2].powi(2)).sqrt();
        assert!((l - 1.0).abs() < 1e-3, "{name} normal {l}");
    }
}
#[test] fn variant_counts_and_triangle_budgets() {
    let n = build_all();
    assert_eq!((n.trees.len(), n.bushes.len(), n.rocks.len()), (4, 2, 3));
    for (i, m) in n.trees.iter().enumerate() { check(m, "tree"); assert!(m.tris() <= 320, "tree {i}: {}", m.tris()); }
    for m in &n.bushes { check(m, "bush"); assert!(m.tris() <= 150, "bush {}", m.tris()); }
    for m in &n.rocks { check(m, "rock"); assert!(m.tris() <= 120, "rock {}", m.tris()); }
    check(&n.grass, "grass"); assert_eq!(n.grass.tris(), 6);
}
#[test] fn sizes_match_the_unit_conventions() {
    let n = build_all();
    for m in &n.trees {
        let (lo, hi) = m.bounds();
        assert!(lo.y > -0.05 && hi.y > 1.5 && hi.y < 2.1, "tree height {} .. {}", lo.y, hi.y);
        assert!(m.verts.iter().filter(|v| v.pos[1] < 0.3).all(|v| v.pos[0].abs() <= 0.12 && v.pos[2].abs() <= 0.12), "trunk is thin");
        assert!(hi.x.max(hi.z) < 0.9 && lo.x.min(lo.z) > -0.9);
    }
    for m in &n.rocks {
        let (lo, hi) = m.bounds();
        assert!(m.verts.iter().all(|v| (v.pos[0].powi(2) + v.pos[1].powi(2) + v.pos[2].powi(2)).sqrt() < 1.05));
        assert!(hi.y - lo.y > 1.0 && hi.y - lo.y < 1.5, "rock height {}", hi.y - lo.y);
    }
    for m in &n.bushes { let (lo, hi) = m.bounds(); assert!(lo.y > -0.05 && hi.y < 0.9 && hi.x < 0.9 && lo.x > -0.9 && hi.z < 0.9 && lo.z > -0.9); }
    let (lo, hi) = n.grass.bounds(); assert!(lo.y >= 0.0 && hi.y <= 0.65 && hi.x < 0.3 && lo.x > -0.3);
}
#[test] fn variant_choice_is_deterministic_and_covers_everything() {
    for (kind, count) in [(0u8, 4usize), (1, 2), (2, 3)] {
        let mut seen = vec![false; count];
        for i in 0..200 { let p = v3(i as f32 * 3.7, 0.0, i as f32 * -1.9); let v = variant(kind, p); assert_eq!(v, variant(kind, p)); seen[v] = true; }
        assert!(seen.iter().all(|s| *s), "kind {kind} covers {seen:?}");
    }
    for i in 0..200 { let t = tint(v3(i as f32, 0.0, 2.0 * i as f32)); assert!(t.iter().all(|c| *c >= 0.7 && *c <= 1.3)); }
}
