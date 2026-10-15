use super::*;

#[test]
fn embedded_config_parses() {
    let c = cfg();
    assert_eq!(c.n, [48, 24, 96]); assert!((c.cell - 0.25).abs() < 1e-6); assert!(!c.debug);
}
#[test]
fn bad_or_missing_block_is_a_clear_error() {
    assert!(AirflowCfg::from_json("{}").unwrap_err().contains("airflow"));
    let bad = r#"{"airflow":{"cell":0.01,"nx":48,"ny":24,"nz":96,"iterations":8,"substeps":1,"confinement":0.5}}"#;
    assert!(AirflowCfg::from_json(bad).unwrap_err().contains("cell"));
}
