use super::*;

/// Goertzel power of `buf` at `freq` (normalised by length^2).
fn power(buf: &[f32], freq: f32) -> f32 {
    let w = std::f32::consts::TAU * freq / RATE as f32;
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in buf { let s0 = x + 2.0 * w.cos() * s1 - s2; s2 = s1; s1 = s0; }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2) / (buf.len() as f32 * buf.len() as f32)
}
fn secs(n: f32) -> usize { (n * RATE as f32) as usize }
fn car(rpm: f32) -> AudioParams { AudioParams { machine: Machine::Car, rpm, load: 0.5, active: true, ..Default::default() } }
fn run(s: &mut Synth, n: f32) -> Vec<f32> { let mut v = vec![0.0; secs(n)]; s.render_f32(&mut v); v }

#[test] fn output_stays_finite_and_in_range_under_random_sweeps() {
    let mut seed = 12345u32; let mut r = move || { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); (seed >> 8) as f32 / 16_777_216.0 };
    for machine in [Machine::None, Machine::Car] {
        let mut s = Synth::new();
        for _ in 0..50 {
            s.set_params(&AudioParams { machine, rpm: r() * 1.4 - 0.2, load: r(), roll: r(), surface: if r() > 0.5 { Surface::Water } else { Surface::Ground }, slip: r(), wind: r(), active: true });
            if r() > 0.5 { s.push(Event::Crash); }
            if r() > 0.5 { s.push(Event::Splash(r())); }
            for x in run(&mut s, 0.2) { assert!(x.is_finite() && x.abs() <= 1.0, "{machine:?}: {x}"); }
        }
    }
    let mut s = Synth::new(); s.set_params(&AudioParams { rpm: f32::NAN, load: f32::INFINITY, active: true, ..Default::default() });
    assert!(run(&mut s, 0.1).iter().all(|x| x.is_finite()));
}
#[test] fn engine_fundamental_follows_rpm() {
    assert!(engine_freq(0.2) < engine_freq(0.5) && engine_freq(0.5) < engine_freq(0.9));
    let (lo, hi) = (engine_freq(0.2), engine_freq(0.8));
    let mut a = Synth::new(); a.set_params(&car(0.2)); run(&mut a, 0.6); let la = run(&mut a, 1.0);
    let mut b = Synth::new(); b.set_params(&car(0.8)); run(&mut b, 0.6); let lb = run(&mut b, 1.0);
    assert!(power(&la, lo) > 3.0 * power(&la, hi), "low rpm: {} vs {}", power(&la, lo), power(&la, hi));
    assert!(power(&lb, hi) > 3.0 * power(&lb, lo), "high rpm: {} vs {}", power(&lb, hi), power(&lb, lo));
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
    feed(&car(0.6), &[Event::Touchdown(0.5)]);
    let mut buf = vec![0i16; 4410];
    render_into(&mut buf);
    render_into(&mut buf);
    assert!(buf.iter().any(|x| *x != 0));
    feed(&AudioParams::default(), &[]); set_muted(false);
}

fn rms(v: &[f32]) -> f32 { (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt() }
#[test] fn a_crash_is_louder_than_a_loud_engine_and_the_engine_has_mid_band_body() {
    let loud = AudioParams { machine: Machine::Car, rpm: 0.9, load: 0.9, roll: 0.8, active: true, ..Default::default() };
    let mut a = Synth::new(); a.set_params(&loud); run(&mut a, 0.5); let base = rms(&run(&mut a, 0.1));
    let mut b = Synth::new(); b.set_params(&loud); run(&mut b, 0.5); b.push(Event::Crash); let with = rms(&run(&mut b, 0.1));
    assert!(with > 1.4 * base, "crash {with} vs engine {base}");
    let mut e = Synth::new(); e.set_params(&car(0.5)); run(&mut e, 0.6); let buf = run(&mut e, 1.0);
    let f0 = engine_freq(0.5);
    assert!(power(&buf, 3.0 * f0) > 0.5 * power(&buf, f0), "the 3rd harmonic must carry weight for small speakers: {} vs {}", power(&buf, 3.0 * f0), power(&buf, f0));
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
