//! Embedded exact inputs, shared by every installed wheel. No filesystem I/O.
use crate::{DataError, GeometryData, RuleData, TheoryData};
use serde::Deserialize;

pub const PRESET_NAMES: [&str; 3] = ["Z3", "Z6-II", "Z3xZ3"];

#[derive(Deserialize)]
pub struct PresetData {
    pub geometry: GeometryData,
    pub rules: Vec<RuleData>,
    pub geometry_sha256: String,
}

pub fn preset(name: &str, theory: TheoryData) -> Result<PresetData, DataError> {
    let text = match name {
        "Z3" => include_str!("../../../python/orbifolder/presets/Z3.json"),
        "Z6-II" | "Z6_II" => include_str!("../../../python/orbifolder/presets/Z6-II.json"),
        "Z3xZ3" => include_str!("../../../python/orbifolder/presets/Z3xZ3.json"),
        _ => {
            return Err(DataError(format!(
                "Unknown preset {name:?}; choose Z3, Z6-II or Z3xZ3"
            )));
        }
    };
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| DataError(e.to_string()))?;
    let key = match theory {
        TheoryData::Susy => "susy",
        TheoryData::NonSusy => "non_susy",
    };
    serde_json::from_value(value[key].clone()).map_err(|e| DataError(e.to_string()))
}
