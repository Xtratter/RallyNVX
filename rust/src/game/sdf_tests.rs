use super::*;

/// A closed axis-aligned box of half extents `h` centred on the origin as a triangle mesh.
fn cube(h: [f32; 3]) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut v = Vec::new();
    for z in [-1.0f32, 1.0] { for y in [-1.0f32, 1.0] { for x in [-1.0f32, 1.0] { v.push([x * h[0], y * h[1], z * h[2]]); } } }
    // vertex index = x + 2y + 4z (0 = low)
    let quads = [[0, 1, 3, 2], [4, 5, 7, 6], [0, 1, 5, 4], [2, 3, 7, 6], [0, 2, 6, 4], [1, 3, 7, 5]];
    let mut t = Vec::new();
    for q in quads { t.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]); }
    (v, t)
}

#[test] fn a_box_has_the_right_distances_inside_outside_and_near_its_faces() {
    let (v, t) = cube([0.5, 0.4, 1.0]);
    let s = build(&v, &t, 0.1, 0.5);
    assert!(s.at([0.0, 0.0, 0.0]) < -0.25, "inside: {}", s.at([0.0, 0.0, 0.0]));
    assert!((s.at([0.8, 0.0, 0.0]) - 0.3).abs() < 0.08, "0.3 m off the +x face: {}", s.at([0.8, 0.0, 0.0]));
    assert!((s.at([0.0, 0.0, 1.35]) - 0.35).abs() < 0.08, "off the +z face: {}", s.at([0.0, 0.0, 1.35]));
    assert!(s.at([0.0, 0.4 + 0.2, 0.0]) > 0.12 && s.at([0.0, 0.4 + 0.2, 0.0]) < 0.3);
    assert!(s.at([0.0, 0.0, -0.9]) < 0.0 && s.at([0.0, 0.0, -1.2]) > 0.0, "the sign flips through the -z face");
    assert!(s.at([40.0, 0.0, 0.0]) > 0.3, "far away is positive and finite: {}", s.at([40.0, 0.0, 0.0]));
}
#[test] fn the_gradient_points_away_from_the_body() {
    let (v, t) = cube([0.5, 0.5, 0.5]);
    let s = build(&v, &t, 0.08, 0.5);
    let g = s.gradient([0.75, 0.0, 0.0]);
    assert!(g[0] > 0.9 && g[1].abs() < 0.3 && g[2].abs() < 0.3, "{g:?}");
    let up = s.gradient([0.0, 0.8, 0.0]);
    assert!(up[1] > 0.9, "{up:?}");
    let down = s.gradient([0.0, -0.8, 0.0]);
    assert!(down[1] < -0.9, "{down:?}");
    let len = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-4);
}
#[test] fn an_empty_or_broken_mesh_gives_an_all_outside_field() {
    let s = build(&[], &[], 0.1, 0.5);
    assert!(s.at([0.0, 0.0, 0.0]) > 1.0);
    let s2 = build(&[[f32::NAN, 0.0, 0.0]], &[0, 0, 0], 0.1, 0.5);
    assert!(s2.at([0.0, 0.0, 0.0]) > 1.0);
    assert_eq!(s.gradient([0.0; 3]), [0.0; 3]);
}
#[test] fn the_peugeot_asset_gives_a_sane_field_quickly() {
    let (body, _) = crate::raster::rcar::parse(include_bytes!("../../assets/rally_car.rcar")).unwrap();
    let verts: Vec<[f32; 3]> = body.verts.iter().map(|v| v.pos).collect();
    let t0 = std::time::Instant::now();
    let s = build(&verts, &body.idx, 0.1, 0.5);
    let took = t0.elapsed().as_secs_f32();
    assert!(took < 4.0, "built in {took:.2} s");
    assert!(s.n[0] * s.n[1] * s.n[2] < 200_000, "{:?}", s.n);
    assert!(s.at([0.0, 0.8, 4.0]) > 0.5, "well ahead of the car");
    assert!(s.at([2.0, 0.8, 0.0]) > 0.5, "beside it");
    let inside = s.d.iter().filter(|d| **d < 0.0).count() as f32 / s.d.len() as f32;
    assert!(inside > 0.05, "the body fills part of the grid: {inside}");
    // on the roof's surface the distance is about nothing, a little above it it is outside
    assert!(s.at([0.0, 1.9, -0.3]) > 0.15, "above the roof: {}", s.at([0.0, 1.9, -0.3]));
}
