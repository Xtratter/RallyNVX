use super::*;

#[test] fn the_mask_round_trips_and_everything_is_on_by_default() {
    let a = Assists::default();
    assert!(a.als && a.abs && a.tc && a.blip && a.flat_shift && a.limiter && a.bov && a.launch && a.mask() == 0xFF);
    for m in 0..256 { assert_eq!(Assists::from_mask(m).mask(), m); }
    let off = Assists::from_mask(0b1111101); assert!(!off.abs && off.als && off.tc);
    assert_eq!(Assists::from_mask(0xFFFF_FFFF), a, "extra bits are ignored");
}
#[test] fn tuning_is_clamped_to_its_range_ignores_garbage_and_scales_the_parts() {
    let mut t = Tuning::default(); for i in 0..TUNING_PARTS { assert_eq!(t.get(i), 1.0, "part {i} starts as built"); }
    t.set(0, 9.0); assert_eq!(t.engine, 2.0); t.set(0, -1.0); assert_eq!(t.engine, 0.3);
    t.set(6, f32::NAN); assert_eq!(t.gearing, 1.0); t.set(99, 1.5);
    for i in 0..TUNING_PARTS { let (lo, hi) = Tuning::range(i); assert!(lo < 1.0 && hi > 1.0, "{i}: 1.0 is inside the range"); t.set(i, 1.0); assert_eq!(t.get(i), 1.0); }
    let spec = VehicleSpec::default_car();
    let mut t = Tuning::default(); t.set(2, 1.5); t.set(3, 1.2); t.set(4, 0.8); t.set(5, 2.0);
    let s = t.apply(&spec);
    assert!((s.brake_force / spec.brake_force - 1.5).abs() < 1e-5 && (s.mu / spec.mu - 1.2).abs() < 1e-5 && (s.spring / spec.spring - 0.8).abs() < 1e-5);
    assert!((s.aero_base.cla_rear / spec.aero_base.cla_rear - 2.0).abs() < 1e-5 || spec.aero_base.cla_rear == 0.0);
    assert_eq!(Tuning::default().apply(&spec).mass, spec.mass);
}

#[test] fn the_differential_parts_are_tuning_parts_with_their_own_ranges() {
    assert_eq!(TUNING_PARTS, 19);
    let mut t = Tuning::default();
    t.set(7, 9.0); assert_eq!(t.diff_front, 2.0); t.set(8, -1.0); assert_eq!(t.diff_rear, 0.0); t.set(9, 0.5); assert_eq!(t.diff_centre, 0.5);
    t.set(10, 9.0); assert_eq!(t.split, 1.4); t.set(10, 0.0); assert_eq!(t.split, 0.6); t.set(10, f32::NAN); assert_eq!(t.split, 0.6, "garbage is ignored");
    let d = t.diffs(); assert_eq!((d.front, d.rear, d.centre, d.split), (2.0, 0.0, 0.5, 0.6));
}

#[test] fn the_brake_bias_is_a_tuning_part_that_reaches_the_vehicle() {
    assert_eq!(TUNING_PARTS, 19);
    let mut t = Tuning::default();
    t.set(12, 9.0); assert_eq!(t.tyre_bar, 1.25); t.set(12, 0.0); assert_eq!(t.tyre_bar, 0.8); t.set(12, f32::NAN); assert_eq!(t.tyre_bar, 0.8, "garbage is ignored");
    t.set(11, 9.0); assert_eq!(t.bias, 1.25); t.set(11, 0.0); assert_eq!(t.bias, 0.7); t.set(11, f32::NAN); assert_eq!(t.bias, 0.7, "garbage is ignored");
    let mut w = crate::game::world::World::test_track(VehicleSpec::default_car());
    w.tuning.set(11, 1.1); w.step(&crate::game::input::Input::default(), 1.0 / 60.0);
    assert!((w.vehicle.brake_bias - 0.62 * 1.1).abs() < 1e-4, "{}", w.vehicle.brake_bias);
    w.tuning.set(11, 1.25); w.step(&crate::game::input::Input::default(), 1.0 / 60.0);
    assert!(w.vehicle.brake_bias <= 0.8 && w.vehicle.brake_bias > 0.7, "{}", w.vehicle.brake_bias);
}

#[test] fn the_tyre_pressure_and_the_air_reach_the_vehicle() {
    let mut w = crate::game::world::World::test_track(VehicleSpec::default_car());
    w.tuning.set(12, 0.9); w.step(&crate::game::input::Input::default(), 1.0 / 60.0);
    assert!((w.vehicle.tyre_bar - 0.9).abs() < 1e-5, "{}", w.vehicle.tyre_bar);
    let wv = w.climate.weather();
    let air = crate::raster::smoke::Air::from_sky(w.sun_dir().y, wv.cloud, wv.rain, wv.fog).celsius;
    assert!((w.vehicle.air_c - air).abs() < 1e-3 && w.vehicle.air_c > -30.0 && w.vehicle.air_c < 40.0, "{} against {air}", w.vehicle.air_c);
}
#[test] fn the_chassis_parts_are_tuning_parts_and_reach_the_vehicle() {
    let mut t = Tuning::default();
    for i in 13..19 { assert_eq!(t.get(i), 1.0, "part {i} starts as built"); t.set(i, 9.0); assert_eq!(t.get(i), 2.0); t.set(i, -1.0); assert_eq!(t.get(i), 0.0); t.set(i, f32::NAN); assert_eq!(t.get(i), 0.0, "garbage is ignored"); t.set(i, 1.0); }
    let mut w = crate::game::world::World::test_track(VehicleSpec::default_car());
    w.tuning.set(13, 0.0); w.tuning.set(14, 2.0); w.tuning.set(15, 0.0); w.tuning.set(17, 2.0);
    w.step(&crate::game::input::Input::default(), 1.0 / 60.0);
    let c = w.vehicle.chassis;
    assert_eq!((c.arb_front, c.arb_rear), (0.0, 2.0 * crate::game::chassis::BASE.arb_rear));
    assert!(c.camber_front == 0.0 && c.toe_front > crate::game::chassis::BASE.toe_front, "{c:?}");
    assert_eq!(Tuning::default().chassis(), crate::game::chassis::BASE);
}
mod physics {
    use super::*;
    use crate::game::{input::Input, world::World};
    fn track() -> World { World::test_track(VehicleSpec::default_car()) }
    fn stop_distance(w: &mut World, speed: f32) -> f32 {
        let f = w.vehicle.forward(); w.vehicle.vel = f * speed;
        let p0 = w.vehicle.pos;
        for _ in 0..60 * 12 { w.step(&Input { brake: true, brake_amt: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() < 0.2 { break; } }
        (w.vehicle.pos - p0).len()
    }
    fn time_to(w: &mut World, speed: f32) -> f32 {
        for k in 0..60 * 20 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() >= speed { return k as f32 / 60.0; } }
        99.0
    }
    #[test] fn without_abs_a_hard_stop_is_longer() {
        // on gravel the brakes are stronger than the grip, so a locked wheel costs distance (on tarmac the car's brakes are weaker than its tyres); the car gets to speed on its own,
        // with its engine and its gear (a speed set by hand at the idle would drag the wheels back through the clutch)
        let go = |abs: bool| {
            let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Gravel); w.assists.abs = abs;
            for _ in 0..60 * 9 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
            let (p0, v0) = (w.vehicle.pos, w.vehicle.vel.len());
            for _ in 0..60 * 20 { w.step(&Input { brake: true, brake_amt: 1.0, ..Default::default() }, 1.0 / 60.0); if w.vehicle.vel.len() < 0.2 { break; } }
            ((w.vehicle.pos - p0).len(), v0)
        };
        let ((a, va), (b, vb)) = (go(true), go(false));
        assert!(va > 20.0 && (va - vb).abs() < 1.0, "the same speed to brake from: {va} {vb}");
        assert!(b > a * 1.04, "ABS on {a} m, off {b} m");
    }
    /// The largest slip over the peak that any wheel reaches in a full-throttle launch.
    fn peak_spin(tc: bool, surface: crate::game::surface::Surface, secs: f32) -> f32 {
        let mut w = track(); w.vehicle.force_surface = Some(surface); w.assists.tc = tc;
        let mut peak = 0.0f32;
        for _ in 0..(secs * 60.0) as usize { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); peak = peak.max(w.vehicle.slip.iter().cloned().fold(0.0, f32::max)); }
        peak
    }
    #[test] fn traction_control_limits_the_wheel_spin_of_a_launch_on_gravel() {
        let (on, off) = (peak_spin(true, crate::game::surface::Surface::Gravel, 3.0), peak_spin(false, crate::game::surface::Surface::Gravel, 3.0));
        assert!(off > 1.5 && on < off * 0.8, "gravel launch: the wheels spin to {on} of the peak with TC, {off} without");
    }
    #[test] fn the_tuning_changes_how_the_car_goes_and_stops() {
        let (mut base, mut weak, mut strong) = (track(), track(), track());
        weak.tuning.set(0, 0.5); strong.tuning.set(0, 1.5);
        let (tb, tw, ts) = (time_to(&mut base, 30.0), time_to(&mut weak, 30.0), time_to(&mut strong, 30.0));
        assert!(tw > tb * 1.1 && ts < tb, "engine 50 % {tw} s, 100 % {tb} s, 150 % {ts} s to 108 km/h");
        let (mut b1, mut b2) = (track(), track());
        b2.tuning.set(2, 2.0);
        b1.assists.abs = false; b2.assists.abs = false;
        let (d1, d2) = (stop_distance(&mut b1, 30.0), stop_distance(&mut b2, 30.0));
        assert!(d1 > 0.0 && d2 > 0.0 && d1.is_finite() && d2.is_finite(), "{d1} {d2}");
        let mut grip = track(); grip.tuning.set(3, 1.4); let mut slick = track(); slick.tuning.set(3, 0.7);
        assert!(stop_distance(&mut grip, 33.0) < stop_distance(&mut slick, 33.0), "more grip stops shorter");
    }
    #[test] fn the_switches_reach_the_drivetrain() {
        let mut w = track(); w.assists.limiter = false; w.assists.flat_shift = false; w.assists.blip = false; w.tuning.set(6, 1.2); w.tuning.set(1, 0.0);
        w.step(&Input::default(), 1.0 / 60.0);
        let o = w.drivetrain.opts;
        assert!(!o.limiter && !o.flat_shift && !o.blip && (o.gearing - 1.2).abs() < 1e-6 && o.turbo == 0.0);
        assert!(!w.vehicle.abs_on == false && w.vehicle.tc_on);
    }
    #[test] fn a_car_on_water_does_not_run_away() {
        let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Water);
        for _ in 0..60 * 8 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        assert!(w.vehicle.vel.len().is_finite() && w.vehicle.vel.len() < 40.0 && w.drivetrain.last.rpm < 8300.0 && w.vehicle.w.iter().all(|x| x.is_finite() && x.abs() < 600.0), "{} m/s {} rpm {:?}", w.vehicle.vel.len(), w.drivetrain.last.rpm, w.vehicle.w);
    }
    #[test] fn a_spinning_launch_does_not_chatter() {
        let mut w = track(); w.vehicle.force_surface = Some(crate::game::surface::Surface::Gravel); w.assists.tc = false;
        let mut t = Vec::new();
        for k in 0..60 * 4 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); if k > 60 { t.push(w.drivetrain.last.wheel_torque); } }
        let mean = t.iter().sum::<f32>() / t.len() as f32;
        let var = t.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / t.len() as f32;
        assert!(var.sqrt() < mean.abs() * 0.35 + 50.0, "wheel torque mean {mean}, std {} N m", var.sqrt());
    }
    #[test] fn the_centre_split_sends_the_torque_where_the_driver_set_it() {
        let share = |part10: f32| { let mut w = track(); w.tuning.set(10, part10);
            for _ in 0..90 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
            let q = w.vehicle.drive_torque.unwrap(); (q[0] + q[1]) / q.iter().sum::<f32>() };
        let (even, front, rear) = (share(1.0), share(1.4), share(0.6));
        assert!((even - 0.5).abs() < 0.06 && (front - 0.7).abs() < 0.08 && (rear - 0.3).abs() < 0.08, "{even} {front} {rear}");
    }
    #[test] fn the_wheel_torques_add_up_to_the_engines_wheel_torque() {
        let mut w = track();
        for _ in 0..120 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        let sum: f32 = w.vehicle.drive_torque.unwrap().iter().sum();
        let mean = w.drivetrain.last.wheel_torque;
        assert!((sum - mean).abs() < 0.2 * mean.abs().max(100.0), "the last substep {sum} against the frame's mean {mean}");
    }
}
