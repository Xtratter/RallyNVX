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

#[test] fn the_learned_style_holds_the_taught_numbers_inside_the_search_bounds_and_ignores_garbage() {
    use super::super::train::{lo, hi, Mode};
    reset_learned();
    assert_eq!(cfg_style(4), cfg(), "before any training the learned style is the normal one");
    set_learned(&[9.0, -1.0, 0.6, 1.0, 7.0, 0.3, 0.0]);
    let c = cfg_style(4);
    assert_eq!((c.lat_factor, c.brake_factor), (hi(Mode::Safe)[0], lo(Mode::Safe)[1]), "clamped into the bounds");
    assert!((c.offset_max - 0.6).abs() < 1e-4 && (c.look_base - 7.0).abs() < 1e-4 && c.slide == cfg().slide);
    set_learned(&[f32::NAN, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]); set_learned(&[1.0, 2.0]);
    assert_eq!(cfg_style(4).look_base, c.look_base, "garbage and short input change nothing");
    assert_eq!(cfg_style(3), custom());
    reset_learned();
}

#[test] fn the_time_attack_style_is_the_files_until_it_is_taught_and_the_taught_numbers_are_bounded() {
    use super::super::train::{lo, hi, Mode};
    reset_attack();
    let a = cfg_style(5);
    assert_eq!(a.attack, 1.0); assert!(a.slide > 0.5 && a.edge_margin < 0.5 && a.lat_factor > cfg().lat_factor * 0.8, "the style is all out: {a:?}");
    assert_eq!(cfg().attack, 0.0); assert_eq!(cfg_style(0).attack + cfg_style(1).attack + cfg_style(2).attack, 0.0, "only Time Attack attacks");
    set_attack(&[9.0, 9.0, 9.0, -9.0, 9.0, 9.0, 0.0]);
    let t = cfg_style(5);
    assert_eq!((t.lat_factor, t.brake_factor, t.edge_margin, t.slide), (hi(Mode::Attack)[0], hi(Mode::Attack)[1], lo(Mode::Attack)[3], lo(Mode::Attack)[6]));
    assert_eq!(t.attack, 1.0);
    set_attack(&[1.0; 3]); set_attack(&[f32::NAN; 7]);
    assert_eq!(cfg_style(5).lat_factor, t.lat_factor, "short or non-finite input is ignored");
    reset_attack();
}
