//! Glass dispersion optics shared by the scene code and the tests (the shaders get the same maths as generated text).

/// Refractive index at `lambda_nm`; `dispersion` 1.0 is ordinary crown glass, 0 switches dispersion off.
pub fn ior(lambda_nm: f32, base: f32, dispersion: f32) -> f32 {
    let l = lambda_nm * 1e-3;
    base + 0.012 * dispersion * ((0.589 / l) * (0.589 / l) - 1.0)
}

fn lobe(l: f32, mu: f32, s1: f32, s2: f32) -> f32 {
    let t = (l - mu) / if l < mu { s1 } else { s2 };
    (-0.5 * t * t).exp()
}

/// Wyman, Sloan and Shirley's multi-lobe fit of the CIE 1931 observer, converted to linear sRGB (negatives clamped).
pub fn wavelength_rgb(l: f32) -> [f32; 3] {
    let x = 1.056 * lobe(l, 599.8, 37.9, 31.0) + 0.362 * lobe(l, 442.0, 16.0, 26.7) - 0.065 * lobe(l, 501.1, 20.4, 26.2);
    let y = 0.821 * lobe(l, 568.8, 46.9, 40.5) + 0.286 * lobe(l, 530.9, 16.3, 31.1);
    let z = 1.217 * lobe(l, 437.0, 11.8, 36.0) + 0.681 * lobe(l, 459.0, 26.0, 13.8);
    [
        (3.2406 * x - 1.5372 * y - 0.4986 * z).max(0.0),
        (-0.9689 * x + 1.8758 * y + 0.0415 * z).max(0.0),
        (0.0557 * x - 0.2040 * y + 1.0570 * z).max(0.0),
    ]
}

/// Per-channel factor that makes the mean of `wavelength_rgb` over a uniform 380..780 nm sample equal to 1 (white).
pub fn spectrum_norm() -> [f32; 3] {
    let n = 400;
    let mut s = [0.0f32; 3];
    for i in 0..n {
        let c = wavelength_rgb(380.0 + 400.0 * (i as f32 + 0.5) / n as f32);
        for j in 0..3 { s[j] += c[j]; }
    }
    [n as f32 / s[0], n as f32 / s[1], n as f32 / s[2]]
}

/// Total deviation (degrees) of a ray through a prism with apex angle `apex_deg`, index `n`, first-face incidence `incidence_deg`.
pub fn deviation_deg(apex_deg: f32, n: f32, incidence_deg: f32) -> f32 {
    let a = apex_deg.to_radians();
    let i1 = incidence_deg.to_radians();
    let r1 = (i1.sin() / n).asin();
    let r2 = a - r1;
    let i2 = (n * r2.sin()).asin();
    (i1 + i2 - a).to_degrees()
}

pub fn min_deviation_deg(apex_deg: f32, n: f32) -> f32 {
    let a = apex_deg.to_radians();
    (2.0 * (n * (a * 0.5).sin()).asin() - a).to_degrees()
}

/// WGSL for the shaders: `ior_l(lambda_nm, base, dispersion)` and `wavelength_rgb(lambda_nm)` (already normalised to a white mean).
pub fn spectrum_wgsl() -> String {
    let k = spectrum_norm();
    format!(
        "fn ior_l(l_nm: f32, base: f32, disp: f32) -> f32 {{\n  let l = l_nm * 0.001;\n  return base + 0.012 * disp * ((0.589 / l) * (0.589 / l) - 1.0);\n}}\n\
fn lobe_w(l: f32, mu: f32, s1: f32, s2: f32) -> f32 {{\n  let t = (l - mu) / select(s2, s1, l < mu);\n  return exp(-0.5 * t * t);\n}}\n\
fn wavelength_rgb(l: f32) -> vec3<f32> {{\n\
  let x = 1.056 * lobe_w(l, 599.8, 37.9, 31.0) + 0.362 * lobe_w(l, 442.0, 16.0, 26.7) - 0.065 * lobe_w(l, 501.1, 20.4, 26.2);\n\
  let y = 0.821 * lobe_w(l, 568.8, 46.9, 40.5) + 0.286 * lobe_w(l, 530.9, 16.3, 31.1);\n\
  let z = 1.217 * lobe_w(l, 437.0, 11.8, 36.0) + 0.681 * lobe_w(l, 459.0, 26.0, 13.8);\n\
  let c = vec3<f32>(3.2406 * x - 1.5372 * y - 0.4986 * z, -0.9689 * x + 1.8758 * y + 0.0415 * z, 0.0557 * x - 0.2040 * y + 1.0570 * z);\n\
  return max(c, vec3<f32>(0.0)) * vec3<f32>({:?}, {:?}, {:?});\n}}\n",
        k[0], k[1], k[2]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ior_follows_the_cauchy_like_formula() {
        assert!((ior(589.0, 1.5, 1.0) - 1.5).abs() < 1e-4);
        let (b, r) = (ior(400.0, 1.5, 1.0), ior(700.0, 1.5, 1.0));
        assert!(b > r);
        assert!((b - r - 0.0177).abs() < 0.002, "crown glass spread {}", b - r);
        assert_eq!(ior(450.0, 1.62, 0.0), 1.62);
        assert!(ior(400.0, 1.5, 2.5) - ior(700.0, 1.5, 2.5) > 0.04);
    }

    #[test]
    fn spectrum_is_white_on_average() {
        let k = spectrum_norm();
        let mut sum = [0.0f32; 3];
        let n = 400;
        for i in 0..n {
            let c = wavelength_rgb(380.0 + 400.0 * (i as f32 + 0.5) / n as f32);
            for j in 0..3 { sum[j] += c[j] * k[j]; }
        }
        for j in 0..3 { assert!((sum[j] / n as f32 - 1.0).abs() < 0.01, "channel {j}: {}", sum[j] / n as f32); }
    }

    #[test]
    fn colours_are_where_they_should_be() {
        let red = wavelength_rgb(650.0);
        assert!(red[0] > red[1] && red[0] > red[2]);
        let green = wavelength_rgb(540.0);
        assert!(green[1] > green[0] && green[1] > green[2]);
        let blue = wavelength_rgb(450.0);
        assert!(blue[2] > blue[0] && blue[2] > blue[1]);
        assert!(wavelength_rgb(380.0).iter().all(|v| *v >= 0.0));
    }

    #[test]
    fn deviation_matches_closed_form() {
        // equilateral prism, n = 1.5: minimum deviation 2*asin(n*sin(30deg)) - 60deg = 37.18 deg
        assert!((min_deviation_deg(60.0, 1.5) - 37.18).abs() < 0.02);
        let inc_min = (1.5f32 * 30f32.to_radians().sin()).asin().to_degrees();
        assert!((deviation_deg(60.0, 1.5, inc_min) - min_deviation_deg(60.0, 1.5)).abs() < 0.02);
        assert!(deviation_deg(60.0, 1.5, inc_min + 15.0) > min_deviation_deg(60.0, 1.5));
        assert!(min_deviation_deg(60.0, ior(400.0, 1.5, 2.5)) > min_deviation_deg(60.0, ior(700.0, 1.5, 2.5)) + 2.0);
    }

    #[test]
    fn wgsl_snippet_has_both_functions() {
        let s = spectrum_wgsl();
        assert!(s.contains("fn ior_l(") && s.contains("fn wavelength_rgb("));
    }
}
