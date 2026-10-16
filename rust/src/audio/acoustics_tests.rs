use super::*;

fn flat(_: f32, _: f32) -> f32 { 0.0 }
fn l_db(o: &Occlusion) -> f32 { o.loss[1] }

#[test] fn maekawa_gives_about_five_db_at_the_shadow_edge_and_more_for_higher_notes_and_deeper_shadows() {
    assert!((maekawa_db(0.0, 1000.0) - 4.77).abs() < 0.1, "{}", maekawa_db(0.0, 1000.0));
    assert!(maekawa_db(0.3, 4000.0) > maekawa_db(0.3, 250.0) + 6.0, "the highs are lost first");
    assert!(maekawa_db(2.0, 1000.0) > maekawa_db(0.3, 1000.0));
    assert!(maekawa_db(1000.0, 4000.0) <= 24.0 + 1e-3 && maekawa_db(-5.0, 1000.0) == 0.0);
    assert!((detour(10.0, 10.0, 0.0)).abs() < 1e-5 && detour(10.0, 10.0, 2.0) > 0.3);
}
#[test] fn an_open_field_loses_nothing_and_a_ridge_between_takes_the_highs() {
    let open = occlusion(v3(0.0, 1.5, 0.0), v3(0.0, 1.0, 40.0), &flat, &[], None, [0.0; 3]);
    assert_eq!(open, Occlusion::default());
    let ridge = |x: f32, z: f32| { let _ = x; if (z - 20.0).abs() < 3.0 { 6.0 } else { 0.0 } };
    let o = occlusion(v3(0.0, 1.5, 0.0), v3(0.0, 1.0, 40.0), &ridge, &[], None, [0.0; 3]);
    assert!(o.loss[0] > 4.0 && o.loss[2] > o.loss[0] + 8.0 && o.extra_delay > 0.0, "{o:?}");
    let (g, hz) = o.gain_and_cutoff();
    assert!(g < 0.6 && hz < 3000.0, "{g} {hz}");
    // a low bump the line clears costs nothing
    let bump = |_: f32, z: f32| if (z - 20.0).abs() < 3.0 { 0.4 } else { 0.0 };
    assert_eq!(occlusion(v3(0.0, 1.5, 0.0), v3(0.0, 1.0, 40.0), &bump, &[], None, [0.0; 3]), Occlusion::default());
}
#[test] fn a_rock_is_a_screen_a_wall_is_muffled_and_a_crown_dulls_a_little() {
    let (l, s) = (v3(0.0, 1.5, 0.0), v3(0.0, 1.0, 30.0));
    let rock = Obstacle { shape: Shape::Sphere { c: v3(0.0, 1.2, 15.0), r: 1.5 }, kind: Kind::Hard };
    let crown = Obstacle { shape: Shape::Sphere { c: v3(0.0, 1.2, 15.0), r: 1.5 }, kind: Kind::Foliage };
    let wall = Obstacle { shape: Shape::Aabb { min: v3(-6.0, 0.0, 14.0), max: v3(6.0, 4.0, 16.0) }, kind: Kind::Wall };
    let miss = Obstacle { shape: Shape::Sphere { c: v3(8.0, 1.2, 15.0), r: 1.5 }, kind: Kind::Hard };
    let (r, c, w, m) = (occlusion(l, s, &flat, &[rock], None, [0.0; 3]), occlusion(l, s, &flat, &[crown], None, [0.0; 3]), occlusion(l, s, &flat, &[wall], None, [0.0; 3]), occlusion(l, s, &flat, &[miss], None, [0.0; 3]));
    assert!(l_db(&r) > 5.0 && r.loss[2] > r.loss[0], "{r:?}");
    assert!(c.loss[2] > 4.0 && c.loss[2] < 8.0 && c.extra_delay == 0.0, "{c:?}");
    assert!(l_db(&w) > 10.0 && w.loss[2] > w.loss[0], "a wall: {w:?}");
    assert_eq!(m, Occlusion::default(), "a rock off to the side is nothing");
}
#[test] fn the_cabin_muffles_the_outside_and_the_car_stands_in_its_own_way() {
    let cabin = Cabin { centre: v3(0.0, 0.0, 0.0), x: v3(1.0, 0.0, 0.0), y: v3(0.0, 1.0, 0.0), z: v3(0.0, 0.0, 1.0), min: v3(-0.9, 0.5, -1.0), max: v3(0.9, 1.4, 0.4) };
    let trans = [8.0, 14.0, 24.0];
    let inside = v3(0.3, 1.0, -0.2);
    assert!(cabin.contains(inside) && !cabin.contains(v3(0.0, 1.0, 5.0)));
    let outside_source = v3(0.0, 0.3, -3.0);
    let heard = occlusion(inside, outside_source, &flat, &[], Some(&cabin), trans);
    assert_eq!(heard.loss, trans, "{heard:?}");
    // both inside: nothing between them
    assert_eq!(occlusion(inside, v3(-0.3, 0.9, 0.1), &flat, &[], Some(&cabin), trans), Occlusion::default());
    // the listener stands beside the car and the source is on its far side: the body is in the way
    let beside = occlusion(v3(4.0, 1.0, 0.0), v3(-4.0, 1.0, 0.0), &flat, &[], Some(&cabin), trans);
    assert!(beside.loss[2] > 8.0, "{beside:?}");
}
#[test] fn the_ground_echoes_by_its_surface_and_there_is_none_underground() {
    let gc = |x: f32, z: f32| { let _ = (x, z); (0.5, 3000.0) };
    let r = ground_reflection(v3(0.0, 1.5, 0.0), v3(0.0, 0.5, 20.0), &flat, &gc).expect("an echo");
    assert!((r.image.y + 0.5).abs() < 1e-5, "the image is mirrored in the ground: {:?}", r.image);
    assert!(r.coeff == 0.5 && r.hz == 3000.0);
    assert!(ground_reflection(v3(0.0, -1.0, 0.0), v3(0.0, 0.5, 20.0), &flat, &gc).is_none());
    assert!(ground_reflection(v3(0.0, 1.5, 0.0), v3(0.0, 0.5, 20.0), &flat, &|_, _| (0.0, 1000.0)).is_none());
    use crate::game::surface::Surface::*;
    assert!(surface_reflection(Asphalt).0 > surface_reflection(Grass).0 && surface_reflection(Grass).1 < surface_reflection(Asphalt).1 && surface_reflection(Water).0 > 0.9);
}
#[test] fn a_hard_ground_echo_is_a_comb_filter_and_the_taps_stay_finite() {
    use super::super::spatial::{path, Emitter};
    use super::super::RATE;
    // a noise source 20 m away, 1.5 m high, over asphalt: the sum of the direct sound and the echo has dips; compare the power with and without the echo
    let l = v3(0.0, 1.5, 0.0); let src = v3(0.0, 0.5, 20.0);
    let r = ground_reflection(l, src, &flat, &|_, _| (0.93, 16000.0)).unwrap();
    let mut with = Emitter::new(); let mut without = Emitter::new();
    let d = path(l, v3(0.0, 0.0, 1.0), src).tap(); let mut e = path(l, v3(0.0, 0.0, 1.0), r.image).tap(); e.gain *= r.coeff;
    with.set_taps(&d, Some(&e)); without.set_taps(&d, None);
    let mut seed = 7u32; let (mut pw, mut po) = (0.0f32, 0.0f32);
    for i in 0..RATE as usize {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); let x = (seed >> 8) as f32 / 8_388_608.0 - 1.0;
        let (a, b) = with.process(x); let (c, dd) = without.process(x);
        assert!(a.is_finite() && b.is_finite());
        if i > 3000 { pw += a * a; po += c * c; }
    }
    assert!((pw / po - 1.0).abs() > 0.02, "the echo changes the sound: {pw} vs {po}");
}
