//! WGSL sources: every pass is `common.wgsl` + its own body. Ungated so the host
//! (CI) can validate all shaders with naga without a GPU.

const COMMON: &str = include_str!("shaders/common.wgsl");
const SCENE: &str = include_str!("shaders/scene.wgsl");

fn join(body: &str) -> String { format!("{}\n{}", COMMON, body) }
fn join_scene(sc: &crate::scene::SceneData, body: &str) -> String {
    format!("{}\n{}\n{}\n{}\n{}\n{}", COMMON, SCENE, crate::scenegen::scene_wgsl(sc), crate::scenegen::caustic_at_wgsl(sc, true), crate::dispersion::spectrum_wgsl(), body)
}

/// Photon pass: sun photons through the prism into the receivers' counters (scene must satisfy `SceneData::photons`).
pub fn photon(sc: &crate::scene::SceneData) -> String {
    format!("{}\n{}\n{}\n{}\n{}\n{}\n{}", COMMON, SCENE, crate::scenegen::scene_wgsl(sc), crate::scenegen::caustic_at_wgsl(sc, false), crate::dispersion::spectrum_wgsl(), crate::scenegen::photon_wgsl(sc), include_str!("shaders/photon.wgsl"))
}

/// Resolve pass: counters -> exponentially averaged irradiance map.
pub fn caustic_resolve(sc: &crate::scene::SceneData) -> String {
    format!("{}\nconst PI: f32 = 3.14159265;\n{}\n{}", COMMON, crate::scenegen::photon_wgsl(sc), include_str!("shaders/caustic_resolve.wgsl"))
}

pub fn trace(sc: &crate::scene::SceneData) -> String { join_scene(sc, include_str!("shaders/trace.wgsl")) }
pub fn temporal() -> String { join(include_str!("shaders/temporal.wgsl")) }
pub fn atrous() -> String { join(include_str!("shaders/atrous.wgsl")) }
pub fn gi_trace(sc: &crate::scene::SceneData) -> String { join_scene(sc, include_str!("shaders/gi_trace.wgsl")) }
pub fn gi_temporal() -> String { join(include_str!("shaders/gi_temporal.wgsl")) }
pub fn gi_atrous() -> String { join(include_str!("shaders/gi_atrous.wgsl")) }
pub fn composite() -> String { join(include_str!("shaders/composite.wgsl")) }
pub fn raster() -> String { include_str!("shaders/raster.wgsl").to_string() }
pub fn fluid() -> String { include_str!("shaders/fluid.wgsl").to_string() }
pub fn airflow() -> String { include_str!("shaders/airflow.wgsl").to_string() }
pub fn present() -> String { join(include_str!("shaders/present.wgsl")) }

/// Parses and validates WGSL with naga (host tests; no GPU needed).
#[cfg(test)]
pub(crate) fn validate_for_tests(name: &str, src: String) {
    use wgpu::naga;
    let m = naga::front::wgsl::parse_str(&src).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&src)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&m).unwrap_or_else(|e| panic!("{name}: {e:?}"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::validate_for_tests as validate;

    #[test]
    fn shaders_validate() {
        for i in 0..crate::scene::BUILTIN.len() {
            let sc = crate::scene::builtin(i);
            validate(&format!("trace/{}", sc.name), trace(&sc));
            validate(&format!("gi_trace/{}", sc.name), gi_trace(&sc));
        }
        validate("temporal", temporal());
        validate("atrous", atrous());
        validate("present", present());
        validate("gi_temporal", gi_temporal());
        validate("gi_atrous", gi_atrous());
        validate("composite", composite());
        validate("raster", raster());
        validate("fluid", fluid());
        validate("airflow", airflow());
    }
}
