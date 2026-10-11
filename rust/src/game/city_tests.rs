use super::*;
use crate::game::math::{v3, Q};
fn site() -> CitySite { CitySite { centre: v3(1000.0, 20.0, -700.0), half: HALF } }
fn rect_overlap(a: &Building, b: &Building) -> bool { a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z }

#[test] fn building_is_deterministic_per_seed_and_differs_between_seeds() {
    let (a, b, c) = (City::build(site(), 5), City::build(site(), 5), City::build(site(), 6));
    assert_eq!(a.buildings.len(), b.buildings.len());
    assert!(a.buildings.iter().zip(&b.buildings).all(|(x, y)| x.min == y.min && x.max == y.max && x.color == y.color));
    assert!(a.buildings.len() != c.buildings.len() || a.buildings.iter().zip(&c.buildings).any(|(x, y)| x.max != y.max));
}
#[test] fn buildings_are_inside_the_city_never_overlap_and_stay_off_the_streets() {
    for seed in 0..20u64 {
        let c = City::build(site(), seed);
        assert!(c.buildings.len() > 60, "{}", c.buildings.len());
        for (i, b) in c.buildings.iter().enumerate() {
            assert!(b.min.x >= 1000.0 - HALF && b.max.x <= 1000.0 + HALF && b.min.z >= -700.0 - HALF && b.max.z <= -700.0 + HALF);
            assert!((b.min.y - 20.0).abs() < 1e-4 && b.max.y - b.min.y >= 4.0 - 1e-4 && b.max.y - b.min.y <= 90.0 + 1e-4);
            let (lx, lz) = (b.min.x - (1000.0 - HALF), b.min.z - (-700.0 - HALF));
            let (hx, hz) = (b.max.x - (1000.0 - HALF), b.max.z - (-700.0 - HALF));
            assert_eq!(((lx / PITCH).floor(), (lz / PITCH).floor()), ((hx / PITCH).floor(), (hz / PITCH).floor()), "building spans a street");
            let off_x = lx.rem_euclid(PITCH); let off_z = lz.rem_euclid(PITCH);
            assert!(off_x >= (PITCH - BLOCK) / 2.0 - 1e-3 && off_z >= (PITCH - BLOCK) / 2.0 - 1e-3);
            for o in &c.buildings[i + 1..] { assert!(!rect_overlap(b, o), "overlap, seed {seed}"); }
        }
    }
}
#[test] fn heights_fall_with_distance_from_the_centre_and_parks_exist() {
    let c = City::build(site(), 3);
    let mean = |lo: f32, hi: f32| { let v: Vec<f32> = c.buildings.iter().filter(|b| { let d = (b.centre().x - 1000.0).hypot(b.centre().z + 700.0); d >= lo && d < hi }).map(|b| b.max.y - b.min.y).collect(); v.iter().sum::<f32>() / v.len().max(1) as f32 };
    assert!(mean(0.0, 90.0) > mean(180.0, 400.0) * 1.5, "{} vs {}", mean(0.0, 90.0), mean(180.0, 400.0));
    assert!(c.tiles.iter().any(|t| t.kind == TileKind::Park));
    assert!(c.tiles.iter().filter(|t| t.kind == TileKind::Asphalt).count() >= 22);
    assert!(c.roofs.iter().all(|r| r.y - 20.0 >= 20.0 - 1e-3));
    assert!(c.roof_at(1000.0, -700.0).is_none());
}
#[test] fn queries_agree_with_brute_force() {
    let c = City::build(site(), 11);
    for k in 0..400u32 {
        let (x, z) = (1000.0 - 300.0 + (k * 37 % 600) as f32, -700.0 - 300.0 + (k * 91 % 600) as f32);
        let brute = c.buildings.iter().find(|b| x >= b.min.x && x <= b.max.x && z >= b.min.z && z <= b.max.z).map(|b| b.max.y);
        assert_eq!(c.roof_at(x, z), brute, "{x} {z}");
        assert_eq!(c.inside(x, z), (x - 1000.0).abs() <= HALF && (z + 700.0).abs() <= HALF);
        let near = c.buildings_near(v3(x, 25.0, z), 30.0);
        let want = c.buildings.iter().filter(|b| b.min.x - 30.0 <= x && x <= b.max.x + 30.0 && b.min.z - 30.0 <= z && z <= b.max.z + 30.0).count();
        assert!(near.len() >= want, "near must include every building within the radius ({} < {want})", near.len());
    }
}
#[test] fn street_points_are_intersections_inside_the_city() {
    let c = City::build(site(), 2);
    for key in 0..50u64 {
        let p = c.street_point(key);
        assert!(c.inside(p.x, p.z) && (p.y - 20.0).abs() < 1e-4);
        assert!(c.roof_at(p.x, p.z).is_none());
        let u = (p.x - (1000.0 - HALF)) / PITCH;
        assert!((u - u.round()).abs() < 1e-3, "x on a street centre line");
    }
}
fn wall() -> City { City::from_buildings(site(), vec![Building { min: v3(990.0, 20.0, -690.0), max: v3(1010.0, 60.0, -670.0), color: 0 }]) }
#[test] fn push_out_gives_the_depth_and_the_surface_normal() {
    let c = wall();
    let (push, n) = c.push_out(v3(988.0, 30.0, -680.0), 5.0).expect("sphere overlaps the -x face by 3 m");
    assert!(n.x < -0.9 && push.x < 0.0 && push.len() > 2.9 && push.len() < 3.1, "{push:?} {n:?}");
    assert!(c.push_out(v3(1000.0, 70.0, -680.0), 5.0).is_none(), "above the roof by more than the radius");
    let (_, top) = c.push_out(v3(1000.0, 62.0, -680.0), 5.0).unwrap();
    assert!(top.y > 0.9, "resting on the roof: {top:?}");
    assert!(c.push_out(v3(1000.0, 40.0, -680.0), 1.0).is_some(), "centre inside the box is still resolved");
}
#[test] fn touches_side_ignores_the_roof_but_catches_walls_and_undersides() {
    let c = wall();
    assert!(c.touches_side(v3(987.0, 40.0, -680.0), 5.0));
    assert!(!c.touches_side(v3(1000.0, 63.0, -680.0), 5.0), "hovering just above the roof");
    assert!(c.touches_side(v3(1000.0, 17.0, -680.0), 5.0), "underside / base");
    assert!(!c.touches_side(v3(900.0, 40.0, -680.0), 5.0));
}
#[test] fn the_car_hull_stops_at_a_wall_even_at_speed_and_slides_along_it() {
    let c = wall();
    let mut pos = v3(960.0, 21.0, -680.0); let mut vel = v3(30.0, 0.0, 0.0);
    for _ in 0..400 { pos = pos + vel * (1.0 / 60.0); c.resolve_hull(&mut pos, &mut vel, Q::axis_angle(v3(0., 1., 0.), std::f32::consts::FRAC_PI_2)); }
    assert!(pos.x <= 990.0 - 0.9, "the hull must stay outside the wall: x {}", pos.x);
    assert!(vel.x < 3.0, "velocity into the wall is removed: {}", vel.x);
    let mut pos = v3(985.0, 21.0, -700.0); let mut vel = v3(0.0, 0.0, 10.0);
    for _ in 0..60 { pos = pos + vel * (1.0 / 60.0); c.resolve_hull(&mut pos, &mut vel, Q::IDENT); }
    assert!(vel.z > 8.0 && pos.z > -692.0, "sliding keeps most of the speed: {vel:?} {pos:?}");
}
#[test] fn a_car_teleported_inside_a_building_is_lifted_out() {
    let c = wall();
    let mut pos = v3(1000.0, 21.0, -680.0); let mut vel = v3(0.0, 0.0, 0.0);
    for _ in 0..10 { c.resolve_hull(&mut pos, &mut vel, Q::IDENT); }
    let inside = pos.x > 990.0 - 1.0 && pos.x < 1010.0 + 1.0 && pos.z > -690.0 - 1.0 && pos.z < -670.0 + 1.0 && pos.y < 60.0;
    assert!(!inside, "still inside the box at {pos:?}");
}
