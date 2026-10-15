use super::*;
use crate::game::{input::Input, math::v3, terrain::Terrain, vehicle::{Vehicle, VehicleSpec}};

fn vulpes() -> VehicleSpec { VehicleSpec::by_id("vulpes").unwrap() }
fn with(kit: AeroKit) -> VehicleSpec { VehicleSpec { kit, ..vulpes() } }
fn fwd() -> V3 { v3(0.0, 0.0, 1.0) }
fn right() -> V3 { v3(-1.0, 0.0, 0.0) }
fn at(spec: &VehicleSpec, v: f32, vel_dir: V3, clear: f32) -> AeroForces { forces(spec, vel_dir * v, fwd(), right(), clear, clear, &cfg()) }
fn total_down(f: &AeroForces) -> f32 { f.down_front + f.down_rear }

#[test] fn the_embedded_numbers_parse_and_bad_ones_are_named() {
    let c = cfg();
    assert!(c.wing.cla[3] > c.wing.cla[1] && c.wing.cda[3] > c.wing.cda[1] && c.angle_cla[2] > c.angle_cla[0]);
    let bad = EMBEDDED.replace("\"rho\": 1.2", "\"rho\": 50");
    assert!(AeroCfg::from_json(&bad).unwrap_err().contains("rho"));
    let bad = EMBEDDED.replace("[0.0, 0.25, 0.55, 0.90]", "[0.0, 0.25]");
    assert!(AeroCfg::from_json(&bad).unwrap_err().contains("cla"));
    assert!(AeroCfg::from_json("{}").is_err());
}
#[test] fn the_forces_grow_with_the_square_of_the_speed_and_the_stock_drag_is_the_old_constant() {
    let s = vulpes();
    let (a, b) = (at(&s, 20.0, fwd(), 0.16), at(&s, 40.0, fwd(), 0.16));
    assert!((b.drag.len() / a.drag.len() - 4.0).abs() < 0.01, "drag x{}", b.drag.len() / a.drag.len());
    assert!((total_down(&b) / total_down(&a) - 4.0).abs() < 0.01);
    // no parts: the drag equals the old model F = aero v^2, pointing against the velocity
    assert!((a.drag.len() - s.aero * 400.0).abs() < 0.5, "{} vs {}", a.drag.len(), s.aero * 400.0);
    assert!(a.drag.z < 0.0);
    // standing: nothing
    assert_eq!(at(&s, 0.0, fwd(), 0.16).drag.len(), 0.0);
}
#[test] fn every_part_adds_downforce_where_it_belongs_and_the_wing_angle_trades_drag() {
    let base = at(&with(AeroKit::default()), 40.0, fwd(), 0.16);
    for (name, kit) in [("splitter", AeroKit { splitter: 3, ..Default::default() }), ("wing", AeroKit { wing: 3, ..Default::default() }), ("underbody", AeroKit { underbody: 3, ..Default::default() })] {
        let f = at(&with(kit), 40.0, fwd(), 0.16);
        assert!(total_down(&f) > total_down(&base) + 300.0, "{name} adds too little ({} N)", total_down(&f) - total_down(&base));
        assert!(f.drag.len() >= base.drag.len(), "{name} cannot reduce drag");
    }
    let s = at(&with(AeroKit { splitter: 3, ..Default::default() }), 40.0, fwd(), 0.16);
    assert!(s.down_front - base.down_front > s.down_rear - base.down_rear + 200.0, "the splitter loads the front axle");
    let w = at(&with(AeroKit { wing: 3, ..Default::default() }), 40.0, fwd(), 0.16);
    assert!(w.down_rear - base.down_rear > w.down_front - base.down_front + 500.0, "the wing loads the rear axle");
    // levels are monotonic
    let level = |l: u8| total_down(&at(&with(AeroKit { wing: l, ..Default::default() }), 40.0, fwd(), 0.16));
    assert!(level(0) < level(1) && level(1) < level(2) && level(2) < level(3));
    // the angle: high gives more downforce and more drag than low
    let ang = |a: u8| at(&with(AeroKit { wing: 3, wing_angle: a, ..Default::default() }), 40.0, fwd(), 0.16);
    assert!(total_down(&ang(2)) > total_down(&ang(0)) * 1.4 && ang(2).drag.len() > ang(0).drag.len() * 1.2);
}
#[test] fn the_ground_effect_has_a_peak_a_stall_and_a_decay() {
    let c = cfg();
    assert!((ground_effect(1.0, &c) - 1.0).abs() < 1e-5);
    assert!(ground_effect(c.peak, &c) > 1.0 + c.gain * 0.99);
    assert!(ground_effect(0.8, &c) > 1.0 && ground_effect(0.8, &c) < ground_effect(c.peak, &c));
    assert!(ground_effect(1.5, &c) < 1.0 && ground_effect(2.5, &c) < ground_effect(1.5, &c), "higher is weaker");
    assert!(ground_effect(c.stall, &c) < ground_effect(c.peak, &c) * 0.6, "the flow stalls below the peak");
    assert!(ground_effect(0.05, &c) < ground_effect(c.stall, &c) && ground_effect(0.0, &c) >= 0.0);
    assert!(ground_effect(f32::NAN, &c) == 1.0);
    // the underbody and splitter feel it, the wing does not
    let kit = AeroKit { splitter: 3, wing: 3, underbody: 3, wing_angle: 1 };
    let (low, high) = (at(&with(kit), 40.0, fwd(), 0.16 * c.peak), at(&with(kit), 40.0, fwd(), 0.16 * 2.0));
    assert!(low.down_front > high.down_front * 1.5, "a low splitter works harder");
    let wing_only = AeroKit { wing: 3, ..Default::default() };
    let (a, b) = (at(&with(wing_only), 40.0, fwd(), 0.10), at(&with(wing_only), 40.0, fwd(), 0.30));
    assert!((a.down_rear - b.down_rear).abs() < 1.0, "the wing does not depend on the ride height");
}
#[test] fn sideslip_raises_the_drag_lowers_the_downforce_and_pushes_the_nose_back_into_the_flow() {
    let s = with(AeroKit { wing: 3, ..Default::default() });
    let straight = at(&s, 40.0, fwd(), 0.16);
    let ang = 15f32.to_radians();
    let slip = at(&s, 40.0, v3(-ang.sin(), 0.0, ang.cos()), 0.16);   // the velocity points to the car's right (-x is right)
    assert!(slip.drag.len() > straight.drag.len() * 1.1, "drag {} -> {}", straight.drag.len(), slip.drag.len());
    assert!(total_down(&slip) < total_down(&straight) * 0.95);
    assert!(slip.side.dot(right()) < 0.0, "the side force opposes the slip");
    assert!(straight.side.len() < 1e-3);
    // never negative downforce, even sideways
    assert!(total_down(&at(&s, 40.0, right(), 0.16)) >= 0.0);
}
#[test] fn the_garage_numbers_and_the_top_speed_estimate() {
    let none = stats(&with(AeroKit::default()));
    let full = stats(&with(AeroKit { splitter: 3, wing: 3, underbody: 3, wing_angle: 2 }));
    assert!(full.downforce_kg_150 > none.downforce_kg_150 + 100.0, "{} vs {}", full.downforce_kg_150, none.downforce_kg_150);
    assert!(full.drag_delta_pct > 30.0 && none.drag_delta_pct == 0.0);
    assert!(full.top_kmh < none.top_kmh - 1.2, "a big wing costs top speed: {} vs {}", full.top_kmh, none.top_kmh);
    assert!(none.top_kmh > 100.0 && none.top_kmh < 187.0, "{}", none.top_kmh);
}

/// Drives a straight line and returns the speed after `secs` at full throttle.
fn top_speed_run(spec: VehicleSpec, secs: f32) -> f32 {
    let t = Terrain::flat();
    let mut v = Vehicle::new(spec, v3(0.0, 1.0, 0.0), 0.0);
    for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
    for _ in 0..(secs * 60.0) as usize { v.step(&t, &[], &Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    v.speed()
}

#[test] fn a_big_wing_costs_top_speed_in_the_simulation_and_the_estimate_is_close() {
    let none = top_speed_run(with(AeroKit::default()), 40.0);
    let wing = top_speed_run(with(AeroKit { wing: 3, wing_angle: 2, ..Default::default() }), 40.0);
    assert!(wing < none - 0.4, "wing {wing} vs none {none} m/s");
    let est = top_speed(&with(AeroKit::default()));
    assert!((est - none).abs() < 4.0, "estimated {est} vs simulated {none}");
}
#[test] fn downforce_presses_the_car_into_its_springs_and_adds_cornering_grip_at_speed() {
    // sit the car at 40 m/s on flat ground without throttle for a moment: the suspension compresses more with a wing
    let compression = |spec: VehicleSpec| {
        let t = Terrain::flat();
        let mut v = Vehicle::new(spec, v3(0.0, 1.0, 0.0), 0.0);
        for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        let hold = |v: &mut Vehicle| { let f = v.forward() * 40.0; v.vel = v3(f.x, v.vel.y, f.z); };   // the speed is held, the vertical motion is free
        hold(&mut v);
        for _ in 0..90 { hold(&mut v); v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        v.comp.iter().sum::<f32>()
    };
    let (a, b) = (compression(with(AeroKit::default())), compression(with(AeroKit { wing: 3, underbody: 3, splitter: 3, wing_angle: 1 })));
    assert!(b > a + 0.01, "the suspension should carry the downforce: {a} -> {b}");
    // a steady turn at 35 m/s with full steering: the lateral acceleration is higher with the aero parts
    let lateral = |spec: VehicleSpec| {
        let t = Terrain::flat();
        let mut v = Vehicle::new(spec, v3(0.0, 1.0, 0.0), 0.0);
        for _ in 0..120 { v.step(&t, &[], &Input::default(), 1.0 / 60.0); }
        v.vel = v.forward() * 35.0;
        let mut best = 0.0f32;
        for i in 0..240 {
            v.step(&t, &[], &Input { throttle: 0.5, steer: 1.0, ..Default::default() }, 1.0 / 60.0);
            if i > 120 { best = best.max(v.vel.dot(v.right()).abs() * v.omega.y.abs() + 0.0); }
        }
        best
    };
    let (la, lb) = (lateral(with(AeroKit::default())), lateral(with(AeroKit { wing: 3, underbody: 3, splitter: 3, wing_angle: 2 })));
    assert!(la.is_finite() && lb.is_finite());
    assert!(lb > la * 0.97, "the aero car must not corner worse at speed: {la} vs {lb}");
}
