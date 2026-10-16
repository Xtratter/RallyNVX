use super::*;

fn burst(moles: f32) -> Fire { let mut f = Fire::new(); f.emit(v3(0.0, 0.3, -2.0), v3(0.0, 0.0, -1.0), moles, V3::ZERO); f }

#[test] fn the_colour_of_a_black_body_goes_from_red_through_orange_to_white() {
    let (red, orange, yellow, white) = (blackbody(1000.0), blackbody(1700.0), blackbody(2600.0), blackbody(6500.0));
    assert!(red[0] > 0.95 && red[1] < 0.35 && red[2] < 0.05, "{red:?}");
    assert!(orange[0] > 0.95 && orange[1] > 0.4 && orange[1] < 0.65 && orange[2] < 0.2, "{orange:?}");
    assert!(yellow[1] > orange[1] + 0.12 && yellow[2] > orange[2] + 0.1, "{yellow:?}");
    assert!(white.iter().all(|c| *c > 0.95), "{white:?}");
    assert!(blackbody(0.0).iter().all(|c| c.is_finite() && *c >= 0.0 && *c <= 1.0));
}
#[test] fn a_bigger_burn_is_a_bigger_and_faster_fireball_and_nothing_burning_is_no_fire() {
    let (pop, big) = (burst(0.002), burst(0.02));
    assert!(big.parts.len() > pop.parts.len(), "{} vs {}", big.parts.len(), pop.parts.len());
    let size = |f: &Fire| f.parts.iter().map(|p| p.size).sum::<f32>() / f.parts.len() as f32;
    let speed = |f: &Fire| f.parts.iter().map(|p| p.vel.len()).sum::<f32>() / f.parts.len() as f32;
    assert!(size(&big) > size(&pop) && speed(&big) > speed(&pop), "size {} {} speed {} {}", size(&big), size(&pop), speed(&big), speed(&pop));
    assert!(speed(&pop) > 15.0 && speed(&big) < 80.0);
    let mut none = Fire::new(); none.emit(V3::ZERO, v3(0.0, 0.0, -1.0), 0.0, V3::ZERO); none.emit(V3::ZERO, v3(0.0, 0.0, -1.0), f32::NAN, V3::ZERO); none.emit(V3::ZERO, v3(0.0, 0.0, -1.0), -1.0, V3::ZERO);
    assert!(!none.is_active());
}
#[test] fn the_fire_leaves_the_pipe_spreads_cools_dims_and_goes_out() {
    let mut f = burst(0.02);
    let (s0, t0) = (f.parts.iter().map(|p| p.size).sum::<f32>(), f.parts.iter().map(|p| p.temp).sum::<f32>());
    let x0 = f.parts.iter().map(|p| p.pos.z).sum::<f32>() / f.parts.len() as f32;
    for _ in 0..6 { f.update(1.0 / 60.0, V3::ZERO); }
    let n1 = f.parts.len();
    assert!(n1 > 0 && f.parts.iter().map(|p| p.size).sum::<f32>() / n1 as f32 > s0 / 36.0, "it spreads");
    assert!(f.parts.iter().map(|p| p.temp).sum::<f32>() / (n1 as f32) < t0 / 36.0, "it cools");
    assert!(f.parts.iter().map(|p| p.pos.z).sum::<f32>() / (n1 as f32) < x0 - 1.0, "and travels back from the pipe");
    let bright = |f: &Fire| f.instances().iter().map(|i| i.e[3]).fold(0.0f32, f32::max);
    let b1 = bright(&f);
    for _ in 0..12 { f.update(1.0 / 60.0, V3::ZERO); }
    assert!(bright(&f) < b1 || !f.is_active(), "it dims");
    for _ in 0..120 { f.update(1.0 / 60.0, V3::ZERO); }
    assert!(!f.is_active(), "and goes out within a second or two");
}
#[test] fn a_moving_car_leaves_the_fire_behind_in_the_still_air_and_the_pool_is_capped() {
    // the parcels leave with the car's speed and the still air slows them: after a third of a second the car is far ahead of them
    let mut f = Fire::new(); f.emit(v3(0.0, 0.3, -2.0), v3(0.0, 0.0, -1.0), 0.02, v3(0.0, 0.0, 30.0));
    for _ in 0..20 { f.update(1.0 / 60.0, V3::ZERO); }
    let car_z = 30.0 / 3.0 - 2.0;
    assert!(f.parts.iter().all(|p| p.pos.z < car_z - 2.0 && p.vel.is_finite()), "they trail the car: {:?}", f.parts.iter().map(|p| p.pos.z).fold(f32::MIN, f32::max));
    let mut f = Fire::new(); for _ in 0..40 { f.emit(V3::ZERO, v3(0.0, 0.0, -1.0), 0.05, V3::ZERO); }
    assert!(f.parts.len() <= MAX_PARTS);
    assert!(f.instances().iter().all(|i| i.p.iter().chain(i.c.iter()).chain(i.e.iter()).all(|x| x.is_finite())));
}
