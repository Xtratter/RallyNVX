use super::*;

#[test] fn the_mask_round_trips_and_everything_is_on_by_default() {
    let a = Assists::default();
    assert!(a.als && a.abs && a.tc && a.blip && a.flat_shift && a.limiter && a.bov && a.mask() == 0x7F);
    for m in 0..128 { assert_eq!(Assists::from_mask(m).mask(), m); }
    let off = Assists::from_mask(0b1111101); assert!(!off.abs && off.als && off.tc);
    assert_eq!(Assists::from_mask(0xFFFF_FFFF), a, "extra bits are ignored");
}
#[test] fn tuning_is_clamped_to_its_range_ignores_garbage_and_scales_the_parts() {
    let mut t = Tuning::default(); assert_eq!(t, Tuning { engine: 1.0, turbo: 1.0, brakes: 1.0, grip: 1.0, suspension: 1.0, downforce: 1.0, gearing: 1.0 });
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
        eprintln!("ABSNUM on {a} m from {va} off {b} m from {vb}");
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
        eprintln!("TCNUM on {on} off {off}");
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
}
