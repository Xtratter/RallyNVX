use super::*;
use crate::game::dust::phys;

fn gp(coarse: bool) -> GP { GP { pos: v3(0.0, 5.0, 0.0), vel: V3::ZERO, age: 0.0, life: 10.0, size0: 0.5, color: [0.7, 0.6, 0.5], coarse, seed: 0.3 } }
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
