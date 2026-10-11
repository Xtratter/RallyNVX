use super::*;
use crate::game::{math::v3, terrain::Terrain};
fn real_city() -> Vec<City> { let t = Terrain::new(7); t.sites().iter().map(|s| City::build(*s, 7)).collect() }
fn cam_in(c: &City) -> V3 { v3(c.site.centre.x, c.site.centre.y + 5.0, c.site.centre.z) }
#[test] fn culling_keeps_only_parts_within_the_radius_and_orders_buildings_nearest_first() {
    let cs = real_city(); let cam = cam_in(&cs[0]);
    let near = city_parts(&cs, cam, 150.0, 100_000, 150.0);
    let far = city_parts(&cs, cam, 700.0, 100_000, 150.0);
    assert!(!near.parts.is_empty() && far.parts.len() > near.parts.len());
    let d = |p: &Part| (p.pos.x - cam.x).hypot(p.pos.z - cam.z);
    assert!(near.parts.iter().all(|p| d(p) <= 150.0 + p.scale.x.max(p.scale.z)), "culling radius");
    assert!(far.parts.windows(2).filter(|w| w[0].facade && w[1].facade).all(|w| d(&w[0]) <= d(&w[1]) + 1e-3), "buildings nearest first");
}
#[test] fn the_budget_is_a_hard_cap_and_keeps_the_nearest() {
    let cs = real_city(); let cam = cam_in(&cs[0]);
    let all = city_parts(&cs, cam, 700.0, 100_000, 150.0);
    let cut = city_parts(&cs, cam, 700.0, 50, 150.0);
    assert_eq!(cut.parts.len(), 50.min(all.parts.len()));
    assert!(cut.near <= cut.parts.len());
    for (a, b) in cut.parts.iter().zip(&all.parts) { assert_eq!(a.pos, b.pos); }
}
#[test] fn buildings_carry_the_facade_flag_and_palette_colours_tiles_do_not() {
    let cs = real_city(); let cam = cam_in(&cs[0]);
    let p = city_parts(&cs, cam, 700.0, 100_000, 150.0).parts;
    assert!(p.iter().any(|x| x.facade) && p.iter().any(|x| !x.facade));
    for x in p.iter().filter(|x| x.facade) { assert!(PALETTE.contains(&x.color) && !x.glow && x.mesh == 0 && x.scale.y >= 4.0 - 1e-3); }
    for x in p.iter().filter(|x| !x.facade) { assert!(x.scale.y <= 1.0 && x.mesh == 0); }
}
#[test] fn near_counts_the_leading_shadow_casters_and_a_far_camera_sees_nothing() {
    let cs = real_city(); let cam = cam_in(&cs[0]);
    let r = city_parts(&cs, cam, 700.0, 100_000, 100.0);
    assert!(r.near > 0 && r.near < r.parts.len());
    let d = |p: &Part| (p.pos.x - cam.x).hypot(p.pos.z - cam.z);
    assert!(r.parts[..r.near].iter().all(|p| p.facade && d(p) <= 100.0 + p.scale.x.max(p.scale.z)));
    assert!(city_parts(&cs, v3(0.0, 5.0, 0.0), 300.0, 1000, 100.0).parts.is_empty(), "no city near the spawn point");
}
#[test] fn a_typical_frame_stays_within_the_instance_budget() {
    let cs = real_city(); let cam = cam_in(&cs[0]);
    let p = city_parts(&cs, cam, 700.0, 6000, 150.0);
    assert!(p.parts.len() <= 6000 && p.parts.len() > 100, "{}", p.parts.len());
}
