use super::*;
use crate::game::dust::phys;

fn gp(coarse: bool) -> GP { GP { pos: v3(0.0, 5.0, 0.0), vel: V3::ZERO, age: 0.0, life: 10.0, size0: 0.5, color: [0.7, 0.6, 0.5], coarse, seed: 0.3, speck: false } }
fn flat(_: f32, _: f32) -> f32 { 0.0 }

#[test]
fn fine_dust_in_still_air_settles_slowly_coarse_faster() {
    let ph = phys();
    let (mut f, mut c) = (gp(false), gp(true));
    for _ in 0..60 { step(&mut f, V3::ZERO, flat, &ph, 1.0 / 60.0); step(&mut c, V3::ZERO, flat, &ph, 1.0 / 60.0); }
    assert!(f.pos.y < 5.0 && c.pos.y < f.pos.y, "fine {} coarse {}", f.pos.y, c.pos.y);
    assert!(f.pos.x == 0.0 && f.pos.z == 0.0);
}
#[test]
fn it_relaxes_to_the_air_with_the_time_constant_of_its_class() {
    let ph = phys();
    let mut p = gp(false);
    p.pos.y = 100.0;
    let air = v3(10.0, 0.0, 0.0);
    let mut t = 0.0;
    while t < ph.tau_fine { step(&mut p, air, flat, &ph, 1.0 / 240.0); t += 1.0 / 240.0; }
    assert!((p.vel.x - 10.0 * (1.0 - (-1.0f32).exp())).abs() < 0.2, "{}", p.vel.x);   // 63 % of the air's speed after one tau
}
#[test]
fn grit_is_deposited_fine_dust_slides() {
    let ph = phys();
    let mut c = gp(true); c.pos.y = 0.05; c.vel = v3(3.0, -2.0, 0.0);
    assert!(!step(&mut c, V3::ZERO, flat, &ph, 0.1));
    let mut f = gp(false); f.pos.y = 0.05; f.vel = v3(3.0, -2.0, 0.0);
    assert!(step(&mut f, V3::ZERO, flat, &ph, 0.1));
    assert!(f.vel.y == 0.0 && f.vel.x < 3.0 && f.pos.y >= 0.04, "{:?}", f);
}
#[test]
fn old_particles_retire() {
    let mut p = gp(false); p.age = 9.99;
    assert!(!step(&mut p, V3::ZERO, flat, &phys(), 0.1));
}
#[test]
fn a_bad_air_velocity_is_still_air() {
    let mut p = gp(false);
    assert!(step(&mut p, v3(f32::NAN, 1.0, 1.0), flat, &phys(), 0.016));
    assert!(p.pos.is_finite() && p.vel.is_finite());
}
#[test]
fn render_fades_in_and_out() {
    let mut p = gp(false);
    assert!(render(&p).alpha == 0.0);   // just born: invisible
    p.age = 1.0; let mid = render(&p).alpha;
    p.age = 9.9; let end = render(&p).alpha;
    assert!(mid > 0.3 && end < 0.01, "{mid} {end}");
    assert!(render(&p).size > p.size0);
}

const SIZE: [f32; 3] = [12.0, 6.0, 24.0];

#[test]
fn box_coordinates_follow_the_car() {
    for yaw in [0.0f32, 0.7, 2.0, -1.3, 3.0] {
        let car = v3(10.0, 3.0, -4.0);
        let c = to_box(car, car, yaw, 2.0, SIZE);
        assert!((c[0] - 6.0).abs() < 1e-4 && (c[2] - 12.0).abs() < 1e-4 && (c[1] - 1.0).abs() < 1e-4, "{c:?}");   // the car's centre is the box's middle
        let ahead = car + v3(yaw.sin(), 0.0, yaw.cos()) * 5.0;
        assert!((to_box(ahead, car, yaw, 2.0, SIZE)[2] - 17.0).abs() < 1e-4, "5 m ahead is 5 m further along z");
        // a box vector along z is the car's forward in the world
        let d = from_box_dir(v3(0.0, 0.0, 1.0), yaw);
        assert!((d.x - yaw.sin()).abs() < 1e-5 && (d.z - yaw.cos()).abs() < 1e-5);
    }
}
#[test]
fn the_box_weight_is_one_inside_zero_outside_and_continuous() {
    assert!((box_weight([6.0, 3.0, 12.0], SIZE) - 1.0).abs() < 1e-6);
    assert_eq!(box_weight([-0.5, 3.0, 12.0], SIZE), 0.0);
    assert_eq!(box_weight([6.0, -0.2, 12.0], SIZE), 0.0);   // below the road
    assert_eq!(box_weight([6.0, 3.0, 30.0], SIZE), 0.0);
    let mut last = box_weight([0.0, 3.0, 12.0], SIZE);
    for i in 1..=200 { let w = box_weight([i as f32 * 0.02, 3.0, 12.0], SIZE); assert!((w - last).abs() < 0.03, "jump at {i}: {last} -> {w}"); last = w; }
    assert!(box_weight([0.75, 3.0, 12.0], SIZE) > 0.3 && box_weight([0.75, 3.0, 12.0], SIZE) < 0.7);
}
#[test]
fn air_in_still_air_is_the_wind_for_a_yawed_moving_car() {
    // the grid holds the air relative to the car: against its motion, plus the wind
    let (yaw, wind) = (0.9f32, v3(2.0, 0.0, -1.0));
    let car_vel = v3(yaw.sin(), 0.0, yaw.cos()) * 20.0;
    let rel = car_vel - wind;
    let (f, r) = (v3(yaw.sin(), 0.0, yaw.cos()), v3(-yaw.cos(), 0.0, yaw.sin()));
    let grid = v3(-rel.dot(r), 0.0, -rel.dot(f));
    let car = v3(0.0, 1.0, 0.0);
    let a = air_in_box(car, car, car_vel, yaw, 0.0, grid, v3(9.0, 9.0, 9.0), SIZE);
    assert!((a - wind).len() < 1e-3, "{a:?}");
    let far = air_in_box(v3(100.0, 1.0, 0.0), car, car_vel, yaw, 0.0, grid, v3(9.0, 9.0, 9.0), SIZE);
    assert!((far - v3(9.0, 9.0, 9.0)).len() < 1e-4, "{far:?}");
}
#[test]
fn turbulence_is_finite_and_grows_with_speed() {
    let ph = phys();
    let (a, b) = (turbulence(v3(1.0, 2.0, 3.0), 4.0, 0.0, &ph), turbulence(v3(1.0, 2.0, 3.0), 4.0, 30.0, &ph));
    assert!(a.is_finite() && b.is_finite() && b.len() > a.len());
}

#[test]
fn the_ground_window_is_toroidal_and_interpolates() {
    let base = window_base([100, -40]);
    assert_eq!(base, [68, -72]);
    assert!(in_window([68, -72], base) && in_window([131, -9], base) && !in_window([132, -9], base) && !in_window([100, -73], base));
    assert_eq!(slot([0, 0]), slot([64, 64]));
    assert_ne!(slot([1, 0]), slot([0, 1]));
    // a slope of 0.5 m per metre in x and 0.2 in z, sampled at the cell centres
    let mut heights = vec![0.0f32; (WINDOW * WINDOW) as usize];
    for cx in base[0]..base[0] + WINDOW { for cz in base[1]..base[1] + WINDOW { heights[slot([cx, cz])] = 0.5 * (cx as f32 + 0.5) + 0.2 * (cz as f32 + 0.5); } }
    let h = |i: usize| heights[i];
    let g = ground_height(100.3, -40.8, base, h, -999.0);
    assert!((g - (0.5 * 100.3 + 0.2 * -40.8)).abs() < 1e-3, "{g}");
    assert_eq!(ground_height(500.0, 0.0, base, h, 7.0), 7.0);   // outside the window: the fallback
}

#[test]
fn the_coupling_config_parses_and_errors_are_clear() {
    let c = coupling();
    assert!(c.push > 0.0 && c.drag_max <= 1.0 && c.mass_coarse > c.mass_fine);
    assert!(Coupling::from_json("{}").unwrap_err().contains("dust_air"));
    assert!(Coupling::from_json(r#"{"dust_air":{"push":-1,"drag":0.3,"drag_max":0.5,"mass_fine":1,"mass_coarse":3}}"#).unwrap_err().contains("push"));
}
#[test]
fn the_field_adds_up_mass_and_momentum_and_ignores_particles_outside() {
    let mut f = Field::new([8, 6, 10], 0.5);
    f.splat([1.2, 1.2, 2.2], v3(2.0, 0.0, -4.0), 1.0);
    f.splat([1.3, 1.3, 2.3], v3(4.0, 0.0, -2.0), 3.0);   // the same cell
    f.splat([99.0, 1.0, 1.0], v3(1.0, 1.0, 1.0), 5.0);   // outside
    f.splat([1.0, -0.1, 1.0], v3(1.0, 1.0, 1.0), 5.0);   // below the road
    let c = [2, 2, 4];
    assert!((f.mass(c) - 4.0).abs() < 1e-2, "{}", f.mass(c));
    let v = f.mean_velocity(c);
    assert!((v.x - 3.5).abs() < 0.02 && (v.z + 2.5).abs() < 0.02, "{v:?}");
    let total: f32 = (0..8).flat_map(|x| (0..6).flat_map(move |y| (0..10).map(move |z| [x, y, z]))).map(|c| f.mass(c)).sum();
    assert!((total - 4.0).abs() < 1e-2, "{total}");
    assert_eq!(f.mean_velocity([0, 0, 0]), V3::ZERO);
}
#[test]
fn dust_pushes_away_from_dense_dust_and_not_in_a_uniform_field() {
    let mut f = Field::new([12, 12, 12], 0.5);
    for _ in 0..20 { f.splat([3.0, 3.0, 3.0], V3::ZERO, 1.0); }   // a blob
    let right = f.push([3.7, 3.25, 3.25], 2.0);   // on the +x side of the blob: pushed towards +x
    let left = f.push([2.3, 3.25, 3.25], 2.0);
    assert!(right.x > 0.0 && left.x < 0.0, "{right:?} {left:?}");
    assert!(f.push([5.5, 5.5, 5.5], 2.0).len() < 1e-6, "far from the blob nothing pushes");
    let mut u = Field::new([8, 8, 8], 0.5);
    for x in 0..8 { for y in 0..8 { for z in 0..8 { u.splat([x as f32 * 0.5 + 0.25, y as f32 * 0.5 + 0.25, z as f32 * 0.5 + 0.25], V3::ZERO, 1.0); } } }
    assert!(u.push([2.0, 2.0, 2.0], 2.0).len() < 1e-4, "a uniform field has no gradient");
}
#[test]
fn the_air_drag_grows_with_the_mass_and_saturates() {
    let c = coupling();
    assert_eq!(air_drag(0.0, &c), 0.0);
    assert!(air_drag(0.5, &c) < air_drag(1.0, &c) || air_drag(1.0, &c) == c.drag_max);
    assert_eq!(air_drag(1000.0, &c), c.drag_max);
}

#[test]
fn the_noise_is_deterministic_bounded_and_smooth() {
    let p = v3(1.3, -4.7, 9.1);
    assert_eq!(vnoise3(p, 0), vnoise3(p, 0));
    assert_ne!(vnoise3(p, 0), vnoise3(p, 1));
    let mut m = 0.0f32; let mut sum = 0.0f32;
    for i in 0..2000 { let q = v3(i as f32 * 0.37, i as f32 * 0.11 - 50.0, (i % 17) as f32 * 0.9); let n = vnoise3(q, 1); assert!(n.abs() <= 1.0001, "{n}"); m = m.max(n.abs()); sum += n; }
    assert!(m > 0.6, "uses the range: {m}");
    assert!((sum / 2000.0).abs() < 0.1, "mean {}", sum / 2000.0);
    // smooth: a step of 1/1000 changes little
    assert!((vnoise3(p + v3(0.001, 0.0, 0.0), 2) - vnoise3(p, 2)).abs() < 0.01);
}
#[test]
fn curl_noise_has_no_sinks_or_sources() {
    // the divergence of a curl is zero; with finite differences it is small against the field's own variation
    let e = 0.05;
    let (mut div_sum, mut mag_sum) = (0.0f32, 0.0f32);
    for i in 0..200 {
        let q = v3(i as f32 * 0.173, (i as f32 * 0.071).sin() * 5.0, i as f32 * 0.093 - 8.0);
        let d = (curl_noise(q + v3(e, 0.0, 0.0)).x - curl_noise(q - v3(e, 0.0, 0.0)).x + curl_noise(q + v3(0.0, e, 0.0)).y - curl_noise(q - v3(0.0, e, 0.0)).y
            + curl_noise(q + v3(0.0, 0.0, e)).z - curl_noise(q - v3(0.0, 0.0, e)).z) / (2.0 * e);
        div_sum += d.abs(); mag_sum += curl_noise(q).len();
    }
    assert!(mag_sum / 200.0 > 0.3, "the field is not empty: {}", mag_sum / 200.0);
    assert!(div_sum / 200.0 < 0.25 * mag_sum / 200.0 + 0.05, "divergence {} against magnitude {}", div_sum / 200.0, mag_sum / 200.0);
}
#[test]
fn turbulence_scales_with_amplitude_and_moves_with_the_drift() {
    let p = v3(3.0, 1.0, -2.0);
    let a = turbulence_curl(p, 1.0, V3::ZERO, 2.0);
    let b = turbulence_curl(p, 1.0, V3::ZERO, 4.0);
    assert!(a.is_finite() && ((b - a * 2.0).len() < 1e-4), "{a:?} {b:?}");
    assert_eq!(turbulence_curl(p, 0.0, V3::ZERO, 0.0), V3::ZERO);
    // the pattern is carried by the drift: the same feature one second later sits `drift` further on (the slow evolution aside, which is small for the first octave)
    let drift = v3(2.0, 0.0, 0.0);
    let (v0, v1) = (turbulence_curl(p, 0.0, drift, 1.0), turbulence_curl(p + drift, 1.0, drift, 1.0));
    assert!((v0 - v1).len() < 0.6 * v0.len().max(0.05) + 0.2, "{v0:?} {v1:?}");
}

#[test]
fn the_analytic_gradient_matches_finite_differences_and_the_value() {
    for i in 0..60 {
        let p = v3(i as f32 * 0.31 - 7.0, (i as f32 * 0.17).cos() * 4.0, i as f32 * 0.23 + 1.0);
        let (val, g) = vnoise3_grad(p, 1);
        assert!((val - vnoise3(p, 1)).abs() < 1e-5, "value {val} vs {}", vnoise3(p, 1));
        let e = 0.001;
        let fd = v3((vnoise3(p + v3(e, 0.0, 0.0), 1) - vnoise3(p - v3(e, 0.0, 0.0), 1)) / (2.0 * e), (vnoise3(p + v3(0.0, e, 0.0), 1) - vnoise3(p - v3(0.0, e, 0.0), 1)) / (2.0 * e), (vnoise3(p + v3(0.0, 0.0, e), 1) - vnoise3(p - v3(0.0, 0.0, e), 1)) / (2.0 * e));
        assert!((g - fd).len() < 0.02 * (1.0 + fd.len()), "{g:?} vs {fd:?}");
    }
}

#[test]
fn the_trailing_vortices_press_the_centre_down_lift_the_outside_and_fade() {
    let (hw, hl, h) = (0.9, 2.0, 1.4);
    // behind the car on the centre line at the height of the vortices: downwash; outboard of the left vortex: upwash
    let mid = trailing_vortices(v3(0.0, 0.9, -hl - 2.0), hw, hl, h, 20.0, 0.3);
    assert!(mid.y < -0.5 && mid.x.abs() < 1e-3 && mid.z == 0.0, "{mid:?}");
    let outside = trailing_vortices(v3(hw * 0.8 + 1.5, 0.9, -hl - 2.0), hw, hl, h, 20.0, 0.3);
    assert!(outside.y > 0.1, "{outside:?}");
    // mirror symmetry
    let l = trailing_vortices(v3(-0.7, 0.9, -hl - 3.0), hw, hl, h, 20.0, 0.3);
    let r = trailing_vortices(v3(0.7, 0.9, -hl - 3.0), hw, hl, h, 20.0, 0.3);
    assert!((l.x + r.x).abs() < 1e-5 && (l.y - r.y).abs() < 1e-5, "{l:?} {r:?}");
    // nothing ahead of the rear, fading with the distance, bounded by the peak, finite on the core
    assert_eq!(trailing_vortices(v3(0.0, 0.9, 0.0), hw, hl, h, 20.0, 0.3), V3::ZERO);
    let near = trailing_vortices(v3(0.3, 0.9, -hl - 1.0), hw, hl, h, 20.0, 0.3).len();
    let far = trailing_vortices(v3(0.3, 0.9, -hl - 20.0), hw, hl, h, 20.0, 0.3).len();
    assert!(near > 3.0 * far, "{near} {far}");
    for i in 0..400 { let p = v3(i as f32 * 0.02 - 4.0, 0.1 + (i % 17) as f32 * 0.15, -hl - 0.5 - (i % 23) as f32); let v = trailing_vortices(p, hw, hl, h, 20.0, 0.3); assert!(v.is_finite() && v.len() <= 2.0 * 0.3 * 20.0 + 1e-3, "{p:?} {v:?}"); }
    let on_core = trailing_vortices(v3(hw * 0.8, 0.55 * h + 0.15, -hl - 3.0), hw, hl, h, 20.0, 0.3);
    assert!(on_core.is_finite());
    assert_eq!(trailing_vortices(v3(0.0, 0.9, -hl - 2.0), hw, hl, h, 0.0, 0.3), V3::ZERO);
}

#[test]
fn the_vortex_street_alternates_is_periodic_bounded_and_lives_by_the_body() {
    let (hw, hl, h, u) = (0.9f32, 2.0f32, 1.4f32, 20.0f32);
    let period = 2.0 * hw / (0.2 * u);
    let p = v3(0.2, 0.7, -hl - 4.0);
    let a = karman_street(p, 1.0, hw, hl, h, u, 0.25);
    let b = karman_street(p, 1.0 + 2.0 * period, hw, hl, h, u, 0.25);   // the street alternates, so it repeats every two shedding periods
    assert!(a.is_finite() && (a - b).len() < 0.05 * a.len().max(1.0), "periodic in two shedding periods: {a:?} {b:?}");
    assert!(a.len() > 0.1, "there is a swirl behind the car: {a:?}");
    // the flow changes sign over half a period of shedding at a fixed point (the street alternates)
    let mut signs = std::collections::HashSet::new();
    for i in 0..40 { let v = karman_street(p, i as f32 * period / 20.0, hw, hl, h, u, 0.25); signs.insert(v.x > 0.0); }
    assert_eq!(signs.len(), 2, "alternates");
    // above the roof it fades out, with no speed none at all, and always bounded
    assert!(karman_street(v3(0.2, 5.0, -hl - 4.0), 1.0, hw, hl, h, u, 0.25).len() < 1e-6);
    assert_eq!(karman_street(p, 1.0, hw, hl, h, 0.0, 0.25), V3::ZERO);
    for i in 0..500 { let q = v3(i as f32 * 0.02 - 5.0, (i % 13) as f32 * 0.15, -hl - 0.1 - (i % 31) as f32); let v = karman_street(q, i as f32 * 0.07, hw, hl, h, u, 0.25); assert!(v.is_finite() && v.len() <= 5.0 * 0.25 * u * 2.0, "{q:?} {v:?}"); }
}

#[test]
fn the_recirculation_draws_the_dust_in_and_up_and_only_just_behind_the_car() {
    let (hw, hl, h) = (0.9f32, 2.0f32, 1.4f32);
    let a = wake_inflow(v3(0.6, 0.4, -hl - 0.8), hw, hl, h, 20.0, 1.0);
    assert!(a.x < 0.0 && a.y > 0.0 && a.z > 0.0, "inwards, up and towards the car: {a:?}");
    let b = wake_inflow(v3(-0.6, 0.4, -hl - 0.8), hw, hl, h, 20.0, 1.0);
    assert!((a.x + b.x).abs() < 1e-5 && (a.z - b.z).abs() < 1e-5, "mirror");
    assert_eq!(wake_inflow(v3(0.0, 0.4, 0.0), hw, hl, h, 20.0, 1.0), V3::ZERO);   // not under the car
    assert_eq!(wake_inflow(v3(0.0, 0.4, -hl - 12.0), hw, hl, h, 20.0, 1.0), V3::ZERO);
    assert!(wake_inflow(v3(0.0, 0.4, -hl - 5.0), hw, hl, h, 20.0, 1.0).len() < 0.2 * wake_inflow(v3(0.0, 0.4, -hl - 0.8), hw, hl, h, 20.0, 1.0).len());
    assert_eq!(wake_inflow(v3(0.6, 0.4, -hl - 0.8), hw, hl, h, 0.0, 1.0), V3::ZERO);
}
