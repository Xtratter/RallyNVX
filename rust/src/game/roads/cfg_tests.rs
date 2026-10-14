use super::*;

fn with(from: &str, to: &str) -> String { EMBEDDED.replacen(from, to, 1) }

#[test]
fn embedded_parses_with_sane_values() {
    let c = RoadsCfg::from_json(EMBEDDED).unwrap();
    assert_eq!(c.crest.series, (3, 5));
    assert!(c.crest.height.0 < c.crest.height.1);
    assert!(c.ditch_stage.depth > c.ditch_liaison.depth);
    assert!(c.rock_band.0 >= 4.0);
}

#[test]
fn bad_values_are_named() {
    for (from, to, field) in [
        ("\"height\": [0.6, 2.0]", "\"height\": [3.0, 2.0]", "height"),
        ("\"length\": [30, 60]", "\"length\": [0, 60]", "length"),
        ("\"depth\": 0.7", "\"depth\": -1", "depth"),
        ("\"gravel_width\": 0.8", "\"gravel_width\": -0.1", "gravel_width"),
        ("\"asym\": 0.35", "\"asym\": \"x\"", "asym"),
    ] {
        let e = RoadsCfg::from_json(&with(from, to)).unwrap_err();
        assert!(e.contains(field), "{field}: {e}");
    }
}

#[test]
fn garbage_and_missing_sections_are_errors() {
    assert!(RoadsCfg::from_json("not json").unwrap_err().contains("JSON"));
    assert!(RoadsCfg::from_json("{}").unwrap_err().contains("crest"));
}
