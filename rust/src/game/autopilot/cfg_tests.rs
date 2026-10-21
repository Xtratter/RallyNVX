use super::*;

#[test] fn embedded_parses() {
    let c = AutoCfg::from_json(EMBEDDED).unwrap();
    assert!(c.lat_factor > 0.0 && c.lat_factor < 2.0 && c.stuck_time >= 1.0);
}
#[test] fn bad_values_are_named() {
    for (from, to, field) in [("\"lat_factor\": 1.3", "\"lat_factor\": 5", "lat_factor"), ("\"look_base\": 6.0", "\"look_base\": 0", "look_base"), ("\"stuck_time\": 4.0", "\"stuck_time\": \"x\"", "stuck_time")] {
        let e = AutoCfg::from_json(&EMBEDDED.replacen(from, to, 1)).unwrap_err();
        assert!(e.contains(field), "{e}");
    }
    assert!(AutoCfg::from_json("nope").unwrap_err().contains("JSON"));
    assert!(AutoCfg::from_json("{}").unwrap_err().contains("missing"));
}

#[test] fn the_custom_style_is_clamped_and_falls_back_to_normal() {
    reset_custom();
    assert_eq!(cfg_style(3), cfg());
    set_custom(9.0, -1.0, f32::NAN, 0.1, 5.0, 100.0);
    let c = cfg_style(3);
    assert!(c.lat_factor <= 3.0 && c.brake_factor >= 0.2 && c.offset_max == cfg().offset_max && c.speed_cap >= 0.3 && c.slide <= 1.0 && c.edge_margin <= 4.0);
    set_custom(1.0, 0.8, 0.5, 0.8, 1.0, 2.0);
    assert_eq!((cfg_style(3).lat_factor, cfg_style(3).speed_cap, cfg_style(3).slide, cfg_style(3).edge_margin), (1.0, 0.8, 1.0, 2.0));
    reset_custom();
}
