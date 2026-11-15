//! Reproducibly export only model/geometry INPUTS. No expected states enter the API.
#[path = "../../orbifolder-core/tests/common/mod.rs"]
mod common;
#[path = "../../orbifolder-core/tests/common/geometry_input.rs"]
mod geometry_input;
use orbifolder_data::{Exact, GeometryData, ModelData, RuleData, TheoryData};
use serde_json::{Value, json};
fn rules(d: &Value) -> Vec<RuleData> {
    if d["variant"] != "susy" {
        return Vec::new();
    }
    let text = geometry_input::geometry_text(
        "susy",
        d["input"]["model_text"].as_str().unwrap(),
        Some(d["input"]["geometry_sha256"].as_str().unwrap()),
    );
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with("//"))
        .collect::<Vec<_>>();
    let mut i = lines
        .iter()
        .position(|&s| s == "begin discrete symmetries")
        .unwrap()
        + 1;
    let mut out = Vec::new();
    while lines[i] != "end discrete symmetries" {
        let r = lines[i] == "new R symmetry";
        assert!(r || lines[i] == "new non-R symmetry");
        let mut op = lines[i + 3]
            .split_whitespace()
            .map(|s| Exact(s.parse().unwrap()))
            .collect::<Vec<_>>();
        if !r {
            op.insert(2, Exact(0.into()));
        }
        out.push(RuleData {
            kind: if r { "r" } else { "non_r" }.into(),
            order: lines[i + 1].parse().unwrap(),
            target: Exact(lines[i + 2].parse().unwrap()),
            operator: op,
        });
        i += if r { 4 } else { 5 };
    }
    out
}
fn main() {
    let cases = [
        "susy_z3",
        "susy_z6_ii",
        "susy_z3xz3_wl",
        "non_susy_z3",
        "non_susy_z6_ii",
        "non_susy_z3xz3_wl",
        "non_susy_tachyonic",
    ];
    let mut inputs = serde_json::Map::new();
    for case in cases {
        let d = common::json(&format!("fixtures/{case}.json"));
        let text = d["input"]["model_text"].as_str().unwrap();
        let geometry = GeometryData::from_spec(&geometry_input::from_fixture(&d));
        let mut model = ModelData::from_text(
            text,
            geometry,
            if d["variant"] == "susy" {
                TheoryData::Susy
            } else {
                TheoryData::NonSusy
            },
        )
        .unwrap();
        model.rules = rules(&d);
        // This also proves the production text adapter agrees with prior input parsing.
        let old = geometry_input::embedding(d["variant"].as_str().unwrap(), text);
        let new = model.embedding().unwrap();
        assert_eq!(old.shifts, new.shifts);
        assert_eq!(old.wilson_lines, new.wilson_lines);
        inputs.insert(case.into(),json!({"model":model,"model_text":text,"model_sha256":d["input"]["model_sha256"],"geometry_sha256":d["input"]["geometry_sha256"]}));
    }
    println!("{}", Value::Object(inputs));
}
