use super::*;

fn small() -> (Fluid, Window) { let n = [16, 8, 16]; (Fluid::new(n, 1.0), Window { start: [0, 0, 0], n }) }

#[test] fn the_embedded_numbers_parse_and_bad_ones_are_named() {
    let c = cfg();
    assert_eq!(c.n, [96, 16, 96]);
    assert!(c.cell > 0.0 && c.iterations >= 8);
    let bad = super::super::grip::EMBEDDED.replace("\"iterations\": 8", "\"iterations\": 0");
    assert!(FluidCfg::from_json(&bad).unwrap_err().contains("iterations"));
    assert!(FluidCfg::from_json("{}").unwrap_err().contains("fluid"));
}
#[test] fn a_dust_blob_in_a_uniform_flow_moves_with_the_flow_and_keeps_its_mass() {
    let (mut f, w) = small();
    f.vel.iter_mut().for_each(|v| *v = [2.0, 0.0, 0.0]);
    f.emit(&w, [4.5, 4.5, 8.5], 10.0);
    let (m0, c0) = (f.mass(), f.centre_of_mass(&w));
    for _ in 0..10 { f.advect_dust(&w, 0.2, 0.0, 0.0); }   // 2 s at 2 m/s
    let c1 = f.centre_of_mass(&w);
    assert!((c1[0] - c0[0] - 4.0).abs() < 0.3, "moved {} m", c1[0] - c0[0]);
    assert!((f.mass() - m0).abs() < 0.03 * m0, "mass {} -> {}", m0, f.mass());
    assert!((c1[1] - c0[1]).abs() < 0.05 && (c1[2] - c0[2]).abs() < 0.05);
}
#[test] fn the_projection_removes_most_of_the_divergence() {
    let (mut f, w) = small();
    // a smooth, strongly divergent field on the torus (a collocated grid cannot remove a checkerboard, and a real flow has none)
    for z in 0..16i32 { for y in 0..8i32 { for x in 0..16i32 {
        let i = f.idx([x, y, z]);
        let (fx, fy, fz) = (x as f32 / 16.0 * std::f32::consts::TAU, y as f32 / 8.0 * std::f32::consts::TAU, z as f32 / 16.0 * std::f32::consts::TAU);
        f.vel[i] = [3.0 * fx.sin() + 1.0 * fz.cos(), 2.0 * fy.cos(), 3.0 * fz.sin() + fx.cos()];
    } } }
    let before = f.max_divergence();
    f.step_velocity(&w, 0.0, 0.0, 80);
    let after = f.max_divergence();
    assert!(before > 1.0 && after < before * 0.25, "divergence {before} -> {after}");
}
#[test] fn the_flow_goes_round_a_solid_box_and_nothing_flows_through_it() {
    let (mut f, w) = small();
    f.wind = [4.0, 0.0, 0.0];
    f.vel.iter_mut().for_each(|v| *v = [4.0, 0.0, 0.0]);
    f.set_solids(|p| if (6.0..9.0).contains(&p[0]) && (3.0..6.0).contains(&p[2]) && p[1] < 5.0 { Some([0.0; 3]) } else { None }, &w);
    for _ in 0..40 { f.step_velocity(&w, 0.1, 0.3, 30); }
    for c in 0..f.vel.len() { if f.solid[c].is_some() { assert_eq!(f.vel[c], [0.0; 3]); } }
    // just in front of the box (x = 5, the box starts at 6) the stream is slowed, and beside it (z = 7) it is faster than the wind
    let front = f.vel[f.idx([5, 2, 4])][0];
    let beside = f.vel[f.idx([7, 2, 7])][0];
    assert!(front < 3.0, "in front: {front}");
    assert!(beside > front, "beside {beside} vs front {front}");
}
#[test] fn dust_settles_at_the_set_speed_and_never_enters_the_ground() {
    let (mut f, w) = small();
    f.set_solids(|p| if p[1] < 1.0 { Some([0.0; 3]) } else { None }, &w);   // the ground: the lowest layer
    f.emit(&w, [8.5, 6.5, 8.5], 10.0);
    let y0 = f.centre_of_mass(&w)[1];
    for _ in 0..10 { f.advect_dust(&w, 0.2, 1.0, 0.0); }   // 2 s at 1 m/s
    let y1 = f.centre_of_mass(&w)[1];
    assert!((y0 - y1 - 2.0).abs() < 0.4, "fell {}", y0 - y1);
    for _ in 0..40 { f.advect_dust(&w, 0.2, 1.0, 0.0); }
    for c in 0..f.dens.len() { if f.solid[c].is_some() { assert_eq!(f.dens[c], 0.0); } }
    assert!(f.centre_of_mass(&w)[1] >= 1.0 - 1e-3, "dust is under the ground");
}
#[test] fn decay_removes_dust_over_time() {
    let (mut f, w) = small();
    f.emit(&w, [8.5, 4.5, 8.5], 10.0);
    for _ in 0..10 { f.advect_dust(&w, 0.2, 0.0, 0.5); }
    assert!(f.mass() < 10.0 * 0.5 && f.mass() > 10.0 * 0.2, "mass {}", f.mass());
}
#[test] fn the_window_recycles_exactly_the_cells_that_changed_hands() {
    let n = [16usize, 8, 16];
    let a = Window { start: [0, 0, 0], n };
    let b = Window { start: [3, 0, 0], n };
    let stale = (0..16).flat_map(|x| (0..8).flat_map(move |y| (0..16).map(move |z| [x, y, z]))).filter(|t| b.stale(&a, *t)).count();
    assert_eq!(stale, 3 * 8 * 16, "three columns of cells");
    assert!(!a.stale(&a, [5, 5, 5]));
    // every texel always holds a cell inside the window
    for t in [[0, 0, 0], [15, 7, 15], [7, 3, 9]] { let c = b.cell_of(t); for k in 0..3 { assert!(c[k] >= b.start[k] && c[k] < b.start[k] + n[k] as i32); } }
    // a big jump (more than a window) recycles every texel; a window around the car puts the car in the middle
    assert!((0..16).all(|x| Window { start: [100, 0, 0], n }.stale(&a, [x, 0, 0])));
    let w = Window::around([10.0, 2.0, -7.0], 0.75, [128, 20, 128], 3.0);
    assert!((w.start[0] as f32 + 64.0) * 0.75 - 10.0 < 1.0 && (w.start[1] as f32) * 0.75 < 2.0);
    // recycling resets the dust of those cells to nothing and the air to the wind
    let (mut f, wnd) = (Fluid::new(n, 1.0), a);
    f.wind = [1.0, 0.0, 0.0];
    f.dens.iter_mut().for_each(|d| *d = 1.0);
    f.recycle(&wnd, &b);
    assert_eq!(f.dens.iter().filter(|d| **d == 0.0).count(), 3 * 8 * 16);
}

#[test] fn the_wheel_emitters_follow_the_surface_and_the_ground_contact() {
    use crate::game::{grip, math::v3, surface::Surface, vehicle::{Vehicle, VehicleSpec, WheelInfo}};
    let table = grip::table();
    let mut v = Vehicle::new(VehicleSpec::offroader(), v3(5.0, 1.0, 5.0), 0.7);
    v.vel = v3(10.0, 0.0, 12.0);
    let wheel = |s: Surface, slide: f32| WheelInfo { grounded: true, surface: s, contact: v3(5.0, 0.4, 5.0), speed: 15.0, lat_slip: 2.0 * slide, slide };
    v.wheels = [wheel(Surface::Sand, 0.5); 4];
    let sand = pack(&v, &cfg(), &table);
    v.wheels = [wheel(Surface::Grass, 0.5); 4];
    let grass = pack(&v, &cfg(), &table);
    assert!(sand.wpos[0][3] > grass.wpos[0][3] * 2.0, "sand {} vs grass {}", sand.wpos[0][3], grass.wpos[0][3]);
    // the thrown air goes back (against the car's velocity) and up
    assert!(sand.wvel[0][0] < 0.0 && sand.wvel[0][2] < 0.0 && sand.wvel[0][1] > 1.0);
    // a wheel in the air emits nothing, a sliding one more than a rolling one
    v.wheels = [wheel(Surface::Gravel, 0.0); 4];
    let rolling = pack(&v, &cfg(), &table);
    v.wheels = [wheel(Surface::Gravel, 1.0); 4];
    let sliding = pack(&v, &cfg(), &table);
    assert!(sliding.wpos[0][3] > rolling.wpos[0][3] * 2.0);
    v.wheels[1].grounded = false;
    assert_eq!(pack(&v, &cfg(), &table).wpos[1][3], 0.0);
    // the car's box has the car's yaw and size; everything is finite
    let p = pack(&v, &cfg(), &table);
    assert!((p.car[3] - 0.7).abs() < 1e-3 && p.car_half[2] > 1.5);
    assert!(p.wpos.iter().chain(&p.wvel).chain(&p.wcol).all(|w| w.iter().all(|x| x.is_finite())));
}
