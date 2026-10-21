//! Cost measurements (ignored: run them in release, `cargo test --release --lib bench_ -- --ignored --nocapture`). The host is the phone's own CPU, so the times are the device's.
use crate::game::{input::Input, world::World};
use std::time::Instant;

fn stats(mut v: Vec<f32>) -> (f32, f32, f32) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = v.iter().sum::<f32>() / v.len() as f32;
    (mean, v[v.len() * 95 / 100], *v.last().unwrap())
}

#[test] #[ignore] fn bench_world_step() {
    // a real level with the autopilot driving it: the physics, the driveline, the tyres, the dust, the camera, the sky
    let mut w = World::new(5); w.set_autopilot_style(1);
    let mut t = Vec::new();
    for k in 0..60 * 40 {
        let t0 = Instant::now();
        w.step(&Input::default(), 1.0 / 60.0);
        if k > 120 { t.push(t0.elapsed().as_secs_f32() * 1000.0); }
    }
    let (mean, p95, max) = stats(t);
    eprintln!("BENCH world.step (one 1/60 s step, the autopilot drives): mean {mean:.3} ms, p95 {p95:.3}, max {max:.3}; a frame runs 1 to 2 of them at 30 fps");
}
#[test] #[ignore] fn bench_director() {
    let mut w = World::new(5); w.set_autopilot_style(1);
    for _ in 0..600 { w.step(&Input::default(), 1.0 / 60.0); }
    let mut d = crate::audio::Director::new();
    let (mut tu, mut ts) = (Vec::new(), Vec::new());
    for _ in 0..600 {
        w.step(&Input::default(), 1.0 / 60.0);
        let t0 = Instant::now(); let _ = d.update(&w, 0.5, true); tu.push(t0.elapsed().as_secs_f32() * 1000.0);
        let t1 = Instant::now(); let _ = d.scene(&w); ts.push(t1.elapsed().as_secs_f32() * 1000.0);
    }
    let (a, b, c) = stats(tu); let (e, f, g) = stats(ts);
    eprintln!("BENCH director.update: mean {a:.3} ms, p95 {b:.3}, max {c:.3} | director.scene (the ray tracing): mean {e:.3} ms, p95 {f:.3}, max {g:.3}");
}
#[test] #[ignore] fn bench_audio() {
    use crate::audio::{synth::Synth, AudioParams, Machine};
    let mut s = Synth::new();
    s.set_params(&AudioParams { machine: Machine::Car, drive_rpm: 4500.0, clutch: 1.0, load: 0.7, roll: 0.5, wind: 0.3, active: true, ..Default::default() });
    let mut buf = vec![0i16; 2048];
    for _ in 0..20 { s.render(&mut buf); }
    let t0 = Instant::now(); let blocks = 430;   // 10 s of 1024-frame blocks at 44.1 kHz
    for _ in 0..blocks { s.render(&mut buf); }
    let secs = t0.elapsed().as_secs_f32(); let audio = blocks as f32 * 1024.0 / 44100.0;
    eprintln!("BENCH audio: {:.2} s of CPU for {:.2} s of sound ({:.1} % of one core), engine substeps {}", secs, audio, 100.0 * secs / audio, s.load().1);
}
