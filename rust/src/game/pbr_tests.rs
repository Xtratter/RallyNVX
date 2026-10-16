use super::*;

#[test] fn the_ggx_distribution_integrates_to_one_over_the_hemisphere() {
    // integral of D(h) cos(theta_h) over the hemisphere (2 pi sin t dt) = 1 for every roughness (to the accuracy of the quadrature)
    for rough in [0.2f32, 0.4, 0.7, 1.0] {
        let a = rough * rough;
        let n = 20000;
        let mut sum = 0.0f64;
        for i in 0..n {
            let t = (i as f32 + 0.5) / n as f32 * std::f32::consts::FRAC_PI_2;
            sum += (ggx_d(t.cos(), a) * t.cos() * t.sin()) as f64 * 2.0 * std::f64::consts::PI * (std::f32::consts::FRAC_PI_2 / n as f32) as f64;
        }
        assert!((sum - 1.0).abs() < 0.02, "roughness {rough}: {sum}");
    }
}
#[test] fn a_smoother_surface_has_a_taller_and_narrower_highlight() {
    assert!(ggx_d(1.0, 0.04) > ggx_d(1.0, 0.25) * 10.0);
    assert!(ggx_d(0.95, 0.04) < ggx_d(1.0, 0.04) * 0.05, "narrow");
    assert!(ggx_d(0.95, 0.5) > ggx_d(0.95, 0.04), "a rough surface spreads the light");
}
#[test] fn the_visibility_is_finite_positive_and_shrinks_the_grazing_light() {
    for nl in [0.02f32, 0.3, 1.0] { for nv in [0.02f32, 0.3, 1.0] { for a in [0.01f32, 0.25, 1.0] { let v = smith_vis(nl, nv, a); assert!(v.is_finite() && v > 0.0, "{nl} {nv} {a}: {v}"); } } }
    // the full term D * vis stays below a sane ceiling for a mirror-like surface at a small angle (no explosion that would burn the screen)
    assert!(ggx_d(1.0, 0.01) * smith_vis(1.0, 1.0, 0.01) < 1.0e5);
}
#[test] fn fresnel_is_f0_head_on_and_one_at_grazing_and_a_metal_keeps_its_colour() {
    let f = fresnel([0.04; 3], 1.0); assert!((f[0] - 0.04).abs() < 1e-6);
    let g = fresnel([0.04; 3], 0.0); assert!((g[0] - 1.0).abs() < 1e-6);
    let gold = fresnel([1.0, 0.77, 0.34], 0.8); assert!(gold[0] > gold[1] && gold[1] > gold[2], "{gold:?}");
    let mid = fresnel([0.04; 3], 0.5)[0]; assert!(mid > 0.04 && mid < 0.1, "{mid}");
}
#[test] fn the_environment_term_is_a_fraction_that_grows_with_f0_and_never_exceeds_one() {
    for rough in [0.05f32, 0.3, 0.6, 1.0] { for nv in [0.05f32, 0.4, 1.0] {
        let dielectric = env_brdf([0.04; 3], rough, nv)[0]; let metal = env_brdf([1.0; 3], rough, nv)[0];
        assert!((0.0..=1.0).contains(&dielectric) && (0.0..=1.0 + 1e-3).contains(&metal), "{rough} {nv}: {dielectric} {metal}");
        assert!(metal > dielectric, "a metal reflects more of the sky");
    } }
    assert!(env_brdf([0.04; 3], 0.1, 0.05)[0] > env_brdf([0.04; 3], 0.1, 1.0)[0], "grazing angles reflect more (Fresnel)");
}
