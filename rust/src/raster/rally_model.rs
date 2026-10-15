//! The generated rally hatchback (Vulpes WRC style): a lofted tub with flared arches, a greenhouse with a roof
//! scoop, a bumper intake with four spot lamps, a big rear wing on stands, mirrors and a side exhaust. The body frame
//! starts at the ground (+z forward, +y up, +x to the car's left); base dimensions 1.78 x 1.44 x 3.96 m are scaled to the spec.
use super::vehicle_models::{add_box, add_cylinder, build_wheel, loft, scaled, Axis, CarModel, Mesh3, CHROME, MAT_EMISSIVE, MAT_GLASS, MAT_PAINT, MAT_PLASTIC};
use crate::game::{math::{v3, Q}, vehicle::VehicleSpec};

const BLACK: [f32; 3] = [0.05, 0.05, 0.065];
const BLUE: [f32; 3] = [0.10, 0.22, 0.62];

const ASSET: &[u8] = include_bytes!("../../assets/rally_car.rcar");

/// The rally car: the simplified Vulpes WRC asset (see `assets/ASSET-LICENSE.md`), scaled to the spec size; the
/// generated hatchback below is the fallback when the asset cannot be read.
/// The lamp points of the asset (see `rcar::lamp_points`), found once.
pub fn lamp_points() -> Option<super::rcar::LampPoints> {
    static P: std::sync::OnceLock<Option<super::rcar::LampPoints>> = std::sync::OnceLock::new();
    *P.get_or_init(|| super::rcar::parse(ASSET).ok().and_then(|(b, _)| super::rcar::lamp_points(&b)))
}

pub fn build_rally(spec: &VehicleSpec) -> CarModel {
    match super::rcar::parse_all(ASSET) {
        Ok(super::rcar::Parts { mut body, mut wheel, mut disc, mut caliper, mut alt_wheels, mut glass, mut interior }) => {
            // the lamp units show the car's lamp state in the shader: their own material
            for v in &mut body.verts { if v.col[3] == super::vehicle_models::MAT_EMISSIVE { v.col[3] = super::vehicle_models::MAT_CARLAMP; } }
            for i in super::rcar::position_vertices(&body) { body.verts[i].col[3] = super::vehicle_models::MAT_CARPOS; }
            // the asset is modelled at 1.91 x 1.50 x 3.97 m with 0.326 m wheels: follow a changed spec size and radius
            let (sx, sy, sz) = (spec.size.x / 1.91, spec.size.y / 1.50, spec.size.z / 3.97);
            for v in &mut body.verts { v.pos = [v.pos[0] * sx, v.pos[1] * sy, v.pos[2] * sz]; }
            let k = spec.radius / 0.326;
            let [gravel, snow] = &mut alt_wheels;
            for m in [Some(&mut wheel), disc.as_mut(), caliper.as_mut(), gravel.as_mut(), snow.as_mut()].into_iter().flatten() {
                for v in &mut m.verts { v.pos = [v.pos[0] * spec.look.wheel_width / 0.226, v.pos[1] * k, v.pos[2] * k]; }
            }
            // the glass and the interior sit in the body's frame: they scale like the body
            for m in [glass.as_mut(), interior.as_mut()].into_iter().flatten() {
                for v in &mut m.verts { v.pos = [v.pos[0] * sx, v.pos[1] * sy, v.pos[2] * sz]; }
            }
            CarModel { body, wheel, disc, caliper, alt_wheels, glass, interior }
        }
        Err(e) => { log::error!("rally_car.rcar unusable ({e}); using the generated hatchback"); build_generated(spec) }
    }
}

fn build_generated(spec: &VehicleSpec) -> CarModel {
    let (sx, sy, sz) = (spec.size.x / 1.78, spec.size.y / 1.44, spec.size.z / 3.96);
    let l = spec.look;
    let p = |x: f32, y: f32, z: f32| v3(x * sx, y * sy, z * sz);          // a point in base-size metres
    let h = |x: f32, y: f32, z: f32| v3(x * sx, y * sy, z * sz);          // half extents scale the same way
    let mut b = Mesh3::default();
    // tub: rear panel to bumper
    b.append(&loft(&scaled(&[(-1.88, 0.60, 0.40, 0.92), (-1.78, 0.76, 0.32, 1.02), (-1.30, 0.83, 0.27, 1.10), (-0.30, 0.84, 0.25, 1.10),
        (0.80, 0.84, 0.25, 1.06), (1.40, 0.82, 0.27, 0.96), (1.80, 0.76, 0.30, 0.84), (2.08, 0.62, 0.34, 0.66)], sx, sy, sz), 16, 4.0, l.body, MAT_PAINT));
    // greenhouse glass, roof, scoop, pillars
    b.append(&loft(&scaled(&[(-1.40, 0.60, 1.06, 1.12), (-0.95, 0.70, 1.06, 1.40), (0.45, 0.70, 1.06, 1.42), (1.00, 0.62, 1.06, 1.12)], sx, sy, sz), 12, 3.0, l.glass, MAT_GLASS));
    add_box(&mut b, p(0.0, 1.415, -0.25), h(0.60, 0.012, 0.72), Q::IDENT, l.body, MAT_PAINT);
    add_box(&mut b, p(0.0, 1.455, -0.05), h(0.15, 0.045, 0.20), Q::IDENT, l.body, MAT_PAINT);
    add_box(&mut b, p(0.0, 1.455, 0.16), h(0.12, 0.03, 0.012), Q::IDENT, BLACK, MAT_PLASTIC);
    for x in [0.66f32, -0.66] {
        add_box(&mut b, p(x, 1.24, 0.74), h(0.025, 0.20, 0.03), Q::axis_angle(v3(1., 0., 0.), -0.95), l.body, MAT_PAINT);
        add_box(&mut b, p(x, 1.24, -1.18), h(0.025, 0.20, 0.03), Q::axis_angle(v3(1., 0., 0.), 0.92), l.body, MAT_PAINT);
    }
    // flared arches over the wheels
    for (x, z) in [(0.785f32, 1.225f32), (-0.785, 1.225), (0.785, -1.225), (-0.785, -1.225)] {
        add_box(&mut b, p(x, 0.765, z), h(0.165, 0.215, 0.43), Q::IDENT, l.body, MAT_PAINT);
    }
    // front: bumper intake, splitter, headlights and spot lamps; hood vents
    add_box(&mut b, p(0.0, 0.45, 2.02), h(0.50, 0.10, 0.05), Q::IDENT, BLACK, MAT_PLASTIC);
    add_box(&mut b, p(0.0, 0.30, 2.02), h(0.62, 0.02, 0.07), Q::IDENT, l.accent, MAT_PLASTIC);
    for x in [0.60f32, -0.60] { add_box(&mut b, p(x, 0.74, 1.96), h(0.14, 0.06, 0.04), Q::IDENT, [1.0, 0.96, 0.82], MAT_EMISSIVE); }
    for x in [0.18f32, -0.18, 0.42, -0.42] { add_cylinder(&mut b, p(x, 0.60, 2.03), Axis::Z, 0.055 * sx, 0.05 * sz, 8, [1.0, 0.95, 0.75], MAT_EMISSIVE); }
    for x in [0.30f32, -0.30] { add_box(&mut b, p(x, 0.99, 1.30), h(0.14, 0.012, 0.12), Q::IDENT, BLACK, MAT_PLASTIC); }
    // rear: tail lights, bumper, side exhaust
    for x in [0.58f32, -0.58] { add_box(&mut b, p(x, 0.86, -1.86), h(0.13, 0.05, 0.03), Q::IDENT, [0.9, 0.05, 0.03], MAT_EMISSIVE); }
    add_box(&mut b, p(0.0, 0.42, -1.86), h(0.60, 0.10, 0.04), Q::IDENT, l.accent, MAT_PLASTIC);
    add_cylinder(&mut b, p(0.40, 0.30, -1.90), Axis::Z, 0.045 * sx, 0.22 * sz, 10, CHROME, super::vehicle_models::MAT_CHROME);
    // rear wing: two stands, a tilted blade and end plates
    for x in [0.38f32, -0.38] { add_box(&mut b, p(x, 1.14, -1.60), h(0.02, 0.14, 0.03), Q::IDENT, BLACK, MAT_PLASTIC); }
    add_box(&mut b, p(0.0, 1.30, -1.72), h(0.58, 0.022, 0.22), Q::axis_angle(v3(1., 0., 0.), 0.12), l.body, MAT_PAINT);
    for x in [0.60f32, -0.60] { add_box(&mut b, p(x, 1.28, -1.72), h(0.012, 0.12, 0.24), Q::IDENT, l.accent, MAT_PLASTIC); }
    // mirrors, skirts, door roundels, livery bands
    for x in [0.93f32, -0.93] {
        add_box(&mut b, p(x * 0.957, 1.10, 0.55), h(0.06, 0.05, 0.045), Q::IDENT, l.body, MAT_PAINT);
        add_box(&mut b, p(x * 0.9, 1.07, 0.58), h(0.09, 0.012, 0.02), Q::IDENT, BLACK, MAT_PLASTIC);
    }
    for x in [0.82f32, -0.82] { add_box(&mut b, p(x, 0.30, -0.10), h(0.025, 0.05, 0.95), Q::IDENT, BLACK, MAT_PLASTIC); }
    for x in [0.852f32, -0.852] {
        add_cylinder(&mut b, p(x, 0.82, 0.10), Axis::X, 0.16 * sy, 0.012, 14, BLACK, MAT_PLASTIC);
        add_cylinder(&mut b, p(x * 1.003, 0.82, 0.10), Axis::X, 0.11 * sy, 0.014, 12, [0.95, 0.95, 0.97], MAT_PLASTIC);
    }
    for x in [0.843f32, -0.843] {
        add_box(&mut b, p(x, 0.58, -0.10), h(0.004, 0.04, 1.15), Q::IDENT, l.accent, MAT_PLASTIC);
        add_box(&mut b, p(x, 0.50, -0.10), h(0.004, 0.03, 1.15), Q::IDENT, BLUE, MAT_PLASTIC);
    }
    CarModel::new(b, build_wheel(spec, [0.92, 0.92, 0.95], 6))
}

#[cfg(test)]
#[path = "rally_model_tests.rs"]
mod tests;
