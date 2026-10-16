use super::*;

fn energy_decay(rt: f32) -> (f32, f32) {
    let mut r = Reverb::new();
    r.set_env(&Env { rt60: rt, level: 1.0, damp_hz: 16000.0, echoes: [(0.0, 0.0, 0.0); ECHOES] });
    // let the smoothing settle with silence, then an impulse
    for _ in 0..(RATE as usize * 30) { r.process(0.0); }
    r.process(1.0);
    let rate = RATE as usize;
    let mut e = vec![0.0f32; 12 * rate];
    for s in e.iter_mut() { let (l, rr) = r.process(0.0); *s = l * l + rr * rr; }
    let win = |a: f32, b: f32| e[(a * rate as f32) as usize..(b * rate as f32) as usize].iter().sum::<f32>();
    (win(0.5, 0.75), win(0.5 + rt * 0.5, 0.75 + rt * 0.5))   // the energy at 0.5 s and half a decay time later
}

#[test] fn the_tail_decays_by_the_asked_time_and_longer_asks_take_longer() {
    // after rt/2 the energy is 30 dB lower (the amplitude 60 dB over rt): the ratio of the windows is about 1000 (allowing for the window and the mixing)
    let (a, b) = energy_decay(1.0);
    let db = 10.0 * (a / b.max(1e-30)).log10();
    assert!(db > 18.0 && db < 45.0, "{db} dB over half a decay time of 1 s");
    let (a2, b2) = energy_decay(3.0);
    let db2 = 10.0 * (a2 / b2.max(1e-30)).log10();
    assert!(db2 > 18.0 && db2 < 45.0, "{db2} dB over half of 3 s");
}
#[test] fn it_is_stable_finite_and_silent_without_input() {
    let mut r = Reverb::new();
    r.set_env(&Env { rt60: f32::MAX, level: f32::NAN, damp_hz: -5.0, echoes: [(f32::NAN, 2.0, 9.0); ECHOES] });
    let mut seed = 5u32;
    for i in 0..RATE as usize * 4 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let (l, rr) = r.process(if i < 2000 { (seed >> 8) as f32 / 8_388_608.0 - 1.0 } else { 0.0 });
        assert!(l.is_finite() && rr.is_finite() && l.abs() < 50.0 && rr.abs() < 50.0, "step {i}: {l} {rr}");
    }
    let mut q = Reverb::new();
    for _ in 0..1000 { assert_eq!(q.process(0.0), (0.0, 0.0)); }
}
#[test] fn an_echo_arrives_when_the_rays_say_and_on_its_side() {
    let mut r = Reverb::new();
    let mut env = Env { rt60: 0.2, level: 0.0, ..Env::default() };
    env.echoes[1] = (0.1, 0.8, 1.0);   // a wall on the right, 17 m away: 0.1 s there and back
    r.set_env(&env);
    for _ in 0..RATE as usize { r.process(0.0); }
    r.process(1.0);
    let (mut first, mut lsum, mut rsum) = (None, 0.0f32, 0.0f32);
    for i in 0..RATE as usize / 2 {
        let (l, rr) = r.process(0.0);
        if first.is_none() && rr.abs() > 0.05 { first = Some(i); }
        lsum += l * l; rsum += rr * rr;
    }
    let want = (0.1 * RATE as f32) as i32;
    assert!((first.expect("the echo") as i32 - want).abs() < 600, "{first:?} vs {want}");
    assert!(rsum > 20.0 * lsum, "on the right: {rsum} vs {lsum}");
}
