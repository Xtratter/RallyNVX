use super::*;
use crate::game::math::v3;

fn embedded() -> Params { Params::from_json(EMBEDDED).expect("the embedded vehicles.json must parse") }

#[test] fn the_offroader_keeps_its_known_numbers() {
    let c = embedded().car("offroader").expect("offroader");
    let (m, w, h, l) = (1500.0f32, 2.0f32, 1.4f32, 4.2f32);
    assert_eq!(c.mass, 1500.0);
    assert_eq!(c.inertia, v3(m / 12.0 * (h * h + l * l), m / 12.0 * (w * w + l * l), m / 12.0 * (w * w + h * h)));
    assert_eq!(c.wheels, [v3(-0.95, 0.35, 1.4), v3(0.95, 0.35, 1.4), v3(-0.95, 0.35, -1.4), v3(0.95, 0.35, -1.4)]);
    assert_eq!((c.rest, c.radius, c.spring, c.damper), (0.6, 0.45, 30000.0, 3500.0));
    assert_eq!((c.engine_force, c.brake_force, c.max_speed, c.mu), (14000.0, 16000.0, 35.0, 1.1));
    assert_eq!((c.steer_max, c.steer_speed_ref, c.steer_rate, c.lateral_k, c.rolling), (0.55, 15.0, 3.0, 12.0, 40.0));
    assert_eq!((c.aero, c.ang_damp, c.handbrake_grip, c.reverse_factor, c.max_susp_factor), (0.35, 0.8, 0.4, 0.4, 5.0));
}
#[test] fn the_active_parameters_are_the_embedded_ones_by_default() {
    if std::env::var_os("RAYTRACE_VEHICLES").is_none() { assert_eq!(get().car("offroader").unwrap().mass, 1500.0); }
}
#[test] fn an_override_changes_the_specs_and_inertia_scales_with_mass() {
    let text = EMBEDDED.replacen("\"mass\": 1500", "\"mass\": 3000", 1);
    assert_ne!(text, EMBEDDED, "the replacement must hit the mass field");
    let c = Params::from_json(&text).unwrap().car("offroader").unwrap();
    assert_eq!(c.mass, 3000.0);
    assert!((c.inertia.x / embedded().car("offroader").unwrap().inertia.x - 2.0).abs() < 1e-4);
}
#[test] fn unknown_car_ids_are_none() { assert!(embedded().car("hovercraft").is_none()); }
#[test] fn errors_are_readable() {
    let e = Params::from_json("not json").unwrap_err(); assert!(!e.is_empty());
    let e = Params::from_json("{}").unwrap_err(); assert!(e.contains("cars"), "{e}");
    let no_mass = EMBEDDED.replacen("\"mass\": 1500,", "", 1);
    let e = Params::from_json(&no_mass).unwrap_err(); assert!(e.contains("mass") && e.contains("offroader"), "{e}");
    let neg = EMBEDDED.replacen("\"mass\": 1500", "\"mass\": -5", 1);
    let e = Params::from_json(&neg).unwrap_err(); assert!(e.contains("mass"), "{e}");
    let bad_wheels = EMBEDDED.replacen("[-0.95, 0.35, 1.4]", "[-0.95, 0.0]", 1);
    let e = Params::from_json(&bad_wheels).unwrap_err(); assert!(e.contains("wheel"), "{e}");
}
#[test] fn look_parses_with_defaults_overrides_and_readable_errors() {
    let p = embedded();
    let c = p.car("offroader").unwrap();
    assert_eq!(c.size, v3(2.0, 1.4, 4.2));
    assert!(c.look.body.iter().all(|v| (0.0..=1.0).contains(v)) && c.look.wheel_width > 0.1);
    // a file without any look section still loads, with defaults
    let no_look = EMBEDDED.replace("\"look\":", "\"_unused\":");
    let q = Params::from_json(&no_look).unwrap();
    for l in [q.car("offroader").unwrap().look] {
        assert!(l.body.iter().chain(&l.accent).chain(&l.glass).all(|v| (0.0..=1.0).contains(v)) && l.wheel_width > 0.0);
    }
    let custom = EMBEDDED.replacen("\"body_color\": [0.75, 0.12, 0.08]", "\"body_color\": [0.1, 0.2, 0.3]", 1);
    assert_ne!(custom, EMBEDDED);
    assert_eq!(Params::from_json(&custom).unwrap().car("offroader").unwrap().look.body, [0.1, 0.2, 0.3]);
    let bad = EMBEDDED.replacen("\"body_color\": [0.75, 0.12, 0.08]", "\"body_color\": [1.5, 0.2, 0.3]", 1);
    let e = Params::from_json(&bad).unwrap_err(); assert!(e.contains("look") && e.contains("body_color"), "{e}");
    let bad2 = EMBEDDED.replacen("\"wheel_width\": 0.32", "\"wheel_width\": 5.0", 1);
    assert!(Params::from_json(&bad2).unwrap_err().contains("wheel_width"));
}
#[test] fn water_drag_is_read_and_validated() {
    assert!(embedded().car("offroader").unwrap().water_drag > 0.0);
    let bad = EMBEDDED.replacen("\"water_drag\": 1.4", "\"water_drag\": -1.0", 1);
    let e = Params::from_json(&bad).unwrap_err(); assert!(e.contains("water_drag"), "{e}");
}
#[test] fn cars_carry_model_price_and_upgrade_cost_with_defaults() {
    let p = embedded();
    let ids = p.car_ids(); assert_eq!(ids, vec!["offroader".to_string(), "vulpes".to_string()]);
    let o = p.car("offroader").unwrap(); assert_eq!((o.style, o.price, o.upgrade_cost), (CarStyle::Suv, 0, 300));
    let g = p.car("vulpes").unwrap();
    assert_eq!((g.style, g.price, g.upgrade_cost), (CarStyle::Rally, 3000, 750));
    assert_eq!((g.mass, g.size.x, g.size.y, g.size.z), (1230.0, 1.91, 1.50, 3.97));
    assert_eq!(g.wheels, [v3(-0.767, 0.10, 1.237), v3(0.767, 0.10, 1.237), v3(-0.767, 0.10, -1.210), v3(0.767, 0.10, -1.210)]);
    assert_eq!((g.rest, g.radius, g.spring, g.damper, g.engine_force, g.brake_force, g.max_speed, g.mu), (0.32, 0.326, 32000.0, 3200.0, 18000.0, 14000.0, 52.0, 1.2));
}
#[test] fn bad_model_or_price_is_a_readable_error() {
    let bad = EMBEDDED.replacen("\"model\": \"rally\"", "\"model\": \"boat\"", 1);
    let e = Params::from_json(&bad).unwrap_err(); assert!(e.contains("model"), "{e}");
    let bad = EMBEDDED.replacen("\"price\": 3000", "\"price\": -5", 1);
    let e = Params::from_json(&bad).unwrap_err(); assert!(e.contains("price"), "{e}");
    let none = EMBEDDED.replacen("\"model\": \"rally\",", "", 1).replacen("\"price\": 3000,", "", 1).replacen("\"upgrade_cost\": 750,", "", 1);
    let p = Params::from_json(&none).unwrap(); let g = p.car("vulpes").unwrap();
    assert_eq!((g.style, g.price, g.upgrade_cost), (CarStyle::Suv, 0, 300));
}
