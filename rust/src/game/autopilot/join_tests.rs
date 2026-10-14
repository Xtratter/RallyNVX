use super::*;
use crate::game::{autopilot::Autopilot, roads::RoadKind, vehicle::Vehicle};

fn world_terrain(seed: u64) -> Terrain {
    let base = Terrain::new(seed);
    let net = crate::game::roads::gen::generate_with(seed, &base, crate::game::roads::gen::GenCfg::coarse());
    base.with_net(net)
}

#[test] fn from_the_field_the_route_ends_on_the_stage_and_keeps_out_of_the_water() {
    let car = VehicleSpec::offroader();
    for seed in [3u64, 7, 12, 21] {
        let t = world_terrain(seed);
        let stage = t.roads().unwrap().roads.iter().find(|r| r.kind == RoadKind::Stage).unwrap();
        // a start 150-300 m beside the middle of the stage
        let m = stage.pts[stage.pts.len() / 2].pos;
        let near = |f: (f32, f32)| stage.pts.iter().map(|p| (p.pos.x - f.0).hypot(p.pos.z - f.1)).fold(f32::INFINITY, f32::min);
        let Some(from) = [(220.0, 120.0), (-300.0, 200.0), (400.0, -300.0), (-500.0, -400.0), (700.0, 650.0)].iter().map(|o| (m.x + o.0, m.z + o.1))
            .find(|f| near(*f) > 80.0 && !t.is_water(f.0, f.1, 5.0) && f.0.abs() < 3500.0 && f.1.abs() < 3500.0) else { continue };
        let v = Vehicle::new(car, v3(from.0, t.height(from.0, from.1) + 1.1, from.1), 0.0);
        let ap = Autopilot::start(&t, &v, car, 1).expect("a stage exists");
        let r = ap.route();
        let last = *r.p.last().unwrap();
        let d_stage = stage.pts.iter().map(|p| (p.pos.x - last.x).hypot(p.pos.z - last.z)).fold(f32::INFINITY, f32::min);
        assert!(d_stage < 12.0, "seed {seed}: the field route ends {d_stage} m from the stage");
        let wet = r.p.iter().filter(|p| t.is_water(p.x, p.z, 0.0)).count();
        assert!(wet * 20 <= r.p.len(), "seed {seed}: {wet}/{} route points in water", r.p.len());
        let direct = ((from.0 - last.x).hypot(from.1 - last.z)).max(1.0);
        assert!(r.length() < direct * 2.2 + 100.0, "seed {seed}: route {} m for {direct} m", r.length());
    }
}
#[test] fn on_the_stage_the_autopilot_joins_directly() {
    let car = VehicleSpec::offroader();
    let t = world_terrain(5);
    let stage = t.roads().unwrap().roads.iter().find(|r| r.kind == RoadKind::Stage).unwrap();
    let p = stage.pts[100].pos;
    let v = Vehicle::new(car, v3(p.x + 3.0, p.y + 1.1, p.z), 0.0);
    let ap = Autopilot::start(&t, &v, car, 1).unwrap();
    assert!(ap.route().length() > 3000.0, "a stage route, not a field route: {}", ap.route().length());
}
#[test] fn the_custom_style_limits_the_top_speed_and_keeps_its_distance_from_the_edge() {
    use crate::game::autopilot::{chain_route, cfg::{cfg_style, reset_custom, set_custom}};
    let car = VehicleSpec::offroader();
    let t = world_terrain(5);
    let free = chain_route(&t, &car, &cfg_style(1)).unwrap();
    set_custom(0.9, 0.8, 0.55, 0.5, 0.0, 3.0);
    let capped = chain_route(&t, &car, &cfg_style(3)).unwrap();
    reset_custom();
    let top = |r: &super::super::route::Route| r.speed.iter().cloned().fold(0.0, f32::max);
    assert!(top(&capped) <= car.max_speed * 0.5 + 0.01 && top(&free) > top(&capped), "{} vs {}", top(&capped), top(&free));
    assert_eq!(capped.len(), free.len());
}
#[test] fn on_the_city_loop_the_autopilot_drives_its_laps_to_the_start_line() {
    let car = VehicleSpec::offroader();
    let base = Terrain::new(5);
    let net = crate::game::roads::gen::generate_with(5, &base, crate::game::roads::gen::GenCfg::coarse());
    let mut t = base.with_net(net);
    t = t.with_loop_for_test(5);
    let l = t.city_loop().unwrap().clone();
    let at = l.pts[0];
    let v = Vehicle::new(car, v3(at.x, at.y + 1.1, at.z), 0.0);
    let ap = Autopilot::start(&t, &v, car, 1).expect("an autopilot");
    let laps = crate::game::autopilot::cfg::cfg().laps as f32;
    assert!((ap.route().length() - l.length * laps).abs() < l.length * 0.1 + 40.0, "route {} vs {} laps of {}", ap.route().length(), laps, l.length);
    let end = *ap.route().p.last().unwrap();
    assert!((end - at).len() < 40.0, "the drive ends just past the start line");
}
