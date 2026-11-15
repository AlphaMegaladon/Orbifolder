mod common;
#[path = "common/geometry_input.rs"]
mod geometry_input;
use orbifolder_core::{
    anomalies::Computation,
    geometry::{Geometry, TheorySpec, embed},
    math_core::{GaugeVector, Rational},
    physics_engine::SolverLimits,
    susy::{
        AnalysisLimits, DiscreteRule, Monomial, RuleCharge, SelectionRules, SusySpectrum,
        analyze_couplings, enumerate_superpotential, legacy_fi_charge_test,
    },
};
use serde_json::Value;
use std::collections::BTreeMap;

fn rules(data: &Value) -> SelectionRules {
    let text = geometry_input::geometry_text(
        "susy",
        data["input"]["model_text"].as_str().unwrap(),
        Some(data["input"]["geometry_sha256"].as_str().unwrap()),
    );
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with("//"))
        .collect::<Vec<_>>();
    let mut out = SelectionRules::default();
    let start = lines
        .iter()
        .position(|&s| s == "begin discrete symmetries")
        .unwrap()
        + 1;
    let mut i = start;
    while lines[i] != "end discrete symmetries" {
        let r = lines[i] == "new R symmetry";
        assert!(r || lines[i] == "new non-R symmetry");
        let order = lines[i + 1].parse().unwrap();
        let target = lines[i + 2].parse().unwrap();
        let op = lines[i + 3]
            .split_whitespace()
            .map(|s| s.parse::<Rational>().unwrap())
            .collect::<Vec<_>>();
        let charge = if r {
            RuleCharge::R(Box::new(op.try_into().unwrap()))
        } else {
            assert_eq!(op.len(), 8);
            let mut full = vec![op[0].clone(), op[1].clone(), Rational::ZERO];
            full.extend_from_slice(&op[2..]);
            RuleCharge::NonR(Box::new(full.try_into().unwrap()))
        };
        out.discrete
            .push(DiscreteRule::new(order, target, charge).unwrap());
        i += if r { 4 } else { 5 };
    }
    out
}
fn model(data: &Value) -> orbifolder_core::geometry::OrbifoldGroup {
    let variant = data["variant"].as_str().unwrap();
    embed(
        Geometry::build(geometry_input::from_fixture(data)).unwrap(),
        geometry_input::embedding(variant, data["input"]["model_text"].as_str().unwrap()),
        if variant == "susy" {
            TheorySpec::Susy
        } else {
            TheorySpec::NonSusy
        },
    )
    .unwrap()
}
fn map_fields(view: &SusySpectrum, data: &Value) -> BTreeMap<usize, usize> {
    data["spectrum"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["multiplet"] == 2)
        .map(|f| {
            let source = f["source_index"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<_>>();
            let weights = f["weight_indices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as usize)
                .collect::<Vec<_>>();
            let q = common::vector::<4>(&f["q_sh"], 12);
            let matches = view
                .spectrum()
                .fields()
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    r.source().as_slice() == source
                        && r.weight_indices() == weights
                        && r.scalar_charge() == Some(&q)
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "field mapping {}", f["index"]);
            (f["index"].as_u64().unwrap() as usize, matches[0])
        })
        .collect()
}
fn verify(case: &str) {
    let data = common::json(&format!("fixtures/{case}.json"));
    let supplementary = common::json("couplings_oracle.json");
    let Computation::Computed(view) =
        SusySpectrum::compute(&model(&data), SolverLimits::default()).unwrap()
    else {
        panic!("missing SUSY view")
    };
    let map = map_fields(&view, &data);
    let rules = rules(&data);
    let limits = AnalysisLimits::default();
    let native = &supplementary["cases"][case]["native"]["couplings"];
    for (r, n) in rules
        .discrete
        .iter()
        .zip(native["original_non_r"].as_array().unwrap())
    {
        assert_eq!(r.order() as u64, n["order"].as_u64().unwrap());
        let RuleCharge::NonR(op) = r.operator() else {
            panic!()
        };
        let mut old = vec![op[0].clone(), op[1].clone()];
        old.extend_from_slice(&op[3..]);
        assert_eq!(
            old,
            n["operator"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| common::number(x, 1))
                .collect::<Vec<_>>()
        );
    }
    for (&old, &new) in &map {
        let field = &view.spectrum().fields()[new];
        let expected = &data["spectrum"]["fields"][old];
        for (i, r) in rules.discrete.iter().enumerate() {
            assert_eq!(
                r.field_charge(field).unwrap(),
                common::number(&expected["discrete_non_r_charges"][i], 6)
            );
        }
    }
    let trials = native["trials"].as_array().unwrap();
    let monomials = trials
        .iter()
        .map(|t| {
            Monomial::new(
                &view,
                &t["fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| map[&(id.as_u64().unwrap() as usize)])
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let original = analyze_couplings(&view, &monomials, &rules, limits).unwrap();
    let mut explicit = rules.clone();
    let r_orders = if case == "susy_z6_ii" {
        [6, 3, 2]
    } else {
        [3, 3, 3]
    };
    explicit
        .discrete
        .extend(SelectionRules::internal_r(r_orders).unwrap());
    let with_r = analyze_couplings(&view, &monomials, &explicit, limits).unwrap();
    let native_generators = data["gauge_group"]["u1_generators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| common::vector::<16>(g, 1))
        .collect::<Vec<_>>();
    let native_fi = common::number(&data["gauge_group"]["fi_term"], 1);
    let mut independent_r = 0;
    let mut independent_space = 0;
    let mut independent_gauge = 0;
    for (ti, t) in trials.iter().enumerate() {
        let report = &original[ti];
        let r = &with_r[ti];
        assert_eq!(
            !report.invariant().tensor_singlets().is_zero(),
            t["non_abelian"].as_bool().unwrap(),
            "{case} tensor {ti} {:?}",
            t["fields"]
        );
        let sum = monomials[ti]
            .expanded()
            .iter()
            .fold(GaugeVector::zero(), |s, &i| {
                s.add(view.spectrum().fields()[i].highest_weight())
            });
        let native_charges = native_generators
            .iter()
            .map(|g| g.dot(&sum))
            .collect::<Vec<_>>();
        assert_eq!(
            native_charges,
            t["charge_sum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|q| common::number(q, 12))
                .collect::<Vec<_>>(),
            "{case} charges {ti}"
        );
        assert_eq!(
            legacy_fi_charge_test(&native_charges, &native_fi, true)
                && !report.invariant().tensor_singlets().is_zero(),
            t["monomial_allowed"].as_bool().unwrap()
        );
        let actual_exponents = monomials[ti]
            .fields()
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>();
        let expected_exponents = t["monomial_fields"]
            .as_array()
            .unwrap()
            .iter()
            .zip(t["exponents"].as_array().unwrap())
            .map(|(ids, n)| {
                assert_eq!(ids.as_array().unwrap().len(), 1);
                (
                    map[&(ids[0].as_u64().unwrap() as usize)],
                    n.as_u64().unwrap() as u32,
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(actual_exponents, expected_exponents);
        assert_eq!(
            report.native_precoupling(),
            t["original_pre"].as_bool().unwrap(),
            "{case} pre {ti}"
        );
        assert_eq!(
            u64::from(report.native_allowed()),
            t["original_add"].as_u64().unwrap(),
            "{case} add {ti} {:?} sg {}, fixed {}",
            t["fields"],
            report.space_group(),
            report.native_fixed_torus()
        );
        assert_eq!(
            r.native_precoupling(),
            t["explicit_r_pre"].as_bool().unwrap(),
            "{case} r pre {ti}"
        );
        assert_eq!(
            u64::from(r.native_allowed()),
            t["explicit_r_add"].as_u64().unwrap(),
            "{case} r add {ti}"
        );
        let space_only = monomials[ti].degree() >= 3
            && report.point_group()
            && report.space_group()
            && report.native_fixed_torus();
        assert_eq!(
            u64::from(space_only),
            t["space_only_add"].as_u64().unwrap(),
            "{case} space only {ti} {:?}",
            t["fields"]
        );
        for (i, n) in t["r_charges"].as_array().unwrap().iter().enumerate() {
            let rule = &explicit.discrete[rules.discrete.len() + i];
            let charges = t["fields"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| {
                    rule.field_charge(
                        &view.spectrum().fields()[map[&(id.as_u64().unwrap() as usize)]],
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            assert_eq!(
                charges,
                n.as_array()
                    .unwrap()
                    .iter()
                    .map(|q| common::number(q, 6))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                &charges.iter().fold(Rational::ZERO, |s, q| s + q),
                r.discrete()[rules.discrete.len() + i].charge_sum()
            );
        }
        if report.native_allowed() && !r.native_allowed() {
            independent_r += 1;
        }
        if report.invariant().native_gauge_invariant()
            && report.point_group()
            && !report.space_group()
        {
            independent_space += 1;
        }
        if !r.invariant().native_gauge_invariant()
            && r.point_group()
            && r.space_group()
            && r.native_fixed_torus()
            && r.discrete().iter().all(|c| c.passed())
        {
            independent_gauge += 1;
        }
        assert!(matches!(report.gamma(), Computation::NotApplicable(_)));
        if ti < 56 {
            let core = &data["susy_analysis"]["trials"][ti];
            assert_eq!(t["fields"], core["field_indices"]);
            assert_eq!(
                u64::from(report.native_allowed()),
                core["allowed_couplings"].as_u64().unwrap()
            );
            assert_eq!(
                legacy_fi_charge_test(&native_charges, &native_fi, true)
                    && !report.invariant().tensor_singlets().is_zero(),
                core["gauge_invariant"].as_bool().unwrap()
            );
            assert_eq!(t["exponents"], core["monomial_exponents"]);
            assert_eq!(t["monomial_fields"], core["monomial_fields"]);
        }
    }
    for t in native["fi_charge_tests"].as_array().unwrap() {
        let q = common::number(&t["charge"], 1);
        let fi = common::number(&t["fi"], 1);
        assert_eq!(
            legacy_fi_charge_test(std::slice::from_ref(&q), &fi, true),
            t["demand"].as_bool().unwrap()
        );
        assert_eq!(
            legacy_fi_charge_test(&[q], &fi, false),
            t["relaxed"].as_bool().unwrap()
        );
    }
    let pool = native["field_pool"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| map[&(id.as_u64().unwrap() as usize)])
        .collect::<Vec<_>>();
    let w = enumerate_superpotential(&view, &pool, 3, &rules, limits).unwrap();
    assert_eq!(w.candidates().len(), 83);
    if case == "susy_z3" {
        assert!(independent_r > 0);
        assert!(independent_space > 0);
        let repeat = Monomial::new(&view, &[map[&3]; 3]).unwrap();
        let report = orbifolder_core::susy::analyze_monomials(&view, &[repeat], limits)
            .unwrap()
            .remove(0);
        assert_eq!(report.tensor_singlets().to_string(), "1");
        assert_eq!(report.polynomial_singlets().to_string(), "0");
        let a = Monomial::new(&view, &[map[&3]]).unwrap();
        let b = Monomial::new(&view, &[map[&4], map[&5]]).unwrap();
        assert_eq!(
            a.multiply(&b, &view).unwrap(),
            Monomial::new(&view, &[map[&3], map[&4], map[&5]]).unwrap()
        );
        assert!(matches!(
            a.multiply(&Monomial::new(&view, &[map[&3]; 3]).unwrap(), &view),
            Err(orbifolder_core::susy::SusyError::UnsupportedOrder)
        ));
        assert!(matches!(
            Monomial::new(&view, &[]),
            Err(orbifolder_core::susy::SusyError::InvalidMonomial)
        ));
        assert!(matches!(
            Monomial::new(&view, &[usize::MAX]),
            Err(orbifolder_core::susy::SusyError::InvalidField)
        ));
        let vector = view
            .spectrum()
            .fields()
            .iter()
            .position(|f| {
                f.multiplet()
                    != orbifolder_core::physics_engine::Multiplet::Susy(
                        orbifolder_core::physics_engine::SusyMultiplet::LeftChiral,
                    )
            })
            .unwrap();
        assert!(matches!(
            Monomial::new(&view, &[vector]),
            Err(orbifolder_core::susy::SusyError::InvalidField)
        ));
        assert!(enumerate_superpotential(&view, &pool, 4, &rules, limits).is_err());
        assert!(enumerate_superpotential(&view, &[pool[0], pool[0]], 3, &rules, limits).is_err());
        assert!(
            enumerate_superpotential(
                &view,
                &pool,
                3,
                &rules,
                AnalysisLimits {
                    candidates: 1,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            analyze_couplings(
                &view,
                &monomials,
                &rules,
                AnalysisLimits {
                    character_weights: 1,
                    ..limits
                }
            )
            .is_err()
        );
        for term in w.f_terms(map[&3]) {
            let m = &w.candidates()[term.coefficient_label()]
                .invariant()
                .monomial();
            assert_eq!(
                term.prefactor(),
                m.fields().iter().find(|&&(i, _)| i == map[&3]).unwrap().1
            );
            assert_eq!(
                term.powers().iter().map(|(_, n)| n).sum::<u32>(),
                m.degree() - 1
            );
        }
        assert!(!w.f_terms(map[&3]).is_empty());
        assert!(w.f_terms(usize::MAX).is_empty());
    }
    if case == "susy_z3xz3_wl" {
        let pair = original.last().unwrap();
        assert_eq!(pair.invariant().monomial().degree(), 2);
        assert_eq!(pair.invariant().tensor_singlets().to_string(), "1");
        assert_eq!(pair.invariant().polynomial_singlets().to_string(), "1");
        assert!(
            !pair.invariant().gauge_invariant(),
            "The simultaneous epsilon contractions are symmetric, but U(1) charge remains nonzero"
        );
    }
    if case == "susy_z6_ii" {
        assert!(independent_gauge > 0);
    }
    eprintln!(
        "PASS {case}: {} native trials, original positives {}, R-only failures {independent_r}, SG-only failures {independent_space}, gauge-only failures {independent_gauge}",
        trials.len(),
        original[..56].iter().filter(|r| r.native_allowed()).count()
    );
}
#[test]
fn coupling_provenance_and_fixture_integrity() {
    use sha2::{Digest, Sha256};
    let data = common::json("couplings_oracle.json");
    assert_eq!(data["schema_version"], 1);
    assert_eq!(data["provenance"]["independent_processes"], 2);
    assert_eq!(data["cases"].as_object().unwrap().len(), 3);
    let hash = |path: &str| {
        format!(
            "{:x}",
            Sha256::digest(std::fs::read(common::root().join(path)).unwrap())
        )
    };
    for probe in ["couplings", "characters"] {
        assert_eq!(
            data["provenance"][probe]["source_sha256"],
            hash(&format!("probes/{probe}_probe.cpp"))
        );
    }
    assert_eq!(
        data["provenance"]["build_script_sha256"],
        hash("build_couplings_probe.sh")
    );
    assert_eq!(
        data["provenance"]["generator_sha256"],
        hash("generate_couplings.py")
    );
    for (case, entry) in data["cases"].as_object().unwrap() {
        assert_eq!(
            entry["fixture_sha256"],
            hash(&format!("fixtures/{case}.json"))
        );
    }
}
use num_traits::Zero;
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
fn all_four_non_susy_not_applicable() {
    for case in [
        "non_susy_z3",
        "non_susy_z6_ii",
        "non_susy_z3xz3_wl",
        "non_susy_tachyonic",
    ] {
        let data = common::json(&format!("fixtures/{case}.json"));
        assert_eq!(data["susy_analysis"]["status"], "not_applicable");
        assert!(matches!(
            SusySpectrum::compute(&model(&data), SolverLimits::default()).unwrap(),
            Computation::NotApplicable(_)
        ));
    }
}
