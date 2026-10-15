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
