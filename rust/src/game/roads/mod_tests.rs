use super::*;
use crate::game::math::v3;

fn straight(kind: RoadKind, z0: f32, len: f32) -> Road {
    let n = (len / 4.0) as usize;
    Road { kind, pts: (0..=n).map(|i| RoadPt { pos: v3(0.0, i as f32 * 0.1, z0 + i as f32 * 4.0), width: 8.0, s: i as f32 * 4.0 }).collect() }
}

#[test] fn nearest_finds_the_distance_the_height_the_half_width_and_the_tangent() {
    let net = RoadNet::new(vec![straight(RoadKind::Stage, 0.0, 400.0)]);
    let h = net.nearest(3.0, 100.0).expect("on the road");
    assert!((h.dist - 3.0).abs() < 1e-3 && (h.half - 4.0).abs() < 1e-4 && (h.y - 2.5).abs() < 1e-3 && (h.s - 100.0).abs() < 1e-3);
    assert!((h.tangent.z - 1.0).abs() < 1e-4 && h.tangent.x.abs() < 1e-4);
    assert!(net.nearest(30.0, 100.0).is_none(), "30 m away is out of reach");
    assert!(net.nearest(0.0, -20.0).is_none() && net.nearest(0.0, 500.0).is_none());
}
#[test] fn the_spatial_hash_agrees_with_brute_force() {
    let a = straight(RoadKind::Stage, 0.0, 400.0);
    let mut b = straight(RoadKind::Liaison, 0.0, 300.0);
    for p in &mut b.pts { p.pos = v3(p.pos.z * 0.6 - 90.0, p.pos.y, p.pos.x + 150.0 + p.pos.z * 0.2); }
    let net = RoadNet::new(vec![a, b]);
    let mut seen = 0;
    for ix in -60..60 { for iz in -20..120 {
        let (x, z) = (ix as f32 * 3.7, iz as f32 * 4.3);
        let (f, s) = (net.nearest(x, z), net.nearest_slow(x, z));
        assert_eq!(f.is_some(), s.is_some(), "({x}, {z})");
        if let (Some(f), Some(s)) = (f, s) { seen += 1; assert!((f.dist - s.dist).abs() < 1e-3, "({x}, {z}): {} vs {}", f.dist, s.dist); }
    } }
    assert!(seen > 500);
}
#[test] fn an_empty_net_finds_nothing_and_kinds_map_to_surfaces() {
    assert!(RoadNet::new(vec![]).nearest(0.0, 0.0).is_none() && RoadNet::default().is_empty());
    assert_eq!((RoadKind::Liaison.surface(), RoadKind::Stage.surface()), (Surface::Asphalt, Surface::Gravel));
}
