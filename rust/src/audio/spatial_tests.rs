use super::*;
use crate::game::math::v3;

#[test] fn the_travel_time_is_distance_over_the_speed_of_sound() {
    let p = path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 343.0));
    assert!((p.delay - 1.0).abs() < 1e-3, "{}", p.delay);
    assert!(path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 1.0e6)).delay <= MAX_DELAY);
}
#[test] fn the_sound_gets_quieter_and_duller_with_the_distance_and_never_louder_than_the_cap() {
    let at = |d: f32| path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, d));
    assert!((at(8.0).gain - 1.0).abs() < 1e-4, "the chase distance is the unit");
    assert!(at(16.0).gain < at(8.0).gain * 0.55 && at(64.0).gain < at(16.0).gain * 0.3, "the 1/r law");
    assert!(at(0.1).gain <= 4.0 + 1e-4 && at(0.1).gain.is_finite());
    assert!(at(200.0).air_hz < at(10.0).air_hz && at(10.0).air_hz < 18000.0, "the air takes the highs");
}
#[test] fn the_side_decides_the_pan_and_the_ear_that_hears_first() {
    // facing +z: the right is -x (as for the car), the left is +x
    let right = path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(-10.0, 0.0, 0.0));
    let left = path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(10.0, 0.0, 0.0));
    let ahead = path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 10.0));
    assert!(right.pan > 0.99 && left.pan < -0.99 && ahead.pan.abs() < 1e-4, "{} {} {}", right.pan, left.pan, ahead.pan);
    assert!(right.itd > 0.0 && left.itd < 0.0 && ahead.itd.abs() < 1e-9);
    assert!((right.itd - 0.00066).abs() < 1.5e-4, "about 0.66 ms at 90 degrees: {}", right.itd);
    // turning the head turns the picture: the same source seen with the gaze turned to its side is ahead
    let turned = path(V3::ZERO, v3(-1.0, 0.0, 0.0), v3(-10.0, 0.0, 0.0));
    assert!(turned.pan.abs() < 1e-4);
}
#[test] fn an_emitter_delays_the_sound_by_the_travel_time_and_puts_it_in_the_right_ear() {
    let mut e = Emitter::new();
    let p = Path { delay: 0.01, gain: 1.0, pan: 1.0, air_hz: 18000.0, itd: 0.0 };
    e.set_path(&p);
    let mut first = None; let (mut lsum, mut rsum) = (0.0f32, 0.0f32);
    for i in 0..2000 {
        let (l, r) = e.process(if i == 0 { 1.0 } else { 0.0 });
        if first.is_none() && r.abs() > 0.02 { first = Some(i); }
        lsum += l * l; rsum += r * r;
    }
    let first = first.expect("an echo arrives");
    let want = (0.01 * RATE as f32) as i32;
    assert!((first as i32 - want).abs() <= 6, "arrives after {first} samples, expected about {want}");
    assert!(rsum > lsum * 30.0, "a sound on the right is in the right ear: {rsum} vs {lsum}");
}
#[test] fn a_receding_source_is_lower_in_pitch_and_an_approaching_one_higher() {
    // a 1 kHz tone from a source moving at 30 m/s: the delay changes by -/+ 30/343 s per second
    let tone = |speed: f32| {
        let mut e = Emitter::new();
        let mut out = Vec::new();
        let mut dist = 100.0f32;
        for i in 0..44100 {
            if i % 128 == 0 { e.set_path(&path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, dist))); }
            dist += speed / RATE as f32;
            let (l, r) = e.process((std::f32::consts::TAU * 1000.0 * i as f32 / RATE as f32).sin());
            out.push(l + r);
        }
        // count the rising zero crossings in the last half second
        let tail = &out[22050..]; tail.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 * 2.0
    };
    let (still, away, toward) = (tone(0.0), tone(30.0), tone(-30.0));
    assert!(away < still - 40.0 && toward > still + 40.0, "away {away} still {still} toward {toward}");
    assert!((away - 1000.0 / (1.0 + 30.0 / 343.0)).abs() < 40.0 && (toward - 1000.0 / (1.0 - 30.0 / 343.0)).abs() < 40.0);
}
#[test] fn a_jump_of_the_position_is_taken_at_once_and_the_output_stays_finite() {
    let mut e = Emitter::new();
    e.set_path(&path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 10.0)));
    for _ in 0..1000 { e.process(0.3); }
    e.set_path(&path(V3::ZERO, v3(0.0, 0.0, 1.0), v3(0.0, 0.0, 400.0)));
    for _ in 0..5000 { let (l, r) = e.process(0.3); assert!(l.is_finite() && r.is_finite()); }
}
