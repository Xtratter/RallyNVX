use super::*;

#[test] fn embedded_parses() {
    let c = AutoCfg::from_json(EMBEDDED).unwrap();
    assert!(c.lat_factor > 0.0 && c.lat_factor < 1.0 && c.stuck_time >= 1.0);
}
#[test] fn bad_values_are_named() {
    for (from, to, field) in [("\"lat_factor\": 0.9", "\"lat_factor\": 5", "lat_factor"), ("\"look_base\": 6.0", "\"look_base\": 0", "look_base"), ("\"stuck_time\": 4.0", "\"stuck_time\": \"x\"", "stuck_time")] {
        let e = AutoCfg::from_json(&EMBEDDED.replacen(from, to, 1)).unwrap_err();
        assert!(e.contains(field), "{e}");
    }
    assert!(AutoCfg::from_json("nope").unwrap_err().contains("JSON"));
    assert!(AutoCfg::from_json("{}").unwrap_err().contains("missing"));
}
