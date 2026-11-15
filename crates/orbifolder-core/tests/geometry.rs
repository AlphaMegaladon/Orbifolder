mod common;
#[path = "common/geometry_input.rs"]
mod geometry_input;
use num_traits::ToPrimitive;
use orbifolder_core::geometry::{
    ElementSpec, Geometry, ModelError, SpaceGroupElement, TheorySpec, embed,
};
use orbifolder_core::math_core::Rational;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn same_element(element: &SpaceGroupElement, native: &Value) {
    let f = native["factors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as u32)
        .collect::<Vec<_>>();
    assert_eq!(element.factors().as_slice(), f);
    assert_eq!(
        element.translation(),
        &common::vector(&native["translation"], 1)
    );
}
fn verify_geometry(case: &str) {
    let data = common::json(&format!("fixtures/{case}.json"));
    let geometry = Geometry::build(geometry_input::from_fixture(&data))
        .unwrap_or_else(|e| panic!("{case}: {e}"));
    let supplement = common::json("geometry_oracle.json");
    let cpp = &supplement["cases"][case]["native"];
    let native = &data["geometry"];
    for (i, twist) in native["twists"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            geometry.space_group().generators()[i].twist,
            common::vector(twist, 12)
        );
    }
    for action in native["rotation_actions"].as_array().unwrap() {
        let factors =
            std::array::from_fn(|i| action["generator"]["factors"][i].as_u64().unwrap() as u32);
        assert_eq!(
            geometry.space_group().rotation(factors).unwrap(),
            &common::matrix(&action["matrix"], 1)
        );
    }
    assert_eq!(
        geometry.space_group().gram(),
        &common::matrix(&native["exact_metric"]["gram_matrix"], 1)
    );
    assert_eq!(
        geometry.sectors().len(),
        native["constructing_elements"].as_array().unwrap().len()
    );
    for (i, order) in geometry.wilson_relations().orders().iter().enumerate() {
        assert_eq!(
            order.as_ref().unwrap().to_string(),
            native["wilson_line_orders"][i].to_string()
        );
    }
    assert_eq!(
        serde_json::to_value(geometry.wilson_relations().aliases()).unwrap(),
        native["wilson_line_relations"]
    );
    let mut flat = 0;
    for (si, sector) in geometry.sectors().iter().enumerate() {
        let expected = &native["constructing_elements"][si];
        assert_eq!(
            sector.fixed_branes().len(),
            expected.as_array().unwrap().len()
        );
        for (bi, brane) in sector.fixed_branes().iter().enumerate() {
            same_element(brane.element(), &expected[bi]);
            let list = &native["centralizers"][flat];
            assert_eq!(
                brane.centralizer_presentation().len(),
                list.as_array().unwrap().len()
            );
            for (j, h) in brane.centralizer_presentation().iter().enumerate() {
                same_element(h, &list[j]);
            }
            let a = geometry.space_group().rotation(sector.factors()).unwrap();
            assert_eq!(
                a.apply_vector(brane.locus().point())
                    .unwrap()
                    .add(brane.element().translation()),
                *brane.locus().point()
            );
            for v in brane.locus().invariant_lattice() {
                assert!(v.coordinates().iter().all(Rational::is_integer));
                assert_eq!(a.apply_vector(v).unwrap(), *v);
            }
            let fp = &cpp["fixed_loci"][flat];
            assert_eq!(fp["exists"], true);
            assert_eq!(
                brane.locus().dimension(),
                2 * fp["fixed_planes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x.as_bool().unwrap())
                    .count()
            );
            for component in 0..6 {
                let expected = fp["cartesian"][component / 2][component % 2]
                    .as_f64()
                    .unwrap();
                let value = (0..6)
                    .map(|i| {
                        let coefficient = &brane.locus().point().coordinates()[i];
                        coefficient.numerator().to_f64().unwrap()
                            / coefficient.denominator().to_f64().unwrap()
                            * data["geometry"]["torus_basis"][i][component / 2][component % 2]
                                .as_f64()
                                .unwrap()
                    })
                    .sum::<f64>();
                let size = brane
                    .locus()
                    .point()
                    .coordinates()
                    .iter()
                    .chain(brane.element().translation().coordinates())
                    .map(|x| {
                        x.numerator().to_f64().unwrap().abs() / x.denominator().to_f64().unwrap()
                    })
                    .sum::<f64>();
                // Only comparison of rounded Cartesian output: Q(sqrt(3)) input
                // error plus floating trig/linear-operation error, never a decision.
                let budget = 16.0 * 4e-11 * (1.0 + size)
                    + 256.0 * f64::EPSILON * (1.0 + size) * (1.0 + expected.abs());
                assert!(
                    (value - expected).abs() <= budget,
                    "{case}: locus {flat}, component {component}: {value} vs {expected}"
                );
            }
            flat += 1;
        }
    }
    let variant = data["variant"].as_str().unwrap();
    let theory = if variant == "susy" {
        TheorySpec::Susy
    } else {
        TheorySpec::NonSusy
    };
    let group = embed(
        geometry,
        geometry_input::embedding(variant, data["input"]["model_text"].as_str().unwrap()),
        theory,
    )
    .unwrap_or_else(|e| panic!("{case}: embed {e}"));
    let expected = &data["orbifold_group"];
    assert_eq!(expected["status"], 2);
    assert_eq!(expected["modular_invariance_status"], 2);
    assert_eq!(group.label(), expected["label"].as_str().unwrap());
    assert_eq!(
        group.supercharges(),
        expected["invariant_supercharges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| common::vector::<4>(v, 2))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        group.supercharges().len(),
        expected["supersymmetry"].as_u64().unwrap() as usize
    );
    assert_eq!(
        group.elements().len(),
        expected["elements"].as_array().unwrap().len()
    );
    for (i, g) in group.elements().iter().enumerate() {
        same_element(g.element(), &expected["elements"][i]["element"]);
        assert_eq!(
            g.shift(),
            &common::vector(&expected["elements"][i]["shift"], 12)
        );
        assert_eq!(
            g.twist(),
            &common::vector(&expected["elements"][i]["twist"], 12)
        );
        for (j, h) in group.centralizers()[i].iter().enumerate() {
            let native = &expected["centralizers"][i][j];
            same_element(h.element(), &native["element"]);
            assert_eq!(h.shift(), &common::vector(&native["shift"], 12));
            assert_eq!(h.twist(), &common::vector(&native["twist"], 12));
        }
    }
    let native_slots: Vec<usize> = if variant == "susy" {
        vec![0, 1, 3, 4, 5, 6, 7, 8]
    } else {
        (0..9).collect()
    };
    let rows = cpp["modular_prefactors"].as_array().unwrap();
    assert_eq!(rows.len(), native_slots.len());
    // The non-SUSY archive allocates nine rows of only eight columns, then
    // writes a 9x9 loop. Compare every actually stored coefficient; never read
    // a fictitious ninth native column or adopt the out-of-bounds behavior.
    for (i, row) in rows.iter().enumerate() {
        for (j, value) in row.as_array().unwrap().iter().enumerate() {
            let a = native_slots[i];
            let b = native_slots[j];
            let check = group
                .modular_checks()
                .iter()
                .find(|c| c.indices() == (a.min(b), a.max(b)))
                .unwrap();
            assert_eq!(check.prefactor().to_string(), value.to_string());
            assert!(check.value().is_even_integer());
        }
    }
    assert!(!group.torsion().enabled());
    assert_eq!(expected["torsion"]["enabled"], false);
    for product in cpp["products"].as_array().unwrap() {
        let parse = |v: &Value| {
            group
                .geometry()
                .space_group()
                .element(ElementSpec {
                    factors: std::array::from_fn(|i| v["factors"][i].as_u64().unwrap() as u32),
                    translation: common::vector(&v["translation"], 1),
                })
                .unwrap()
        };
        let actual = group
            .geometry()
            .space_group()
            .multiply(&parse(&product["left"]), &parse(&product["right"]))
            .unwrap();
        if variant == "susy" {
            same_element(&actual, &product["result"]);
        } else {
            // Fingerprint the retired two-factor helper; it drops m, whereas
            // loaded three-factor model labels and Rust preserve it.
            assert_eq!(product["result"]["factors"][0], 0);
            assert_eq!(
                actual.translation(),
                &common::vector(&product["result"]["translation"], 1)
            );
            for i in 1..3 {
                assert_eq!(
                    actual.factors()[i],
                    product["result"]["factors"][i].as_u64().unwrap() as u32
                );
            }
        }
    }
}
macro_rules! geometry_case {
    ($name:ident) => {
        #[test]
        fn $name() {
            verify_geometry(stringify!($name));
        }
    };
}
geometry_case!(susy_z3);
geometry_case!(susy_z6_ii);
geometry_case!(susy_z3xz3_wl);
geometry_case!(non_susy_z3);
geometry_case!(non_susy_z6_ii);
geometry_case!(non_susy_z3xz3_wl);
geometry_case!(non_susy_tachyonic);

#[test]
fn native_rejected_embeddings() {
    let supplement = common::json("geometry_oracle.json");
    assert_eq!(supplement["negative_cases"].as_object().unwrap().len(), 11);
    for (name, entry) in supplement["negative_cases"].as_object().unwrap() {
        let variant = if name.starts_with("non_susy") {
            "non_susy"
        } else {
            "susy"
        };
        let text = entry["model_text"].as_str().unwrap();
        assert_eq!(entry["native"]["loaded"], false);
        let geometry = Geometry::build(geometry_input::from_text(variant, text, None)).unwrap();
        let result = embed(
            geometry,
            geometry_input::embedding(variant, text),
            if variant == "susy" {
                TheorySpec::Susy
            } else {
                TheorySpec::NonSusy
            },
        );
        let error = result.unwrap_err();
        if name.ends_with("wl_relation") {
            assert_eq!(error, ModelError::WilsonRelation);
        } else if name.ends_with("order") {
            assert_eq!(error, ModelError::InvalidEmbeddingOrder);
        } else {
            assert!(
                matches!(error, ModelError::ModularInvariance { .. }),
                "{name}: {error}"
            );
        }
    }
}

#[test]
fn native_spin32_embeddings() {
    let supplement = common::json("geometry_oracle.json");
    assert_eq!(supplement["positive_cases"].as_object().unwrap().len(), 2);
    for entry in supplement["positive_cases"].as_object().unwrap().values() {
        assert_eq!(entry["native"]["loaded"], true);
        let variant = entry["variant"].as_str().unwrap();
        let text = entry["model_text"].as_str().unwrap();
        let geometry = Geometry::build(geometry_input::from_text(variant, text, None)).unwrap();
        let group = embed(
            geometry,
            geometry_input::embedding(variant, text),
            if variant == "susy" {
                TheorySpec::Susy
            } else {
                TheorySpec::NonSusy
            },
        )
        .unwrap();
        assert_eq!(
            group.lattice(),
            orbifolder_core::math_core::GaugeLattice::D16Plus
        );
    }
}

#[test]
fn wilson_equivalence_is_gauge_lattice_equivalence() {
    let data = common::json("fixtures/susy_z3.json");
    let geometry = Geometry::build(geometry_input::from_fixture(&data)).unwrap();
    let mut input =
        geometry_input::embedding("susy", data["input"]["model_text"].as_str().unwrap());
    input.wilson_lines[1] = orbifolder_core::math_core::Vector::from_integers([
        0, 0, 0, 0, 0, 0, 0, 0, 1, -1, 0, 0, 0, 0, 0, 0,
    ]);
    assert_ne!(input.wilson_lines[0], input.wilson_lines[1]);
    assert!(
        input
            .lattice
            .same_coset(&input.wilson_lines[0], &input.wilson_lines[1])
    );
    assert!(embed(geometry, input, TheorySpec::Susy).is_ok());
}

#[test]
fn explicit_native_wilson_norm_conflict() {
    let supplement = common::json("geometry_oracle.json");
    let entry = &supplement["legacy_conflicts"]["non_susy_wl_norm"];
    assert_eq!(entry["native"]["loaded"], true);
    let text = entry["model_text"].as_str().unwrap();
    let geometry = Geometry::build(geometry_input::from_text("non_susy", text, None)).unwrap();
    let error = embed(
        geometry,
        geometry_input::embedding("non_susy", text),
        TheorySpec::NonSusy,
    )
    .unwrap_err();
    assert_eq!(
        error,
        ModelError::ModularInvariance {
            left: 3,
            right: 3,
            value: Rational::new(2, 3).unwrap()
        }
    );
    assert_eq!(
        supplement["negative_cases"]["susy_wl_norm"]["native"]["loaded"],
        false
    );
}

#[test]
fn geometry_reference_integrity() {
    let supplement = common::json("geometry_oracle.json");
    assert_eq!(supplement["cases"].as_object().unwrap().len(), 7);
    for (case, entry) in supplement["cases"].as_object().unwrap() {
        assert_eq!(
            entry["fixture_sha256"],
            format!(
                "{:x}",
                Sha256::digest(
                    std::fs::read(common::root().join(format!("fixtures/{case}.json"))).unwrap()
                )
            )
        );
    }
    for (field, path) in [
        ("probe_source_sha256", "probes/geometry_probe.cpp"),
        ("build_script_sha256", "build_geometry_probes.sh"),
        ("generator_sha256", "generate_geometry.py"),
    ] {
        assert_eq!(
            supplement["provenance"][field],
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(common::root().join(path)).unwrap())
            )
        );
    }
}
