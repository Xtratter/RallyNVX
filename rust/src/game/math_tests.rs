use super::*;
fn near(a: V3, b: V3) -> bool { (a - b).len() < 1e-4 }
#[test] fn quat_rotates_z_to_x_about_y() {
    let q = Q::axis_angle(v3(0., 1., 0.), std::f32::consts::FRAC_PI_2);
    assert!(near(q.rotate(v3(0., 0., 1.)), v3(1., 0., 0.)));
}
#[test] fn quat_conj_inverts() {
    let q = Q::axis_angle(v3(1., 2., 3.), 0.8);
    let v = v3(0.3, -0.7, 1.1);
    assert!(near(q.conj().rotate(q.rotate(v)), v));
}
#[test] fn cross_is_right_handed() { assert!(near(v3(0., 1., 0.).cross(v3(0., 0., 1.)), v3(1., 0., 0.))); }
#[test] fn integrate_keeps_unit_length() {
    let mut q = Q::IDENT;
    for _ in 0..1000 { q = q.integrate(v3(1., 2., 3.), 1.0 / 60.0); }
    assert!((q.w * q.w + q.x * q.x + q.y * q.y + q.z * q.z - 1.0).abs() < 1e-4);
}
#[test] fn perspective_maps_near_far_to_0_1() {
    let p = perspective(1.0, 1.5, 0.5, 100.0);
    let n = transform(p, [0., 0., -0.5, 1.]); let f = transform(p, [0., 0., -100., 1.]);
    assert!((n[2] / n[3]).abs() < 1e-4 && (f[2] / f[3] - 1.0).abs() < 1e-4);
}
#[test] fn look_at_puts_target_on_minus_z() {
    let m = look_at(v3(0., 5., 10.), v3(0., 5., 0.), v3(0., 1., 0.));
    let t = transform(m, [0., 5., 0., 1.]);
    assert!(t[0].abs() < 1e-4 && t[1].abs() < 1e-4 && (t[2] + 10.0).abs() < 1e-4);
}
#[test] fn inverse_roundtrip() {
    let m = mat_mul(perspective(1.0, 1.5, 0.5, 100.0), look_at(v3(3., 4., 5.), v3(0., 0., 0.), v3(0., 1., 0.)));
    let id = mat_mul(m, inverse(m));
    for c in 0..4 { for r in 0..4 { assert!((id[c][r] - if c == r { 1.0 } else { 0.0 }).abs() < 1e-3, "{c}{r} {}", id[c][r]); } }
}
