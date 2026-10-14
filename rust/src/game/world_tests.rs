use super::*;
use crate::game::math::Q;
#[test] fn world_spawns_upright_on_flat_ground_and_runs() {
    let mut w = World::new(42);
    assert!(w.vehicle.up().y > 0.99);
    for _ in 0..120 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.is_finite() && w.vehicle.speed() > 3.0);
}
#[test] fn same_seed_same_world() {
    let (mut a, mut b) = (World::new(5), World::new(5));
    for _ in 0..300 { let i = Input { throttle: 1.0, steer: 0.3, ..Default::default() }; a.step(&i, 1.0 / 60.0); b.step(&i, 1.0 / 60.0); }
    assert_eq!(a.vehicle.pos, b.vehicle.pos);
}
#[test] fn world_wall_keeps_vehicle_inside() {
    let mut w = World::new(1);
    w.vehicle.pos = v3(WORLD_LIMIT - 5.0, 40.0, 0.0);
    w.vehicle.vel = v3(30.0, 0.0, 0.0);
    for _ in 0..600 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.x.abs() <= WORLD_LIMIT + 1.0 && w.vehicle.pos.z.abs() <= WORLD_LIMIT + 1.0);
}
#[test] fn reset_input_uprights_a_flipped_car() {
    let mut w = World::new(3);
    w.vehicle.rot = w.vehicle.rot.mul(Q::axis_angle(w.vehicle.forward(), 3.0));
    w.step(&Input { reset: true, ..Default::default() }, 1.0 / 60.0);
    assert!(w.vehicle.up().y > 0.9);
}
#[test] fn props_load_around_a_tile_and_are_dropped_when_far() {
    let mut w = World::new(9);
    w.ensure_props(10, 10, 1);
    assert!(w.props.len() == 9);
    w.ensure_props(-10, -10, 1);
    assert!(w.props.len() == 9);
}
#[test] fn sun_dir_is_unit_and_above_horizon() {
    let w = World::new(1); let s = w.sun_dir();
    assert!((s.len() - 1.0).abs() < 1e-4 && s.y > 0.3);
}
#[test] fn world_view_lists_props_and_vehicle() {
    let mut w = World::new(17);
    w.ensure_props(0, 0, 3);
    let v = w.view(3);
    assert!(!v.props.is_empty());
    assert!(v.sun.len() > 0.99);
}
use crate::game::city::{Building, City, CitySite};
fn walled_world() -> World {
    let mut w = World::new(1);
    let g = w.terrain.height(0., 40.);
    w.cities = vec![City::from_buildings(CitySite { centre: v3(0., g, 40.), half: 264.0 }, vec![Building { min: v3(-20., g, 30.), max: v3(20., g + 30.0, 50.), color: 0 }])];
    w
}
#[test] fn worlds_have_cities_matching_the_terrain_sites() {
    let w = World::new(7);
    assert_eq!(w.cities.len(), w.terrain.sites().len());
    assert!(w.cities.iter().all(|c| c.buildings.len() > 60));
    assert!(World::new(7).cities[0].buildings.len() == w.cities[0].buildings.len());
}
#[test] fn the_car_cannot_drive_through_a_building() {
    let mut w = walled_world();
    for _ in 0..(8.0 / (1.0 / 60.0)) as usize { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.vehicle.pos.z <= 30.0 - 0.9, "the car went through the wall: z {}", w.vehicle.pos.z);
    assert!(w.vehicle.pos.is_finite());
}
#[test] fn place_near_city_puts_the_player_outside_the_first_city_facing_it() {
    let mut w = World::new(7);
    assert!(w.place_near_city());
    let s = w.cities[0].site;
    let (dx, dz) = (s.centre.x - w.vehicle.pos.x, s.centre.z - w.vehicle.pos.z);
    let f = w.vehicle.forward();
    assert!((f.x * dx + f.z * dz) / dx.hypot(dz) > 0.99, "facing the centre");
    assert!(!w.cities[0].inside(w.vehicle.pos.x, w.vehicle.pos.z));
    assert!(dx.hypot(dz) < s.half * 1.5 + 100.0);
    let mut flat = World::new(1); flat.terrain = crate::game::terrain::Terrain::flat(); flat.cities.clear();
    assert!(!flat.place_near_city());
}
#[test] fn view_carries_the_cities() { let w = World::new(7); assert_eq!(w.view(1).cities.len(), w.cities.len()); }
#[test] fn glancing_wall_hits_at_speed_do_not_roll_the_car() {
    for deg in [20.0f32, 28.0, 35.0] {
        let mut w = World::new(1);
        let g = w.terrain.height(0., 40.);
        w.cities = vec![City::from_buildings(CitySite { centre: v3(0., g, 20.), half: 264.0 }, vec![Building { min: v3(-200., g, 10.), max: v3(200., g + 30.0, 30.), color: 0 }])];
        let yaw = (deg).to_radians();                          // yaw 0 = +z (towards the wall); 20-35 deg off perpendicular
        // the whole run stays within 25 m of the origin, where no props are placed
        w.vehicle = crate::game::vehicle::Vehicle::new(crate::game::vehicle::VehicleSpec::offroader(), v3(0.0, g + 1.1, -12.0), -yaw);
        w.vehicle.vel = w.vehicle.forward() * 30.0;
        for _ in 0..240 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
        assert!(w.vehicle.up().y > 0.5, "the car rolled over at {deg} deg: up.y {}", w.vehicle.up().y);
        assert!(w.vehicle.pos.z <= 10.0 - 0.9 || w.vehicle.pos.z > 30.0 + 0.9 && false, "z {}", w.vehicle.pos.z);
    }
}
#[test] fn place_in_city_puts_the_car_on_a_street_at_the_centre_() {
    let mut w = World::new(7);
    assert!(w.place_in_city());
    let c = &w.cities[0];
    let p = w.vehicle.pos;
    assert!(c.inside(p.x, p.z) && (p.x - c.site.centre.x).abs() < 40.0 && (p.z - c.site.centre.z).abs() < 40.0, "{p:?}");
    assert!(c.roof_at(p.x, p.z).is_none(), "on a street, not inside a building");
    assert!(c.buildings_near(p, 1.5).is_empty() || !c.touches_side(p + v3(0., 0.5, 0.), 1.5));
    let mut none = World::new(1); none.cities.clear();
    assert!(!none.place_in_city());
}
#[test] fn driving_leaves_tracks_in_the_world() {
    let mut w = World::new(1);
    for _ in 0..180 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); }
    assert!(w.tracks.len() > 0);
}
#[test] fn the_view_carries_the_tracks() { let mut w = World::new(1); for _ in 0..120 { w.step(&Input { throttle: 1.0, ..Default::default() }, 1.0 / 60.0); } assert_eq!(w.view(1).tracks.len(), w.tracks.len()); }
#[test] fn place_near_water_puts_the_car_on_dry_ground_facing_the_lake() {
    let mut w = World::new(5);
    assert!(w.place_near_water());
    let p = w.vehicle.pos;
    assert!(!w.terrain.is_water(p.x, p.z, 0.0));
    let f = w.vehicle.forward(); let (wx, wz) = (p.x + f.x * 22.0, p.z + f.z * 22.0);
    assert!(w.terrain.is_water(wx, wz, 0.5), "no lake 22 m ahead");
}

mod garage_world {
    use super::*;
    use crate::game::vehicle::VehicleSpec;
    fn peugeot() -> VehicleSpec { VehicleSpec::by_id("vulpes").unwrap() }
    #[test] fn worlds_use_the_given_car_and_the_placement_helpers_keep_it() {
        let mut w = World::new_with(7, peugeot());
        assert_eq!((w.vehicle.spec.mass, w.car_spec.mass), (1230.0, 1230.0));
        w.place_near_city(); assert_eq!(w.vehicle.spec.mass, 1230.0);
        w.place_in_city(); assert_eq!(w.vehicle.spec.mass, 1230.0);
        w.place_near_water(); assert_eq!(w.vehicle.spec.mass, 1230.0);
        assert_eq!(World::new(7).vehicle.spec.mass, 1500.0, "the plain constructor keeps the off-roader");
    }
    #[test] fn the_garage_world_has_a_still_car_and_an_orbiting_camera() {
        let mut w = World::garage(peugeot());
        let p0 = w.vehicle.pos; let cam0 = w.cam.pos;
        let rel = cam0 - p0;
        assert!((rel.x.hypot(rel.z) - 6.5).abs() < 0.05 && (rel.y - 1.6).abs() < 0.2, "{rel:?}");
        for _ in 0..600 { w.step_garage(1.0 / 60.0); }
        assert_eq!((w.vehicle.pos, w.vehicle.vel.len()), (p0, 0.0), "the car never moves");
        let c10 = w.cam.pos;
        assert!(((c10 - cam0).len() - 13.0).abs() < 0.6, "half a turn after 10 s: {}", (c10 - cam0).len());
        for _ in 0..600 { w.step_garage(1.0 / 60.0); }
        assert!((w.cam.pos - cam0).len() < 0.6, "a full turn after 20 s");
        let t = w.cam.target - p0; assert!((t.y - 0.6).abs() < 0.05 && ((t.x.hypot(t.z)) - 1.8).abs() < 0.05, "looks 1.8 m beside the car so it sits left of centre: {t:?}");
        let to_car = p0 - w.cam.pos; let side = (t.x * to_car.z - t.z * to_car.x) / to_car.x.hypot(to_car.z); assert!(side.abs() > 1.7, "the offset is across the view direction: {side}");
        for k in 0..8 { let r = w.cam.pos - p0; assert!((r.x.hypot(r.z) - 6.5).abs() < 0.05, "radius at {k}"); w.step_garage(0.7); }
    }
}

#[test] fn the_view_carries_the_car_lights_which_follow_the_night() {
    let w = World::new(1);
    let v = w.view(1);
    assert!(!v.lights.head, "levels start by day");
    assert!((v.lamps.head[0] - v.lamps.head[1]).len() > 0.3);
    assert!(v.lamps.head[0].y > v.vehicle.pos.y - 1.0);
}

use crate::game::vehicle::VehicleSpec as RoadCar;
#[test] fn a_world_with_roads_spawns_on_the_liaison_facing_along_it() {
    let w = World::with_roads(4, RoadCar::offroader());
    let r = &w.terrain.roads().expect("roads").roads[0];
    let t0 = (r.pts[1].pos - r.pts[0].pos).norm();
    let f = w.vehicle.forward();
    assert!(f.x * t0.x + f.z * t0.z > 0.99, "{f:?} vs {t0:?}");
    assert!(w.terrain.near_road(w.vehicle.pos.x, w.vehicle.pos.z, 0.0), "the car spawns on the road");
    assert!(World::new(4).terrain.roads().is_none(), "the plain constructor stays roadless");
}
#[test] fn place_on_road_puts_the_car_on_each_road_facing_along_it() {
    let mut w = World::with_roads(4, RoadCar::offroader());
    for mode in 1..=4 {
        assert!(w.place_on_road(mode), "mode {mode}");
        assert!(w.terrain.near_road(w.vehicle.pos.x, w.vehicle.pos.z, 0.0), "mode {mode}");
    }
    assert!(!World::new(4).place_on_road(1));
}
