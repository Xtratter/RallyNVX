use super::*;
use crate::game::roads::gen::{generate_with, GenCfg};
use crate::game::terrain::Terrain;

fn net(seed: u64) -> (Terrain, crate::game::roads::RoadNet) { let t = Terrain::new(seed); let n = generate_with(seed, &t, GenCfg::coarse()); (t, n) }
fn dist(a: (f32, f32), b: (f32, f32)) -> f32 { (a.0 - b.0).hypot(a.1 - b.1) }

#[test] fn every_seed_gets_a_few_sound_side_roads() {
    let c = crate::game::roads::cfg::cfg().side;
    let mut total = 0;
    for seed in 1..=14u64 {
        let (t, n) = net(seed);
        let stage = &n.roads[1];
        let sides: Vec<&Road> = n.roads.iter().filter(|r| r.kind == RoadKind::Side).collect();
        assert!(sides.len() <= c.count.1 as usize, "seed {seed}: {} side roads", sides.len());
        total += sides.len();
        let mut junctions = Vec::new();
        for r in &sides {
            assert!(r.length() >= 90.0 && r.length() <= 520.0, "seed {seed}: side road of {} m", r.length());
            let p0 = (r.pts[0].pos.x, r.pts[0].pos.z);
            let (j, d0) = stage.pts.iter().enumerate().map(|(j, p)| (j, dist(p0, (p.pos.x, p.pos.z)))).fold((0, f32::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
            assert!(d0 < 2.0, "seed {seed}: the side road starts {d0} m from the stage");
            assert!((r.pts[0].pos.y - stage.pts[j].pos.y).abs() < 0.2, "seed {seed}: the mouth is not flush ({} vs {})", r.pts[0].pos.y, stage.pts[j].pos.y);
            junctions.push(stage.pts[j].s);
            for w in r.pts.windows(2) { let d = w[1].pos - w[0].pos; assert!(d.y.abs() / d.x.hypot(d.z).max(1e-3) <= 0.25 + 1e-3, "seed {seed}: grade"); }
            let wet = r.pts.iter().filter(|p| t.is_water(p.pos.x, p.pos.z, 0.0)).count();
            assert!(wet * 20 <= r.pts.len(), "seed {seed}: {wet} wet points");
            for p in r.pts.iter().filter(|p| dist(p0, (p.pos.x, p.pos.z)) > 80.0) {
                let near = stage.pts.iter().map(|q| dist((p.pos.x, p.pos.z), (q.pos.x, q.pos.z))).fold(f32::INFINITY, f32::min);
                assert!(near >= c.min_clear * 0.7, "seed {seed}: a side road comes {near} m from the stage");
            }
            assert!(r.pts.iter().all(|p| (4.9..=8.1).contains(&p.width)), "seed {seed}: widths");
        }
        junctions.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for w in junctions.windows(2) { assert!(w[1] - w[0] >= 450.0, "seed {seed}: junctions only {} m apart", w[1] - w[0]); }
        assert!(junctions.iter().all(|s| *s >= c.keep_ends * 0.8 && *s <= stage.length() - c.keep_ends * 0.8), "seed {seed}: a junction near an end");
    }
    assert!(total >= 14 * 2, "only {total} side roads over 14 seeds");
}
#[test] fn side_roads_are_deterministic_and_the_network_finds_them() {
    let (_, a) = net(7); let (_, b) = net(7);
    assert_eq!(a.roads.len(), b.roads.len());
    assert!(a.roads.iter().zip(&b.roads).all(|(x, y)| x.pts.len() == y.pts.len() && x.pts[3].pos == y.pts[3].pos));
    let side = a.roads.iter().position(|r| r.kind == RoadKind::Side).expect("seed 7 has a side road");
    let mid = a.roads[side].pts[a.roads[side].pts.len() / 2].pos;
    let hit = a.nearest(mid.x + 1.0, mid.z).unwrap();
    assert_eq!(hit.road, side);
}
#[test] #[ignore] fn dbg_counts() {
    for seed in 1..=14u64 { let (_, n) = net(seed); let s: Vec<String> = n.roads.iter().filter(|r| r.kind == RoadKind::Side).map(|r| format!("{:.0}", r.length())).collect(); println!("COUNT seed {seed}: {} [{}]", s.len(), s.join(" ")); }
}
#[test] #[ignore] fn dbg_junctions_21() {
    let t = Terrain::with_roads(21);
    let n = t.roads().unwrap();
    let stage = &n.roads[1];
    for r in n.roads.iter().filter(|r| r.kind == RoadKind::Side) {
        let p0 = (r.pts[0].pos.x, r.pts[0].pos.z);
        let s = stage.pts.iter().min_by(|a, b| dist(p0, (a.pos.x, a.pos.z)).partial_cmp(&dist(p0, (b.pos.x, b.pos.z))).unwrap()).unwrap().s;
        println!("JUNCTION s {s:.0} len {:.0}", r.length());
    }
}
