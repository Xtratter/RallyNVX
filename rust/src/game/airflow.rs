//! The airflow around the car (aero step 2, see docs/superpowers/specs/2026-10-15-airflow-design.md): the CPU reference solver in the car's
//! frame (x lateral, y up, z forward) that the compute shaders mirror, and the voxel body.

/// The numbers of `surfaces.json` -> `airflow`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirflowCfg { pub debug: bool, pub cell: f32, pub n: [usize; 3], pub iterations: u32, pub substeps: u32, pub confinement: f32 }

impl AirflowCfg {
    pub fn from_json(text: &str) -> Result<AirflowCfg, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("surfaces file is not valid JSON: {e}"))?;
        let f = v.get("airflow").filter(|o| o.is_object()).ok_or("surfaces: missing object \"airflow\"")?;
        let num = |key: &str, lo: f64, hi: f64| -> Result<f32, String> {
            let x = f.get(key).and_then(|k| k.as_f64()).filter(|x| x.is_finite()).ok_or_else(|| format!("airflow: missing or non-numeric \"{key}\""))?;
            if x < lo || x > hi { return Err(format!("airflow: \"{key}\" must be between {lo} and {hi} (got {x})")); }
            Ok(x as f32)
        };
        let debug = f.get("debug").and_then(|d| d.as_f64()).map_or(false, |d| d > 0.5);
        Ok(AirflowCfg { debug, cell: num("cell", 0.1, 1.0)?, n: [num("nx", 8.0, 128.0)? as usize, num("ny", 8.0, 64.0)? as usize, num("nz", 16.0, 256.0)? as usize],
            iterations: num("iterations", 1.0, 64.0)? as u32, substeps: num("substeps", 1.0, 4.0)? as u32, confinement: num("confinement", 0.0, 2.0)? })
    }
}

pub fn cfg() -> AirflowCfg { AirflowCfg::from_json(super::grip::EMBEDDED).expect("the embedded surfaces.json has a bad airflow block") }

#[cfg(test)]
#[path = "airflow_tests.rs"]
mod tests;
