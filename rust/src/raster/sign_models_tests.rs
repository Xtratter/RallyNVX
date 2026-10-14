use super::*;

#[test] fn every_sign_kind_has_a_sound_mesh_of_a_sensible_size() {
    let all = build_all();
    assert_eq!(all.len(), SignKind::ALL.len());
    for (k, m) in SignKind::ALL.iter().zip(&all) {
        assert!(m.tris() > 20 && m.tris() < 1500, "{k:?}: {} triangles", m.tris());
        assert!(m.idx.iter().all(|i| (*i as usize) < m.verts.len()), "{k:?}: an index is out of range");
        let (lo, hi) = m.bounds();
        assert!(lo.y >= -0.01 && hi.y > 2.3 && hi.y < 2.9, "{k:?}: the sign spans y {}..{}", lo.y, hi.y);
        assert!(hi.x - lo.x < 1.4 && hi.z - lo.z < 0.2, "{k:?}: too wide ({}) or deep ({})", hi.x - lo.x, hi.z - lo.z);
        assert!(m.verts.iter().all(|v| v.nrm.iter().all(|c| c.is_finite()) && v.pos.iter().all(|c| c.is_finite())));
    }
}
#[test] fn mirrored_signs_are_mirror_images() {
    let all = build_all();
    let (l, r) = (&all[SignKind::CurveL.index()], &all[SignKind::CurveR.index()]);
    assert_eq!(l.verts.len(), r.verts.len());
    let (lb, rb) = (l.bounds(), r.bounds());
    // the arrow head leans to the left in one and the right in the other: their x centres are on opposite sides of the post
    let cx = |m: &Mesh3| m.verts.iter().filter(|v| v.col[0] < 40 && v.col[1] < 40 && v.col[3] == 0).map(|v| v.pos[0]).sum::<f32>() / m.verts.iter().filter(|v| v.col[0] < 40 && v.col[1] < 40 && v.col[3] == 0).count().max(1) as f32;
    assert!(cx(l) < -0.005 && cx(r) > 0.005, "{} {}", cx(l), cx(r));
    let _ = (lb, rb);
}
