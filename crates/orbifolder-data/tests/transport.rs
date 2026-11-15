use orbifolder_data::{Exact, ModelData};
use serde_json::Value;
#[test]
fn exact_roundtrip_bigints_signed_denominators_and_wrong_types() {
    for text in ["123", "\"-1/3\"", "[1,-3]", "{\"num\":1,\"den\":-3}"] {
        let value: Exact = serde_json::from_str(text).unwrap();
        let output = serde_json::to_string(&value).unwrap();
        let roundtrip: Exact = serde_json::from_str(&output).unwrap();
        assert_eq!(value, roundtrip);
    }
    let large = Exact(orbifolder_core::math_core::Rational::from(
        num_bigint::BigInt::from(1) << 20000_usize,
    ));
    assert_eq!(
        serde_json::from_str::<Exact>(&serde_json::to_string(&large).unwrap()).unwrap(),
        large
    );
    for bad in [
        "true",
        "1.0",
        "1e2",
        "[1,0]",
        "{\"num\":1,\"den\":0}",
        "[1,2,3]",
    ] {
        assert!(serde_json::from_str::<Exact>(bad).is_err(), "{bad}");
    }
}
#[test]
fn all_reference_input_roundtrips_match_text_and_reject_wrong_shapes() {
    let all: Value =
        serde_json::from_str(include_str!("../../../tests/python/model_inputs.json")).unwrap();
    for entry in all.as_object().unwrap().values() {
        let model: ModelData = serde_json::from_value(entry["model"].clone()).unwrap();
        let rebuilt = ModelData::from_text(
            entry["model_text"].as_str().unwrap(),
            model.geometry.clone(),
            model.theory,
        )
        .unwrap();
        assert_eq!(
            model.embedding().unwrap().shifts,
            rebuilt.embedding().unwrap().shifts
        );
        assert_eq!(
            model.embedding().unwrap().wilson_lines,
            rebuilt.embedding().unwrap().wilson_lines
        );
        let mut bad = model.clone();
        bad.shifts[0].pop();
        assert!(bad.embedding().is_err());
        let mut bad = model.clone();
        bad.schema_version = 2;
        assert!(bad.request(None).is_err());
        let geometry = model.geometry.build().unwrap();
        assert!(model.request(Some(geometry.clone())).is_ok());
        let mut changed = model.clone();
        changed.geometry.generators[0].twist[0] = Exact(2.into());
        assert!(changed.request(Some(geometry)).is_err());
    }
}

#[test]
fn embedded_presets_match_all_seven_input_conventions() {
    let inputs: Value =
        serde_json::from_str(include_str!("../../../tests/python/model_inputs.json")).unwrap();
    for (case, entry) in inputs.as_object().unwrap() {
        let model: ModelData = serde_json::from_value(entry["model"].clone()).unwrap();
        let name = if case.contains("z3xz3") {
            "Z3xZ3"
        } else if case.contains("z6_ii") || case.contains("tachyonic") {
            "Z6-II"
        } else {
            "Z3"
        };
        let preset = orbifolder_data::preset(name, model.theory).unwrap();
        assert_eq!(
            preset.geometry_sha256,
            entry["geometry_sha256"].as_str().unwrap()
        );
        assert_eq!(
            serde_json::to_value(&preset.rules).unwrap(),
            serde_json::to_value(&model.rules).unwrap()
        );
        model
            .request(Some(preset.geometry.build().unwrap()))
            .unwrap();
        let parsed = ModelData::parse_legacy_models(
            entry["model_text"].as_str().unwrap(),
            preset.geometry,
            model.theory,
        )
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].embedding().unwrap().shifts,
            model.embedding().unwrap().shifts
        );
    }
    assert!(orbifolder_data::preset("Z5", orbifolder_data::TheoryData::Susy).is_err());
}

#[test]
fn legacy_catalog_parser_rejects_malformed_blocks() {
    let inputs: Value =
        serde_json::from_str(include_str!("../../../tests/python/model_inputs.json")).unwrap();
    let model: ModelData = serde_json::from_value(inputs["susy_z3"]["model"].clone()).unwrap();
    let text = inputs["susy_z3"]["model_text"].as_str().unwrap();
    for bad in [
        String::new(),
        text.replace("end model", ""),
        format!("begin model\n{text}"),
        format!("end model\n{text}"),
    ] {
        assert!(
            ModelData::parse_legacy_models(&bad, model.geometry.clone(), model.theory).is_err()
        );
    }
    let catalog = format!("Label:outside\n{text}\nignore CLI commands\n{text}");
    let parsed = ModelData::parse_legacy_models(&catalog, model.geometry, model.theory).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].label, model.label);
}
