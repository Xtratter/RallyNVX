use super::*;
use crate::game::{roads::gen::{generate_with, GenCfg}, terrain::Terrain};

fn roaded(seed: u64) -> Terrain {
    let base = Terrain::new(seed);
    let net = generate_with(seed, &base, GenCfg::coarse());
    base.with_net(net)
}

#[test] fn every_road_segment_is_in_exactly_one_tile_mesh_with_sane_vertices() {
    let t = roaded(5);
    let net = t.roads().unwrap();
    let mut quads = 0usize;
    let (mut tiles_x, mut tiles_z) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
    for r in &net.roads { for p in &r.pts { tiles_x.insert((p.pos.x / TILE_SIZE).floor() as i32); tiles_z.insert((p.pos.z / TILE_SIZE).floor() as i32); } }
    for &tx in &tiles_x { for &tz in &tiles_z {
        if let Some(m) = road_mesh(&t, tx, tz) {
            assert!(m.idx.len() % 6 == 0 && m.verts.len() % 4 == 0 && m.idx.iter().all(|&i| (i as usize) < m.verts.len()));
            for v in &m.verts { assert!(v.iter().all(|c| c.is_finite()) && v[3].abs() <= 1.0 + 1e-4 && v[4] >= 0.0); let w = v[5] % 100.0; assert!((4.5..=9.1).contains(&w), "width {w}"); }
            quads += m.verts.len() / 4;
        }
    } }
    let segs: usize = net.roads.iter().map(|r| r.pts.len() - 1).sum();
    assert_eq!(quads, segs, "each segment belongs to the tile of its midpoint, once");
}
#[test] fn a_tile_without_roads_has_no_mesh_and_the_ribbon_sits_just_above_the_ground() {
    let t = roaded(5);
    assert!(road_mesh(&t, 500, 500).is_none());
    assert!(road_mesh(&Terrain::new(5), 0, 0).is_none(), "no roads, no mesh");
    let r = &t.roads().unwrap().roads[0];
    let p = r.pts[r.pts.len() / 2];
    let (tx, tz) = ((p.pos.x / TILE_SIZE).floor() as i32, (p.pos.z / TILE_SIZE).floor() as i32);
    let m = road_mesh(&t, tx, tz).unwrap();
    for v in m.verts.iter().take(40) { assert!((v[1] - t.height(v[0], v[2]) - 0.07).abs() < 0.45, "the ribbon floats {}", v[1] - t.height(v[0], v[2])); }
    let kinds: Vec<f32> = m.verts.iter().map(|v| (v[5] / 100.0).floor()).collect();
    assert!(kinds.iter().all(|k| *k == 0.0 || *k == 1.0));
}
