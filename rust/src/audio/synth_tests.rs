use super::*;

/// Goertzel power of `buf` at `freq` (normalised by length^2).
fn power(buf: &[f32], freq: f32) -> f32 {
    let w = std::f32::consts::TAU * freq / RATE as f32;
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in buf { let s0 = x + 2.0 * w.cos() * s1 - s2; s2 = s1; s1 = s0; }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2) / (buf.len() as f32 * buf.len() as f32)
}
fn secs(n: f32) -> usize { (n * RATE as f32) as usize }
/// A car whose wheels ask for `rpm` x 8000 revs, the clutch locked, half throttle.
fn car(rpm: f32) -> AudioParams { AudioParams { machine: Machine::Car, drive_rpm: rpm * 8000.0, clutch: 1.0, load: 0.5, active: true, ..Default::default() } }
fn run(s: &mut Synth, n: f32) -> Vec<f32> { let mut v = vec![0.0; secs(n)]; s.render_f32(&mut v); v }

#[test] fn output_stays_finite_and_in_range_under_random_sweeps() {
    let mut seed = 12345u32; let mut r = move || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for machine in [Machine::None, Machine::Car] {
        let mut s = Synth::new();
        for _ in 0..50 {
            s.set_params(&AudioParams { machine, als: r() > 0.5, cut: r() > 0.8, stalled: r() > 0.9, drive_rpm: r() * 11000.0 - 1000.0, clutch: r(), load: r(), roll: r(), surface: if r() > 0.5 { Surface::Water } else { Surface::Ground }, slip: r(), wind: r(), active: true });
            if r() > 0.5 { s.push(Event::Crash); }
            if r() > 0.5 { s.push(Event::Splash(r())); }
            for x in run(&mut s, 0.2) { assert!(x.is_finite() && x.abs() <= 1.0, "{machine:?}: {x}"); }
        }
    }
    let mut s = Synth::new(); s.set_params(&AudioParams { drive_rpm: f32::NAN, clutch: f32::NAN, load: f32::INFINITY, active: true, ..Default::default() });
    assert!(run(&mut s, 0.1).iter().all(|x| x.is_finite()));
}
#[test] fn the_engine_starts_by_itself_idles_and_follows_the_revs_the_wheels_ask() {
    let mut s = Synth::new(); s.set_params(&AudioParams { machine: Machine::Car, active: true, ..Default::default() });
    run(&mut s, 0.1); assert!(s.engine_rpm() > 20.0 && s.engine_rpm() < 800.0, "cranking: {}", s.engine_rpm());
    run(&mut s, 5.0); assert!((s.engine_rpm() - 950.0).abs() < 350.0, "idling: {}", s.engine_rpm());
    s.set_params(&AudioParams { drive_rpm: 3600.0, ..car(0.45) }); run(&mut s, 2.0);
    assert!((s.engine_rpm() - 3600.0).abs() < 400.0, "driven: {}", s.engine_rpm());
    s.set_params(&AudioParams { drive_rpm: 5400.0, ..car(0.675) }); run(&mut s, 2.0);
    assert!((s.engine_rpm() - 5400.0).abs() < 500.0, "driven faster: {}", s.engine_rpm());
    // the clutch open and the throttle wide: it runs free to the limiter
    s.set_params(&AudioParams { clutch: 0.0, load: 1.0, ..car(0.675) }); run(&mut s, 3.0);
    assert!(s.engine_rpm() > 6800.0 && s.engine_rpm() < 8400.0, "free: {}", s.engine_rpm());
}
#[test] fn inactive_and_muted_fade_to_exact_silence_and_come_back() {
    let mut s = Synth::new(); s.set_params(&car(0.5)); run(&mut s, 0.5);
    let loud = run(&mut s, 0.1).iter().fold(0.0f32, |m, x| m.max(x.abs()));
    assert!(loud > 0.02, "{loud}");
    s.set_params(&AudioParams { active: false, ..car(0.5) });
    run(&mut s, 0.1);
    assert!(run(&mut s, 0.1).iter().all(|x| *x == 0.0));
    s.set_params(&car(0.5)); s.set_muted(true); run(&mut s, 0.1);
    assert!(run(&mut s, 0.1).iter().all(|x| *x == 0.0));
    s.set_muted(false); run(&mut s, 0.1);
    assert!(run(&mut s, 0.1).iter().fold(0.0f32, |m, x| m.max(x.abs())) > 0.02);
}
#[test] fn events_sound_then_decay_and_the_pool_is_capped() {
    let mut s = Synth::new(); s.set_params(&AudioParams { active: true, ..Default::default() });
    run(&mut s, 0.1);
    s.push(Event::Touchdown(1.0));
    let b = run(&mut s, 0.12);
    assert!(b.iter().fold(0.0f32, |m, x| m.max(x.abs())) > 0.02, "a touchdown thump is audible");
    run(&mut s, 1.0);
    assert_eq!(s.voices(), 0);
    assert!(run(&mut s, 0.1).iter().all(|x| x.abs() < 1e-6));
    for _ in 0..20 { s.push(Event::Crash); assert!(s.voices() <= 8); }
    for x in run(&mut s, 0.5) { assert!(x.abs() <= 1.0); }
    s.set_muted(true); s.push(Event::Crash); assert!(s.voices() <= 8);
}
#[test] fn rendering_is_deterministic_and_i16_render_matches() {
    let go = || { let mut s = Synth::new(); s.set_params(&car(0.4)); s.push(Event::Touchdown(0.7)); let mut v = vec![0i16; 4000]; s.render(&mut v); v };
    assert_eq!(go(), go());
    assert!(go().iter().any(|x| *x != 0));
    let mut odd = vec![0i16; 1001]; Synth::new().render(&mut odd);
    let mut empty: Vec<i16> = Vec::new(); Synth::new().render(&mut empty);
}

#[test] fn the_global_synth_survives_empty_buffers_and_renders_what_the_game_feeds() {
    render_into(&mut []);
    feed(&car(0.6), &[Event::Touchdown(0.5)], &Scene::default());
    let mut buf = vec![0i16; 4410];
    render_into(&mut buf);
    render_into(&mut buf);
    assert!(buf.iter().any(|x| *x != 0));
    feed(&AudioParams::default(), &[], &Scene::default()); set_muted(false);
}

fn rms(v: &[f32]) -> f32 { (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt() }
#[test] fn a_crash_is_louder_than_a_loud_engine_and_the_engine_has_mid_band_body() {
    let loud = AudioParams { machine: Machine::Car, drive_rpm: 6500.0, clutch: 1.0, load: 0.9, roll: 0.8, active: true, ..Default::default() };
    let mut a = Synth::new(); a.set_params(&loud); run(&mut a, 0.5); let base = rms(&run(&mut a, 0.1));
    let mut b = Synth::new(); b.set_params(&loud); run(&mut b, 0.5); b.push(Event::Crash); let with = rms(&run(&mut b, 0.1));
    assert!(with > 1.4 * base, "crash {with} vs engine {base}");
    let mut e = Synth::new(); e.set_params(&car(0.5)); run(&mut e, 1.5); let buf = run(&mut e, 1.0);
    let mids: f32 = (3..12).map(|k| power(&buf, 100.0 * k as f32)).sum(); let lows: f32 = (1..3).map(|k| power(&buf, 100.0 * k as f32)).sum();
    assert!(mids > 0.05 * lows, "the engine has body above 300 Hz for small speakers: {mids} vs {lows}");
}
#[test] fn silence_clears_voices_and_the_target() {
    let mut s = Synth::new(); s.set_params(&car(0.7)); s.push(Event::Crash); run(&mut s, 0.1);
    s.silence();
    assert_eq!(s.voices(), 0);
    run(&mut s, 0.2);
    assert!(run(&mut s, 0.1).iter().all(|x| *x == 0.0));
}

#[test] fn master_volume_scales_the_output() {
    let rms_at = |v: f32| { let mut s = Synth::new(); s.set_volume(v); s.set_params(&car(0.6)); run(&mut s, 0.8); rms(&run(&mut s, 0.3)) };
    let full = rms_at(1.0);
    assert!(full > 0.02);
    assert_eq!(rms_at(0.0), 0.0);
    let half = rms_at(0.5);
    assert!((half / full - 0.5).abs() < 0.05, "{half} vs {full}");
    assert_eq!(rms_at(5.0), full); assert_eq!(rms_at(f32::NAN), full);
}

#[test] fn the_stereo_render_puts_a_source_on_its_side_and_the_scene_moves_it() {
    use crate::game::math::v3;
    let run_st = |s: &mut Synth, n: f32| { let mut v = vec![0.0f32; secs(n) * 2]; s.render_stereo_f32(&mut v); v };
    let side = |v: &[f32]| { let (mut l, mut r) = (0.0f32, 0.0f32); for c in v.chunks_exact(2) { l += c[0] * c[0]; r += c[1] * c[1]; } (l, r) };
    let mut s = Synth::new(); s.set_params(&car(0.6));
    // the whole car 8 m to the right of a listener looking along +z (the right is -x)
    let mut sc = Scene::default(); sc.src = [v3(-8.0, 0.0, 0.0); N_SRC]; s.set_scene(&sc);
    run_st(&mut s, 0.6);
    let (l, r) = side(&run_st(&mut s, 0.5));
    assert!(r > 2.0 * l, "on the right: {r} vs {l}");
    sc.src = [v3(8.0, 0.0, 0.0); N_SRC]; s.set_scene(&sc);
    run_st(&mut s, 2.0);
    let (l, r) = side(&run_st(&mut s, 0.5));
    assert!(l > 2.0 * r, "on the left: {l} vs {r}");
    // twice the distance is quieter, and the output stays in range
    let mut near = Synth::new(); near.set_params(&car(0.6)); let mut sc = Scene::default(); sc.src = [v3(0.0, 0.0, 6.0); N_SRC]; near.set_scene(&sc); run_st(&mut near, 0.6);
    let mut far = Synth::new(); far.set_params(&car(0.6)); sc.src = [v3(0.0, 0.0, 48.0); N_SRC]; far.set_scene(&sc); run_st(&mut far, 0.6);
    let (a, b) = (rms(&run_st(&mut near, 0.5)), rms(&run_st(&mut far, 0.5)));
    assert!(a > 3.0 * b && run_st(&mut far, 0.1).iter().all(|x| x.abs() <= 1.0), "near {a} far {b}");
    // a scene with a NaN is ignored
    let mut bad = Scene::default(); bad.listener = v3(f32::NAN, 0.0, 0.0); s.set_scene(&bad);
    assert!(run_st(&mut s, 0.1).iter().all(|x| x.is_finite()));
}

#[test] fn the_environment_adds_a_tail_that_rings_after_the_sound_and_echoes_off_a_wall() {
    use super::reverb::Env;
    let tail = |env: Env| {
        let mut s = Synth::new(); s.set_params(&AudioParams { active: true, ..Default::default() });
        let mut sc = Scene::default(); sc.env = env; s.set_scene(&sc);
        let mut v = vec![0.0f32; secs(3.0) * 2]; s.render_stereo_f32(&mut v);
        s.push(Event::Touchdown(1.0));
        let mut v = vec![0.0f32; secs(2.0) * 2]; s.render_stereo_f32(&mut v);
        let late = &v[secs(1.0) * 2..secs(1.5) * 2]; (late.iter().map(|x| x * x).sum::<f32>() / late.len() as f32).sqrt()
    };
    let dry = tail(Env { level: 0.0, ..Env::default() });
    let wet = tail(Env { rt60: 1.5, level: 0.8, damp_hz: 9000.0, ..Env::default() });
    assert!(dry < 1e-6 && wet > 1e-4, "a room rings after the sound has ended: {wet} vs {dry}");
}
