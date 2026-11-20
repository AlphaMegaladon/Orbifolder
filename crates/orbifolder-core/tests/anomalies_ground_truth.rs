mod common;
#[path = "common/geometry_input.rs"]
mod geometry_input;
use orbifolder_core::{
    anomalies::{AnomalyCheck, AnomalyTraces, Computation, representation_indices},
    geometry::{Geometry, TheorySpec, embed},
    math_core::{GaugeVector, Matrix, Rational},
    physics_engine::{SolverLimits, Spectrum, TachyonSearch, calculate_spectrum, solve_sectors},
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn spectrum(data: &Value) -> Spectrum {
    let variant = data["variant"].as_str().unwrap();
    let geometry = Geometry::build(geometry_input::from_fixture(data)).unwrap();
    let group = embed(
        geometry,
        geometry_input::embedding(variant, data["input"]["model_text"].as_str().unwrap()),
        if variant == "susy" {
            TheorySpec::Susy
        } else {
            TheorySpec::NonSusy
        },
    )
    .unwrap();
    calculate_spectrum(
        &solve_sectors(&group, TachyonSearch::Complete, SolverLimits::default()).unwrap(),
        SolverLimits::default(),
    )
    .unwrap()
}
fn align(s: &Spectrum, data: &Value) -> Vec<(usize, Vec<usize>)> {
    let factors = s.basis().root_system().factors();
    data["gauge_group"]["factors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let roots = f["simple_roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| common::vector::<16>(r, 2))
                .collect::<Vec<_>>();
            let fi = factors
                .iter()
                .position(|f| {
                    f.simple_roots().iter().collect::<BTreeSet<_>>() == roots.iter().collect()
                })
                .unwrap();
            let permutation = roots
                .iter()
                .map(|r| {
                    factors[fi]
                        .simple_roots()
                        .iter()
                        .position(|a| a == r)
                        .unwrap()
                })
                .collect();
            (fi, permutation)
        })
        .collect()
}
fn verify(case: &str) {
    let data = common::json(&format!("fixtures/{case}.json"));
    let supplement = common::json("anomalies_oracle.json");
    let spectrum = spectrum(&data);
    let analysis =
        orbifolder_core::anomalies::analyze_anomalies(&spectrum, Default::default()).unwrap();
    assert!(
        analysis.report().is_anomaly_free(),
        "{case}: failed checks {:?}",
        analysis
            .report()
            .checks()
            .iter()
            .filter(|c| !c.passed())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        analysis.report().fi().legacy_d0(),
        &common::number(&data["anomalies"]["gauge_after_check"]["fi_term"], 1)
    );
    let alignment = align(&spectrum, &data);
    let factors = spectrum.basis().root_system().factors();
    let mut cache = BTreeMap::new();
    for f in supplement["cases"][case]["native"]["fields"]
        .as_array()
        .unwrap()
    {
        let field = &data["spectrum"]["fields"][f["index"].as_u64().unwrap() as usize];
        for (ni, (fi, permutation)) in alignment.iter().enumerate() {
            let mut labels = vec![num_bigint::BigInt::from(0); permutation.len()];
            for (j, &target) in permutation.iter().enumerate() {
                labels[target] = field["highest_weights"][ni][j].as_i64().unwrap().into();
            }
            let key = (*fi, labels);
            if !cache.contains_key(&key) {
                cache.insert(
                    key.clone(),
                    representation_indices(&factors[*fi], &key.1).unwrap(),
                );
            }
            let indices = &cache[&key];
            assert_eq!(
                Rational::from(2) * &indices.quadratic,
                common::number(&f["quadratic"][ni], 1),
                "quadratic {case} field {} factor {ni}",
                f["index"]
            );
            assert_eq!(
                indices.cubic_coefficient(),
                common::number(&f["cubic"][ni], 1),
                "cubic {case} field {} factor {ni}",
                f["index"]
            );
            assert_eq!(
                indices.dimension.to_string(),
                field["representations"][ni]["dimension"]
                    .as_i64()
                    .unwrap()
                    .unsigned_abs()
                    .to_string()
            );
        }
    }
    verify_report(case, &spectrum, &data, &supplement, &alignment);
}

fn transform(old: &[GaugeVector], new: &[GaugeVector]) -> Matrix {
    let matrix = Matrix::new(
        old.len(),
        16,
        old.iter()
            .flat_map(|v| v.coordinates().iter().cloned())
            .collect(),
    )
    .unwrap()
    .transpose();
    Matrix::new(
        new.len(),
        old.len(),
        new.iter()
            .flat_map(|v| matrix.solve(v.coordinates()).unwrap().particular().to_vec())
            .collect(),
    )
    .unwrap()
}
fn cubic(traces: &AnomalyTraces, a: &[Rational], b: &[Rational], c: &[Rational]) -> Rational {
    let mut sum = Rational::ZERO;
    for (i, a) in a.iter().enumerate() {
        for (j, b) in b.iter().enumerate() {
            for (k, c) in c.iter().enumerate() {
                sum = sum + a * b * c * traces.abelian_cubic(i, j, k).unwrap();
            }
        }
    }
    sum
}
fn row(matrix: &Matrix, i: usize) -> Vec<Rational> {
    (0..matrix.cols())
        .map(|j| matrix.get(i, j).unwrap().clone())
        .collect()
}
fn qindex(s: &str) -> usize {
    s.trim_start_matches("Q_")
        .split('^')
        .next()
        .unwrap()
        .parse::<usize>()
        .unwrap()
        - 1
}
fn verify_report(
    case: &str,
    spectrum: &Spectrum,
    data: &Value,
    supplement: &Value,
    alignment: &[(usize, Vec<usize>)],
) {
    let analysis =
        orbifolder_core::anomalies::analyze_anomalies(spectrum, Default::default()).unwrap();
    let report = analysis.report();
    let native = &data["anomalies"]["gauge_after_check"];
    let native_generators = native["u1_generators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| common::vector::<16>(u, 1))
        .collect::<Vec<_>>();
    let b = transform(spectrum.basis().u1_generators(), &native_generators);
    let raw = report.raw();
    let mut duplicates = BTreeMap::new();
    for coefficient in data["anomalies"]["coefficients"].as_array().unwrap() {
        let label = coefficient["label"].as_str().unwrap();
        let value = common::number(&coefficient["value"], 1);
        let actual = if label.contains("-th gauge group factor") {
            let ni = label.split('-').next().unwrap().parse::<usize>().unwrap() - 1;
            let fi = alignment[ni].0;
            if label.contains("doublets") {
                Rational::from(raw.su2_doublets()[fi].as_ref().unwrap().clone())
            } else {
                raw.gauge_cubic()[fi].clone()
            }
        } else if label.starts_with("tr l(rep. of ") {
            let algebra = label
                .split("of ")
                .nth(1)
                .unwrap()
                .split(')')
                .next()
                .unwrap();
            let u = qindex(label.split_whitespace().last().unwrap());
            let key = (algebra.to_owned(), u);
            let occurrence = duplicates.entry(key).or_insert(0usize);
            let ni = data["gauge_group"]["factors"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .filter(|(_, f)| f["algebra"] == algebra)
                .nth(*occurrence)
                .unwrap()
                .0;
            *occurrence += 1;
            let fi = alignment[ni].0;
            Rational::from(2)
                * (0..b.cols()).fold(Rational::ZERO, |s, j| {
                    s + b.get(u, j).unwrap() * &raw.mixed()[j][fi]
                })
        } else {
            let words = label.split_whitespace().collect::<Vec<_>>();
            let i = qindex(words[1]);
            let a = row(&b, i);
            if words.len() == 3 {
                let j = qindex(words[2]);
                let other = row(&b, j);
                cubic(raw, &a, &other, &other)
            } else if words[1].contains('^') {
                cubic(raw, &a, &a, &a)
            } else {
                a.iter()
                    .zip(raw.gravitational())
                    .fold(Rational::ZERO, |s, (a, q)| s + a * q)
            }
        };
        assert_eq!(actual, value, "{case}: {label}");
    }
    let rotation = report.rotation();
    assert_eq!(
        rotation.anomalous_index().is_some(),
        native["first_u1_anomalous"].as_bool().unwrap()
    );
    if rotation.anomalous_index().is_some() {
        assert_eq!(&rotation.generators()[0], &native_generators[0]);
    }
    assert_eq!(rotation.transformation().rank().unwrap(), b.cols());
    for i in 0..rotation.generators().len() {
        if Some(i) != rotation.anomalous_index() {
            assert!(report.canonical().gravitational()[i].is_zero());
        }
        for j in 0..i {
            assert!(
                rotation.generators()[i]
                    .dot(&rotation.generators()[j])
                    .is_zero()
            );
        }
    }
    let old_gram = Matrix::new(
        b.cols(),
        b.cols(),
        spectrum
            .basis()
            .u1_generators()
            .iter()
            .flat_map(|a| {
                spectrum
                    .basis()
                    .u1_generators()
                    .iter()
                    .map(move |b| a.dot(b))
            })
            .collect(),
    )
    .unwrap();
    assert_eq!(
        rotation.gram(),
        &rotation
            .transformation()
            .multiply(&old_gram)
            .unwrap()
            .multiply(&rotation.transformation().transpose())
            .unwrap()
    );
    for (old, new) in spectrum
        .fields()
        .iter()
        .chain(spectrum.tachyonic().fields())
        .zip(
            analysis
                .spectrum()
                .fields()
                .iter()
                .chain(analysis.spectrum().tachyonic().fields()),
        )
    {
        assert_eq!(
            new.u1_charges(),
            rotation.transformation().apply(old.u1_charges()).unwrap()
        );
        assert_eq!(old.source(), new.source());
        assert_eq!(old.mass(), new.mass());
        assert_eq!(old.representations(), new.representations());
        for (i, u) in rotation.generators().iter().enumerate() {
            assert_eq!(new.u1_charges()[i], new.highest_weight().dot(u));
        }
    }
    for polarization in supplement["cases"][case]["native"]["polarizations"]
        .as_array()
        .unwrap()
    {
        let summed = polarization["indices"].as_array().unwrap().iter().fold(
            vec![Rational::ZERO; b.cols()],
            |mut s, i| {
                for (j, x) in s.iter_mut().enumerate() {
                    *x = &*x + b.get(i.as_u64().unwrap() as usize, j).unwrap();
                }
                s
            },
        );
        let value = polarization["native"]["coefficients"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["label"] == "tr Q_1^3")
            .unwrap();
        assert_eq!(
            cubic(raw, &summed, &summed, &summed),
            common::number(&value["value"], 1),
            "native three-index polarization {case}"
        );
    }
    let removed = &supplement["cases"][case]["native"]["removed_chiral"];
    if !removed.is_null() {
        let native_index = removed["field_index"].as_u64().unwrap() as usize;
        let native_field = &data["spectrum"]["fields"][native_index];
        let source =
            std::array::from_fn(|i| native_field["source_index"][i].as_u64().unwrap() as usize);
        let weights = native_field["weight_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        let si = &data["sectors"][source[0]]["fixed_branes"][source[1]]["states"][source[2]];
        let position = si["field_indices"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v.as_u64().unwrap() as usize == native_index)
            .unwrap();
        let charges = &si["q_charges"][position % si["q_charges"].as_array().unwrap().len()];
        let right = charges
            .as_array()
            .unwrap()
            .iter()
            .map(|v| common::vector(v, 12))
            .collect::<Vec<_>>();
        let target = spectrum
            .fields()
            .iter()
            .position(|f| {
                f.source() == source && f.weight_indices() == weights && f.right_weights() == right
            })
            .unwrap();
        let mut i = 0;
        let reduced = spectrum.select_fields(|_| {
            let keep = i != target;
            i += 1;
            keep
        });
        let bad =
            orbifolder_core::anomalies::analyze_anomalies(&reduced, Default::default()).unwrap();
        assert!(!bad.report().is_anomaly_free());
        assert!(bad.report().has_failed_checks());
        assert!(bad.report().checks().iter().any(|c| matches!(
            c.kind(),
            AnomalyCheck::GaugeCubic { .. }
        ) && matches!(
            c.status(),
            Computation::Computed(false)
        )));
    }
    eprintln!(
        "PASS {case}: exact indices, all stored traces, {} native polarizations, canonical FI={}",
        supplement["cases"][case]["native"]["polarizations"]
            .as_array()
            .unwrap()
            .len(),
        report.fi().legacy_d0()
    );
}
#[test]
fn susy_z3() {
    verify("susy_z3");
}
#[test]
fn susy_z6_ii() {
    verify("susy_z6_ii");
}
#[test]
fn susy_z3xz3_wl() {
    verify("susy_z3xz3_wl");
}
#[test]
fn non_susy_z3() {
    verify("non_susy_z3");
}
#[test]
fn non_susy_z6_ii() {
    verify("non_susy_z6_ii");
}
#[test]
fn non_susy_z3xz3_wl() {
    verify("non_susy_z3xz3_wl");
}
#[test]
fn non_susy_tachyonic() {
    verify("non_susy_tachyonic");
}

#[test]
fn supplemental_oracle_and_seven_fixtures_are_integrity_bound() {
    use sha2::{Digest, Sha256};
    let data = common::json("anomalies_oracle.json");
    assert_eq!(data["schema_version"], 1);
    assert_eq!(data["provenance"]["independent_processes"], 2);
    assert_eq!(data["cases"].as_object().unwrap().len(), 7);
    for (key, path) in [
        ("probe_source_sha256", "probes/anomalies_probe.cpp"),
        ("build_script_sha256", "build_anomalies_probes.sh"),
        ("generator_sha256", "generate_anomalies.py"),
    ] {
        assert_eq!(
            data["provenance"][key],
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(common::root().join(path)).unwrap())
            )
        );
    }
    for (case, entry) in data["cases"].as_object().unwrap() {
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
}
