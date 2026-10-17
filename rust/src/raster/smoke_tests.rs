use super::*;

const COLD: Air = Air { celsius: 3.0, humidity: 0.9 };
const HOT: Air = Air { celsius: 30.0, humidity: 0.3 };
fn plume(moles: f32, fuel: f32, air: Air) -> Smoke { let mut s = Smoke::new(); s.emit(v3(0.0, 0.3, -2.0), v3(0.0, 0.0, -1.0), moles, 800.0, fuel, 1.0 / 60.0, V3::ZERO, air); s }

#[test] fn the_vapour_shows_in_cold_humid_air_and_hardly_in_hot_dry_air() {
    assert!(COLD.vapour() > 0.5 && HOT.vapour() < 0.1, "{} {}", COLD.vapour(), HOT.vapour());
    assert!(Air { celsius: 10.0, humidity: 0.95 }.vapour() > Air { celsius: 10.0, humidity: 0.2 }.vapour(), "rain and fog make more");
    let night = Air::from_sky(-0.3, 0.0, 0.0, 0.0); let noon = Air::from_sky(0.9, 0.0, 0.0, 0.0); let rain = Air::from_sky(0.9, 1.0, 1.0, 0.0);
    assert!(night.celsius < noon.celsius - 10.0 && rain.humidity > noon.humidity + 0.4 && night.vapour() > noon.vapour());
    assert!((0.0..=1.0).contains(&Air { celsius: -80.0, humidity: 5.0 }.vapour()));
}
#[test] fn a_hot_dry_day_costs_nothing_and_a_cold_one_makes_a_plume_that_grows_with_the_flow() {
    assert!(plume(0.01, 0.0, HOT).parts.is_empty(), "invisible gas is not emitted");
    let (idle, full) = (plume(0.004, 0.0, COLD), plume(0.05, 0.0, COLD));
    assert!(!idle.parts.is_empty() && full.parts.len() > idle.parts.len(), "{} vs {}", full.parts.len(), idle.parts.len());
    let speed = |s: &Smoke| s.parts.iter().map(|p| p.vel.len()).sum::<f32>() / s.parts.len() as f32;
    assert!(speed(&full) > speed(&idle) * 1.5, "more flow, a faster jet: {} {}", speed(&full), speed(&idle));
    let nothing = { let mut s = Smoke::new(); s.emit(V3::ZERO, v3(0.0, 0.0, -1.0), 0.0, 800.0, 0.0, 0.016, V3::ZERO, COLD); s.emit(V3::ZERO, v3(0.0, 0.0, -1.0), f32::NAN, 800.0, 0.0, 0.016, V3::ZERO, COLD); s };
    assert!(nothing.parts.is_empty());
}
#[test] fn unburnt_fuel_makes_dark_smoke_even_on_a_hot_day() {
    let soot = plume(0.02, 0.004, HOT);
    assert!(!soot.parts.is_empty(), "a rich exhaust is visible in any air");
    let i = soot.instances(v3(0.0, 1.5, 6.0));
    assert!(i.iter().all(|d| d.c[0] < 0.5), "dark: {:?}", i.first().map(|d| d.c));
    let white = plume(0.02, 0.0, COLD).instances(v3(0.0, 1.5, 6.0));
    assert!(white.iter().all(|d| d.c[0] > 0.85), "white vapour");
}
#[test] fn the_plume_slows_rises_spreads_thins_and_dies() {
    let mut s = plume(0.05, 0.0, COLD);
    let (a0, z0) = (s.instances(V3::ZERO).iter().map(|d| d.c[3]).sum::<f32>() / s.parts.len() as f32, s.parts.iter().map(|p| p.pos.z).sum::<f32>() / s.parts.len() as f32);
    let size0 = s.parts[0].size;
    for _ in 0..60 { s.update(1.0 / 60.0, V3::ZERO, COLD); }
    assert!(s.is_active() && s.parts.iter().all(|p| p.size > size0 * 3.0), "it spreads");
    let n = s.parts.len() as f32;
    let a1 = s.instances(V3::ZERO).iter().map(|d| d.c[3]).sum::<f32>() / n.max(1.0);
    assert!(a1 < a0, "and thins: {a1} vs {a0}");
    assert!(s.parts.iter().map(|p| p.pos.z).sum::<f32>() / n < z0 - 0.3, "behind the pipe");
    assert!(s.parts.iter().all(|p| p.vel.len() < 12.0), "slowed by the air");
    for _ in 0..240 { s.update(1.0 / 60.0, V3::ZERO, COLD); }
    assert!(!s.is_active(), "the vapour is gone after a few seconds");
}
#[test] fn the_wind_carries_the_plume_and_the_pool_is_capped_and_sorted() {
    let mut calm = plume(0.03, 0.0, COLD); let mut windy = plume(0.03, 0.0, COLD);
    for _ in 0..90 { calm.update(1.0 / 60.0, V3::ZERO, COLD); windy.update(1.0 / 60.0, v3(6.0, 0.0, 0.0), COLD); }
    let x = |s: &Smoke| s.parts.iter().map(|p| p.pos.x).sum::<f32>() / s.parts.len().max(1) as f32;
    assert!(x(&windy) > x(&calm) + 1.0, "{} vs {}", x(&windy), x(&calm));
    let mut s = Smoke::new(); for _ in 0..80 { s.emit(V3::ZERO, v3(0.0, 0.0, -1.0), 0.1, 900.0, 0.0, 0.016, V3::ZERO, COLD); }
    assert!(s.parts.len() <= MAX_PUFFS);
    let cam = v3(0.0, 1.0, 10.0); let inst = s.instances(cam);
    let d = |i: &DustInst| ((i.p[0] - cam.x).powi(2) + (i.p[1] - cam.y).powi(2) + (i.p[2] - cam.z).powi(2)).sqrt();
    assert!(inst.windows(2).all(|w| d(&w[0]) >= d(&w[1]) - 1e-4), "far to near");
    assert!(inst.iter().all(|i| i.p.iter().chain(i.c.iter()).chain(i.e.iter()).all(|x| x.is_finite())));
}
