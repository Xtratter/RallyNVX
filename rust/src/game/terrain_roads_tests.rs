use super::*;
use crate::game::{props::tile_props, scatter::tile_bushes, surface::{surface_at, Surface}};

fn with_roads_coarse(seed: u64) -> Terrain {
    // the same as with_roads but at the coarse test resolution
    let base = Terrain::new(seed);
    let net = crate::game::roads::gen::generate_with(seed, &base, crate::game::roads::gen::GenCfg::coarse());
    base.with_net(net)
}

#[test] fn the_ground_on_the_road_follows_the_profile_and_far_from_it_nothing_changes() {
    let (t, plain) = (with_roads_coarse(5), Terrain::new(5));
    let r = &t.roads().unwrap().roads[1];
    for i in (0..r.pts.len()).step_by(37) {
        let p = r.pts[i];
        assert!((t.height(p.pos.x, p.pos.z) - p.pos.y).abs() < 0.05, "point {i}");
        let side = t.road_hit(p.pos.x + 3.0, p.pos.z + 3.0);
        assert!(side.is_some());
    }
    let mut changed = 0;
    for k in 0..400 {
        let (x, z) = (-3500.0 + (k % 20) as f32 * 350.0, -3500.0 + (k / 20) as f32 * 350.0);
        let far = t.road_hit(x, z).is_none();
        if far { assert_eq!(t.height(x, z), plain.height(x, z), "({x}, {z})"); } else { changed += 1; }
    }
    assert!(changed < 100);
}
#[test] fn the_surface_function_reports_the_road_the_shoulder_and_the_terrain() {
    let t = with_roads_coarse(6);
    let (lia, stage) = (&t.roads().unwrap().roads[0], &t.roads().unwrap().roads[1]);
    let (a, b) = (lia.pts[lia.pts.len() / 2].pos, stage.pts[stage.pts.len() / 2].pos);
    assert_eq!((surface_at(&t, a.x, a.z), surface_at(&t, b.x, b.z)), (Surface::Asphalt, Surface::Gravel));
    let h = t.road_hit(b.x, b.z).unwrap();
    let right = v3(h.tangent.z, 0.0, -h.tangent.x);
    let edge = h.half + 1.0;
    assert_eq!(surface_at(&t, b.x + right.x * edge, b.z + right.z * edge), Surface::Dirt);
    let far = surface_at(&t, b.x + right.x * 60.0, b.z + right.z * 60.0);
    assert!(matches!(far, Surface::Grass | Surface::Dirt | Surface::Sand | Surface::Rock | Surface::Water | Surface::Asphalt));
    assert!(t.road_edge_distance(b.x, b.z).unwrap() < 0.0 && t.near_road(b.x, b.z, 0.5));
}
#[test] fn trees_rocks_and_bushes_keep_off_the_road() {
    let t = with_roads_coarse(3);
    let r = &t.roads().unwrap().roads[1];
    let mut seen = 0;
    for i in (0..r.pts.len()).step_by(60) {
        let (tx, tz) = ((r.pts[i].pos.x / TILE_SIZE).floor() as i32, (r.pts[i].pos.z / TILE_SIZE).floor() as i32);
        for dz in -1..=1 { for dx in -1..=1 {
            for p in tile_props(&t, tx + dx, tz + dz) { seen += 1; assert!(!t.near_road(p.pos.x, p.pos.z, 3.0), "a prop on the road at {:?}", p.pos); }
            for b in tile_bushes(&t, tx + dx, tz + dz) { assert!(!t.near_road(b.pos.x, b.pos.z, 2.5)); }
        } }
    }
    assert!(seen > 50);
}
#[test] fn the_road_bed_is_drivable_flat_across() {
    let t = with_roads_coarse(8);
    let r = &t.roads().unwrap().roads[1];
    for i in (10..r.pts.len() - 10).step_by(50) {
        let p = r.pts[i];
        let n = t.normal(p.pos.x, p.pos.z);
        assert!(n.y > 0.9, "normal.y {} at point {i}", n.y);
    }
}

fn straight(kind: crate::game::roads::RoadKind, y: f32) -> crate::game::roads::RoadNet {
    use crate::game::roads::{Road, RoadNet, RoadPt};
    let pts = (0..=200).map(|i| RoadPt { pos: v3(-400.0 + i as f32 * 4.0, y, 0.0), width: 8.0, s: i as f32 * 4.0 }).collect();
    RoadNet::new(vec![Road { kind, pts }])
}
#[test] fn the_stage_ditch_is_deep_the_liaison_ditch_shallow_and_the_road_unchanged() {
    use crate::game::roads::RoadKind;
    let st = Terrain::flat().with_net(straight(RoadKind::Stage, 0.0));
    let li = Terrain::flat().with_net(straight(RoadKind::Liaison, 0.0));
    let cfg = crate::game::roads::cfg::cfg();
    // the centre of the ditch: 4 m half width + the offset (the road blend still weighs it by ~0.9)
    let (d_st, d_li) = (-st.height(0.0, 4.0 + cfg.ditch_stage.offset), -li.height(0.0, 4.0 + cfg.ditch_liaison.offset));
    assert!((d_st - cfg.ditch_stage.depth).abs() < 0.12, "{d_st}");
    assert!((d_li - cfg.ditch_liaison.depth).abs() < 0.12, "{d_li}");
    assert_eq!(st.height(0.0, 0.0), 0.0);
    assert!(st.height(0.0, 3.9).abs() < 1e-4, "no ditch under the road");
    assert!(st.height(0.0, 40.0).abs() < 1e-4, "none far away");
}
#[test] fn no_ditch_on_a_causeway() {
    use crate::game::roads::RoadKind;
    let t = Terrain { water: Some(0.0), ..Terrain::flat() }.with_net(straight(RoadKind::Stage, 0.5));
    assert!((t.height(0.0, 5.8) - 0.5 * (1.0 - smoothstep(5.0, 12.0, 5.8))).abs() < 1e-3, "{}", t.height(0.0, 5.8));
}
#[test] fn the_stage_verge_is_gravel_and_the_liaison_verge_dirt() {
    use crate::game::roads::RoadKind;
    let st = Terrain::flat().with_net(straight(RoadKind::Stage, 0.0));
    let li = Terrain::flat().with_net(straight(RoadKind::Liaison, 0.0));
    assert_eq!(surface_at(&st, 0.0, 4.5), Surface::Gravel);
    assert_eq!(surface_at(&st, 0.0, 5.5), Surface::Dirt);
    assert_eq!(surface_at(&li, 0.0, 4.5), Surface::Dirt);
}
