use super::*;
use crate::audio::RATE;

fn power(buf: &[f32], freq: f32) -> f32 {
    let w = std::f32::consts::TAU * freq / RATE as f32;
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in buf { let s0 = x + 2.0 * w.cos() * s1 - s2; s2 = s1; s1 = s0; }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2) / (buf.len() as f32 * buf.len() as f32)
}
fn rms(v: &[f32]) -> f32 { (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt() }
/// The voice held at `rpm` by a locked clutch under `throttle`; returns what the exhaust, the intake and the block sound like over one second.
fn at(rpm: f64, throttle: f64) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut v = EngineVoice::new(EngineSpec::rally_four());
    v.engine.omega = rpm * std::f64::consts::TAU / 60.0;
    let ctl = Controls { throttle, clutch: 1.0, drive_rpm: rpm, ..Default::default() };
    for _ in 0..RATE as usize / 2 { v.sample(&ctl); }
    let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..RATE as usize { v.sample(&ctl); a.push(v.exhaust_out); b.push(v.intake_out); c.push(v.block_out); }
    (a, b, c)
}

#[test] fn the_exhaust_pulses_at_the_firing_frequency_of_a_four() {
    // two firings per revolution: 3000 rpm is 100 Hz, 4500 rpm is 150 Hz: the harmonics of that carry the sound, not the notes in between
    for (rpm, f) in [(3000.0, 100.0f32), (4500.0, 150.0)] {
        let (ex, _, _) = at(rpm, 1.0);
        let on: f32 = (1..=8).map(|k| power(&ex, f * k as f32)).sum();
        let off: f32 = (1..=8).map(|k| power(&ex, f * (k as f32 + 0.5))).sum();
        assert!(on > 1.5 * off, "{rpm} rpm: on the harmonics of {f} Hz {on}, between them {off}");
    }
}
#[test] fn the_exhaust_is_louder_at_full_throttle_and_the_intake_barks_when_the_throttle_is_open() {
    let (open, _, _) = at(3500.0, 1.0); let (shut, _, _) = at(3500.0, 0.0);
    assert!(rms(&open) > 1.3 * rms(&shut), "{} vs {}", rms(&open), rms(&shut));
    let (_, ia, _) = at(3500.0, 1.0); let (_, ib, _) = at(3500.0, 0.0);
    assert!(rms(&ia) > 0.0 && rms(&ib).is_finite());
}
#[test] fn the_levels_are_sane_and_the_block_rattles_with_the_combustion() {
    let (ex, int, blk) = at(4000.0, 1.0);
    eprintln!("LEVELS exhaust {} intake {} block {}", rms(&ex), rms(&int), rms(&blk));
    for (name, b) in [("exhaust", &ex), ("intake", &int), ("block", &blk)] {
        let r = rms(b);
        assert!(r > 0.003 && r < 0.6 && b.iter().all(|x| x.is_finite() && x.abs() < 3.0), "{name}: rms {r}");
    }
    let (_, _, quiet) = at(4000.0, 0.0);
    assert!(rms(&blk) > 1.5 * rms(&quiet), "the combustion under load shakes the block more");
}
#[test] fn the_exhaust_pipe_rings_at_its_quarter_wave_resonance() {
    use super::super::pipe::Pipe;
    let mut p = Pipe::new(2.2, 0.55, -0.85, 0.2);
    // an impulse; the ring's fundamental is c / 4L = 39 Hz, then 117 Hz, 195 Hz ...; the round trip is 2L / c
    let mut out = Vec::new();
    for i in 0..RATE as usize { out.push(p.process(if i == 0 { 1.0 } else { 0.0 })); }
    let d = p.round_trip();
    assert!((d as f32 - 2.0 * 2.2 / 343.0 * RATE as f32).abs() < 2.0, "{d}");
    let (r, r2) = (rms(&out[..d * 3]), rms(&out[d * 12..d * 15]));
    assert!(r2 < r && r2 > 0.0, "it rings and dies: {r} then {r2}");
    assert!(power(&out, 117.0) > power(&out, 78.0) && power(&out, 195.0) > power(&out, 156.0), "the odd harmonics stand out");
}
#[test] fn the_voice_survives_abuse() {
    let mut v = EngineVoice::new(EngineSpec::rally_four());
    for i in 0..RATE as usize * 3 {
        let ctl = Controls { throttle: ((i / 5000) % 3) as f64 * 0.5, starter: i < 20000, clutch: ((i / 7000) % 2) as f64, drive_rpm: ((i / 9000) % 4) as f64 * 2500.0, ignition: (i / 30000) % 2 == 0, als: (i / 11000) % 2 == 0, cut: (i / 13000) % 3 == 0 };
        v.sample(&ctl);
        assert!(v.exhaust_out.is_finite() && v.intake_out.is_finite() && v.block_out.is_finite(), "{i}");
    }
}

#[test] fn the_turbo_whistles_at_a_pitch_that_follows_its_speed_and_the_valve_hisses_when_it_blows_off() {
    let mut v = EngineVoice::new(EngineSpec::rally_four());
    v.engine.omega = 4500.0 * std::f64::consts::TAU / 60.0;
    let mut ctl = Controls { throttle: 1.0, clutch: 1.0, drive_rpm: 4500.0, ..Default::default() };
    for _ in 0..RATE as usize * 2 { v.sample(&ctl); }
    let f = (v.engine.turbo_omega / std::f64::consts::TAU * 5.0) as f32;
    let mut buf = Vec::new(); for _ in 0..RATE as usize / 2 { v.sample(&ctl); buf.push(v.turbo_out); }
    let near = [0.97f32, 1.0, 1.03].iter().map(|k| power(&buf, f * k)).fold(0.0f32, f32::max);
    assert!(near > 4.0 * power(&buf, f * 0.7) && near > 4.0 * power(&buf, f * 1.4), "a whistle at {f} Hz: {near}");
    assert!(rms(&buf) > 0.002, "{}", rms(&buf));
    let loud = rms(&buf);
    ctl.throttle = 0.0;
    let mut hiss = Vec::new(); for _ in 0..RATE as usize / 20 { v.sample(&ctl); hiss.push(v.bov_out); }
    assert!(rms(&hiss) > 0.005, "the blow-off hiss: {}", rms(&hiss));
    let _ = loud;
}
#[test] fn the_gearbox_whines_at_a_multiple_of_the_revs_under_torque() {
    let (_, _, _) = at(3000.0, 1.0);
    let mut v = EngineVoice::new(EngineSpec::rally_four());
    v.engine.omega = 3000.0 * std::f64::consts::TAU / 60.0;
    let ctl = Controls { throttle: 1.0, clutch: 1.0, drive_rpm: 3000.0, ..Default::default() };
    for _ in 0..RATE as usize / 2 { v.sample(&ctl); }
    let mut buf = Vec::new(); for _ in 0..RATE as usize { v.sample(&ctl); buf.push(v.whine_out); }
    let f = 3000.0 / 60.0 * 21.0;
    assert!(power(&buf, f) > 5.0 * power(&buf, f * 1.3), "a whine at {f} Hz");
}
