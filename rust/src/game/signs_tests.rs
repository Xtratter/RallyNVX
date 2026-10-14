use super::*;
use crate::game::roads::gen::{generate_with, GenCfg};

fn terrain(seed: u64) -> Terrain {
    let base = Terrain::new(seed);
    let net = generate_with(seed, &base, GenCfg::coarse());
    base.with_net(net)
}
fn count(s: &[Sign], k: SignKind) -> usize { s.iter().filter(|x| x.kind == k).count() }

#[test] fn a_stage_gets_warnings_chevrons_distance_boards_and_the_finish_set() {
    for seed in [3u64, 8, 21] {
        let t = terrain(seed);
        let s = stage_signs(&t);
        assert!(s.len() > 40, "seed {seed}: {} signs", s.len());
        let curves = count(&s, SignKind::CurveL) + count(&s, SignKind::CurveR) + count(&s, SignKind::SCurveL) + count(&s, SignKind::SCurveR) + count(&s, SignKind::HairpinL) + count(&s, SignKind::HairpinR);
        assert!(curves >= 10, "seed {seed}: {curves} corner warnings");
        assert!(count(&s, SignKind::ChevL) + count(&s, SignKind::ChevR) >= 6, "seed {seed}: chevrons");
        assert_eq!((count(&s, SignKind::Dist3), count(&s, SignKind::Dist2), count(&s, SignKind::Dist1)), (1, 1, 1));
        assert_eq!((count(&s, SignKind::FinishBoard), count(&s, SignKind::Stop), count(&s, SignKind::Start)), (2, 1, 1));
        let sides = t.roads().unwrap().roads.iter().filter(|r| r.kind == RoadKind::Side).count();
        assert!(count(&s, SignKind::RouteArrow) <= sides && count(&s, SignKind::NoEntry) <= sides, "one arrow and one no-entry per junction at most");
    }
}
#[test] fn signs_stand_beside_the_road_on_dry_ground_facing_the_driver() {
    let t = terrain(8);
    let stage = t.roads().unwrap().roads.iter().find(|r| r.kind == RoadKind::Stage).unwrap();
    let s = stage_signs(&t);
    for x in s.iter().filter(|x| !matches!(x.kind, SignKind::NoEntry)) {
        let (d, p) = stage.pts.iter().map(|p| ((p.pos.x - x.pos.x).hypot(p.pos.z - x.pos.z), p)).fold((f32::INFINITY, &stage.pts[0]), |a, b| if b.0 < a.0 { b } else { a });
        assert!(d >= p.width * 0.5 + 2.4 && d <= p.width * 0.5 + 4.6, "{:?} stands {d} m from the axis of a {} m road", x.kind, p.width);
        assert!(!t.is_water(x.pos.x, x.pos.z, 0.0));
        assert!((x.pos.y - t.height(x.pos.x, x.pos.z)).abs() < 1e-3);
    }
    // a warning triangle faces back along the road: its facing direction points against the direction of travel
    for x in s.iter().filter(|x| x.kind == SignKind::CurveL || x.kind == SignKind::CurveR).take(5) {
        let i = stage.pts.iter().enumerate().min_by(|a, b| ((a.1.pos.x - x.pos.x).hypot(a.1.pos.z - x.pos.z)).partial_cmp(&((b.1.pos.x - x.pos.x).hypot(b.1.pos.z - x.pos.z))).unwrap()).unwrap().0;
        let (a, b) = (stage.pts[i.saturating_sub(1)].pos, stage.pts[(i + 1).min(stage.pts.len() - 1)].pos);
        let (tx, tz) = (b.x - a.x, b.z - a.z);
        let face = (x.yaw.sin(), x.yaw.cos());
        assert!(face.0 * tx + face.1 * tz < 0.0, "a warning sign does not face the oncoming car");
    }
}
#[test] fn the_sign_set_is_deterministic_and_the_kind_index_is_dense() {
    let (a, b) = (stage_signs(&terrain(5)), stage_signs(&terrain(5)));
    assert_eq!(a.len(), b.len());
    assert!(a.iter().zip(&b).all(|(p, q)| p.kind == q.kind && p.pos == q.pos));
    for (i, k) in SignKind::ALL.iter().enumerate() { assert_eq!(k.index(), i); }
    assert!(stage_signs(&Terrain::flat()).is_empty());
