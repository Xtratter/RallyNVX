//! The host twins of the shader's physically based lighting (`raster.wgsl`: `ggx_d`, `smith_vis`, `fresnel`, `env_brdf`), so that the maths is tested without a GPU:
//! Cook-Torrance with a GGX distribution, Smith-GGX height-correlated visibility and Schlick Fresnel; the ambient specular uses Karis' analytic split-sum fit.

/// The GGX normal distribution for `a` = roughness squared; integrates to 1 over the hemisphere with the cosine of the half vector.
pub fn ggx_d(nh: f32, a: f32) -> f32 {
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    a2 / (std::f32::consts::PI * d * d).max(1e-9)
}

/// Smith-GGX height-correlated visibility already divided by `4 n.l n.v`.
pub fn smith_vis(nl: f32, nv: f32, a: f32) -> f32 {
    let a2 = a * a;
    let gv = nl * (nv * nv * (1.0 - a2) + a2).sqrt();
    let gl = nv * (nl * nl * (1.0 - a2) + a2).sqrt();
    0.5 / (gv + gl + 1e-5)
}

/// Schlick's Fresnel: `f0` at normal incidence rising to 1 at grazing angles.
pub fn fresnel(f0: [f32; 3], vh: f32) -> [f32; 3] {
    let k = (1.0 - vh.clamp(0.0, 1.0)).powi(5);
    f0.map(|f| f + (1.0 - f) * k)
}

/// The split-sum environment term: what the ambient specular multiplies the environment colour by.
pub fn env_brdf(f0: [f32; 3], rough: f32, nv: f32) -> [f32; 3] {
    let c0 = [-1.0f32, -0.0275, -0.572, 0.022];
    let c1 = [1.0f32, 0.0425, 1.04, -0.04];
    let r = [rough * c0[0] + c1[0], rough * c0[1] + c1[1], rough * c0[2] + c1[2], rough * c0[3] + c1[3]];
    let a004 = (r[0] * r[0]).min((-9.28 * nv).exp2()) * r[0] + r[1];
    let ab = [-1.04 * a004 + r[2], 1.04 * a004 + r[3]];
    f0.map(|f| f * ab[0] + ab[1])
}

#[cfg(test)]
#[path = "pbr_tests.rs"]
mod tests;
