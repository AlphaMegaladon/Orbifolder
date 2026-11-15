mod common;
#[path = "common/geometry_input.rs"]
mod geometry_input;
use common::{json, number, vector};
use orbifolder_core::{
    geometry::{Geometry, TheorySpec, embed},
    math_core::{Matrix, Rational},
    physics_engine::{
        Field, HalfState, MassShell, Multiplet, Oscillator, Particle, SolverLimits, SusyMultiplet,
        TachyonSearch, calculate_spectrum, solve_sectors,
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

fn code(kind: Multiplet) -> u64 {
    match kind {
        Multiplet::Susy(m) => match m {
            SusyMultiplet::LeftChiral => 2,
            SusyMultiplet::RightChiral => 3,
            SusyMultiplet::Vector => 4,
            SusyMultiplet::ConjugateVector => 5,
            SusyMultiplet::HalfHyper => 7,
            SusyMultiplet::Gravity => 8,
            SusyMultiplet::ConjugateGravity => 9,
            SusyMultiplet::LeftModulus => 10,
            SusyMultiplet::RightModulus => 11,
        },
        Multiplet::Particle(m) => match m {
            Particle::Gauge => 13,
            Particle::ConjugateGauge => 14,
            Particle::Scalar => 15,
            Particle::ConjugateScalar => 16,
            Particle::LeftFermion => 19,
            Particle::RightFermion => 20,
            Particle::Gravity => 8,
            Particle::ConjugateGravity => 9,
            Particle::Modulus => 17,
            Particle::ConjugateModulus => 18,
        },
        Multiplet::Tachyon { excited, conjugate } => {
            21 + u64::from(conjugate) + 2 * u64::from(excited)
        }
    }
}
fn verify_fields(solved: &orbifolder_core::physics_engine::SolvedSectors, data: &Value) {
    let spectrum = calculate_spectrum(solved, SolverLimits::default()).unwrap();
    let native_factors = data["gauge_group"]["factors"].as_array().unwrap();
    let factors = spectrum.basis().root_system().factors();
    assert_eq!(native_factors.len(), factors.len());
    let alignment = native_factors
        .iter()
        .map(|f| {
            let native_roots = f["simple_roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| vector::<16>(v, 2))
                .collect::<Vec<_>>();
            let set = native_roots.iter().collect::<BTreeSet<_>>();
            let fi = factors
                .iter()
                .position(|f| f.simple_roots().iter().collect::<BTreeSet<_>>() == set)
                .unwrap();
            assert_eq!(
                factors[fi].algebra().to_string(),
                f["algebra"].as_str().unwrap()
            );
            let permutation = native_roots
                .iter()
                .map(|r| {
                    factors[fi]
                        .simple_roots()
                        .iter()
                        .position(|a| a == r)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            (fi, permutation)
        })
        .collect::<Vec<_>>();
    let native_u1 = data["gauge_group"]["u1_generators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| vector::<16>(v, 1))
        .collect::<Vec<_>>();
    let native_matrix = Matrix::new(
        native_u1.len(),
        16,
        native_u1
            .iter()
            .flat_map(|v| v.coordinates().iter().cloned())
            .collect(),
    )
    .unwrap();
    let change = spectrum
        .basis()
        .u1_generators()
        .iter()
        .map(|u| {
            native_matrix
                .transpose()
                .solve(u.coordinates())
                .unwrap()
                .particular()
                .to_vec()
        })
        .collect::<Vec<_>>();
    type FieldKey = ([usize; 3], Vec<usize>, u64);
    let mut matched: BTreeMap<FieldKey, Vec<&Field>> = BTreeMap::new();
    for field in spectrum
        .fields()
        .iter()
        .chain(spectrum.tachyonic().fields())
    {
        matched
            .entry((
                field.source(),
                field.weight_indices().to_vec(),
                code(field.multiplet()),
            ))
            .or_default()
            .push(field);
    }
    let all = data["spectrum"]["fields"].as_array().unwrap();
    assert_eq!(
        spectrum.fields().len() + spectrum.tachyonic().fields().len(),
        all.len(),
        "all field objects"
    );
    assert_eq!(
        spectrum.tachyonic().fields().len(),
        data["tachyonic_spectrum"]["fields"]
            .as_array()
            .unwrap()
            .len()
    );
    for f in all {
        let source = std::array::from_fn(|i| f["source_index"][i].as_u64().unwrap() as usize);
        let indices = f["weight_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        let native_code = f["multiplet"].as_u64().unwrap();
        let is_tachyon = native_code >= 21;
        let b = &data["sectors"][source[0]]["fixed_branes"][source[1]];
        let native_state = if is_tachyon {
            let ground = b["tachyonic_states"].as_array().unwrap().len();
            if source[2] < ground {
                &b["tachyonic_states"][source[2]]
            } else {
                &b["excited_tachyonic_states"][source[2] - ground]
            }
        } else {
            &b["states"][source[2]]
        };
        let field_index = f["index"].as_u64().unwrap() as usize
            - if is_tachyon {
                spectrum.fields().len()
            } else {
                0
            };
        let position = native_state["field_indices"]
            .as_array()
            .unwrap()
            .iter()
            .position(|v| v.as_u64().unwrap() as usize == field_index)
            .unwrap();
        let qgroups = native_state["q_charges"].as_array().unwrap();
        let right_weights = qgroups[position % qgroups.len()]
            .as_array()
            .unwrap()
            .iter()
            .map(|q| vector(q, 12))
            .collect::<Vec<_>>();
        // --tachyon-physics: derive excitation from actual oscillators, never Excited.
        let native_code = if native_code >= 21 {
            let s = &solved.sectors()[source[0]];
            let state = &s.branes()[source[1]].tachyons()[source[2]];
            21 + (native_code - 21) % 2
                + 2 * u64::from(
                    s.tachyon_shells()[state.right().shell()]
                        .excitation()
                        .is_excited(),
                )
        } else {
            native_code
        };
        let a = matched
            .get_mut(&(source, indices, native_code))
            .and_then(|list| {
                let i = list
                    .iter()
                    .position(|a| a.right_weights() == right_weights)?;
                Some(list.remove(i))
            })
            .unwrap_or_else(|| {
                panic!(
                    "missing field {} source {:?}, multiplet {native_code}",
                    f["index"], source
                )
            });
        assert_eq!(
            a.element().factors().as_slice(),
            f["element"]["factors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            a.element().translation(),
            &vector(&f["element"]["translation"], 1)
        );
        assert_eq!(
            a.gamma(),
            f["gamma_phases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| number(v, 72))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            a.oscillator_contribution(),
            &vector(&f["oscillator_contribution"], 1)
        );
        let vacuum = solved.sectors()[source[0]].modes().vacuum();
        let half_norm = Rational::new(1, 2).unwrap();
        assert_eq!(
            &half_norm * a.highest_weight().norm_squared()
                + a.left_excitation().energy()
                + vacuum.left(),
            *a.mass()
        );
        for q in a.right_weights() {
            assert_eq!(
                &half_norm * q.norm_squared() + a.right_excitation().energy() + vacuum.right(),
                *a.mass()
            );
        }
        assert_eq!(is_tachyon, a.mass() < &Rational::ZERO);
        assert!(
            a.gamma()
                .iter()
                .all(|p| p >= &Rational::ZERO && p < &Rational::ONE)
        );
        if let Multiplet::Tachyon { excited, .. } = a.multiplet() {
            assert_eq!(
                excited,
                a.left_excitation().is_excited() || a.right_excitation().is_excited()
            );
        }
        if !f["q_sh"].is_null() {
            assert_eq!(a.scalar_charge(), Some(&vector(&f["q_sh"], 12)));
        }
        for (ni, (fi, permutation)) in alignment.iter().enumerate() {
            let r = &a.representations()[*fi];
            assert_eq!(
                r.dimension().to_string(),
                f["representations"][ni]["dimension"]
                    .as_i64()
                    .unwrap()
                    .unsigned_abs()
                    .to_string()
            );
            assert_eq!(
                permutation
                    .iter()
                    .map(|&i| r.dynkin_labels()[i].to_string())
                    .collect::<Vec<_>>(),
                f["highest_weights"][ni]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            );
        }
        let charges = f["u1_charges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| number(v, 12))
            .collect::<Vec<_>>();
        for (i, u) in native_u1.iter().enumerate() {
            assert_eq!(a.highest_weight().dot(u), charges[i]);
        }
        for (i, row) in change.iter().enumerate() {
            assert_eq!(
                a.u1_charges()[i],
                row.iter()
                    .zip(&charges)
                    .fold(Rational::ZERO, |s, (a, b)| s + a * b)
            );
        }
    }
    assert!(matched.values().all(Vec::is_empty));
    let tail = &all[spectrum.fields().len()..];
    for (appended, separate) in tail
        .iter()
        .zip(data["tachyonic_spectrum"]["fields"].as_array().unwrap())
    {
        let mut a = appended.clone();
        let mut b = separate.clone();
        a.as_object_mut().unwrap().remove("index");
        b.as_object_mut().unwrap().remove("index");
        assert_eq!(
            a, b,
            "native tachyon container must duplicate the appended physical field"
        );
    }
    eprintln!(
        "PASS {}: {} massless + {} tachyon fields",
        data["case_id"],
        spectrum.fields().len(),
        spectrum.tachyonic().fields().len()
    );
    // Basis transforms above compare results only: none enters solve_sectors or
    // calculate_spectrum. The default core never computes anomalous generators.
}

fn modes(actual: &[Oscillator], expected: &Value) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!(a.direction(), e["direction"].as_u64().unwrap() as usize);
        assert_eq!(a.conjugate(), e["complex_conjugate"].as_bool().unwrap());
        assert_eq!(a.frequency(), &number(&e["frequency"], 36));
        assert_eq!(a.energy(), number(&e["number_operator"], 36));
    }
}
fn shells<const N: usize>(actual: &[MassShell<N>], expected: &Value) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "shell count");
    for (a, e) in actual.iter().zip(expected) {
        let ex = &e["excitation"];
        assert_eq!(a.excitation().energy(), &number(&ex["number_operator"], 36));
        assert_eq!(
            a.excitation().target(),
            &number(&ex["squared_weight_target"], 36)
        );
        let indices = ex["oscillators"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        assert_eq!(a.excitation().indices(), indices);
        let weights = e["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| vector::<N>(w, 12))
            .collect::<Vec<_>>();
        assert_eq!(a.weights(), weights, "mass shell weights");
    }
}
fn half(a: &HalfState, e: &Value, right: bool) {
    assert_eq!(a.shell(), e["solution_index"].as_u64().unwrap() as usize);
    assert_eq!(
        a.indices(),
        e["weight_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap() as usize)
            .collect::<Vec<_>>()
    );
    let expected = e["eigenphases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| number(v, 72))
        .collect::<Vec<_>>();
    assert_eq!(&a.phases()[..expected.len()], expected);
    if right {
        assert!(a.phases()[expected.len()..].iter().all(Rational::is_zero));
    } else {
        assert_eq!(a.phases().len(), expected.len());
    }
}
fn verify(case: &str) {
    let data = json(&format!("fixtures/{case}.json"));
    let geometry = Geometry::build(geometry_input::from_fixture(&data)).unwrap();
    let variant = data["variant"].as_str().unwrap();
    let theory = if variant == "susy" {
        TheorySpec::Susy
    } else {
        TheorySpec::NonSusy
    };
    let model = embed(
        geometry,
        geometry_input::embedding(variant, data["input"]["model_text"].as_str().unwrap()),
        theory,
    )
    .unwrap();
    let solved =
        solve_sectors(&model, TachyonSearch::LegacyWindow, SolverLimits::default()).unwrap();
    let complete = solve_sectors(&model, TachyonSearch::Complete, SolverLimits::default()).unwrap();
    assert_eq!(
        solved.sectors(),
        complete.sectors(),
        "complete norm bounds must reproduce the native window for these inputs"
    );
    assert_eq!(
        solved.sectors().len(),
        data["sectors"].as_array().unwrap().len()
    );
    for (si, (a, e)) in solved
        .sectors()
        .iter()
        .zip(data["sectors"].as_array().unwrap())
        .enumerate()
    {
        let _ = si;
        assert_eq!(a.twist(), &vector(&e["twist"], 12));
        assert_eq!(a.modes().vacuum().left(), &number(&e["a_left"], 72));
        assert_eq!(a.modes().vacuum().right(), &number(&e["a_right"], 72));
        modes(a.modes().left(), &e["left_oscillators"]);
        modes(a.modes().right(), &e["right_oscillators"]);
        shells(a.right_shells(), &e["right_solutions"]);
        assert_eq!(
            a.right_movers().len(),
            e["right_movers"].as_array().unwrap().len()
        );
        for (r, n) in a
            .right_movers()
            .iter()
            .zip(e["right_movers"].as_array().unwrap())
        {
            half(r, n, true);
        }
        assert_eq!(
            a.tachyon_shells().len(),
            e["tachyonic_right_solutions"].as_array().unwrap().len()
        );
        for (shell, native) in a
            .tachyon_shells()
            .iter()
            .zip(e["tachyonic_right_solutions"].as_array().unwrap())
        {
            shells(
                std::slice::from_ref(shell),
                &serde_json::json!([native["solution"]]),
            );
        }
        assert_eq!(
            a.tachyon_movers().len(),
            e["tachyonic_right_movers"].as_array().unwrap().len()
        );
        for (r, n) in a
            .tachyon_movers()
            .iter()
            .zip(e["tachyonic_right_movers"].as_array().unwrap())
        {
            half(r, n, true);
        }
        for (i, n) in e["tachyonic_right_solutions"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for mass in n["masses"].as_array().unwrap() {
                assert_eq!(a.tachyon_masses()[i], number(mass, 72));
            }
        }
        for (bi, (b, n)) in a
            .branes()
            .iter()
            .zip(e["fixed_branes"].as_array().unwrap())
            .enumerate()
        {
            let _ = bi;
            shells(b.left_shells(), &n["left_solutions"]);
            assert_eq!(
                b.left_movers().len(),
                n["left_movers"].as_array().unwrap().len()
            );
            for (l, x) in b
                .left_movers()
                .iter()
                .zip(n["left_movers"].as_array().unwrap())
            {
                half(l, x, false);
            }
            let states = n["states"].as_array().unwrap();
            assert_eq!(b.states().len(), states.len(), "projected states");
            for (s, x) in b.states().iter().zip(states) {
                half(s.left(), &x["left"], false);
                half(s.right(), &x["right"], true);
                assert_eq!(
                    s.oscillator_contribution(),
                    &vector(&x["oscillator_contribution"], 1)
                );
                assert_eq!(
                    s.gamma(),
                    x["gamma_phases"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| number(v, 72))
                        .collect::<Vec<_>>()
                );
            }
            assert_eq!(
                b.tachyons().len(),
                n["tachyonic_states"].as_array().unwrap().len()
                    + n["excited_tachyonic_states"].as_array().unwrap().len()
            );
            for (s, x) in b.tachyons().iter().zip(
                n["tachyonic_states"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .chain(n["excited_tachyonic_states"].as_array().unwrap()),
            ) {
                half(s.left(), &x["left"], false);
                half(s.right(), &x["right"], true);
                let left = &b.tachyon_shells()[s.right().shell()][s.left().shell()];
                shells(
                    std::slice::from_ref(left),
                    &serde_json::json!([x["left_solution"]]),
                );
                assert_eq!(
                    s.oscillator_contribution(),
                    &vector(&x["oscillator_contribution"], 1)
                );
                assert_eq!(
                    s.gamma(),
                    x["gamma_phases"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| number(v, 72))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    verify_fields(&solved, &data);
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
fn excited_tachyon_kinematics_and_finite_window_counterexample() {
    use orbifolder_core::{math_core::RightWeight, physics_engine::solve_kinematics};
    let data = json("states_oracle.json");
    assert_eq!(data["provenance"]["independent_processes"], 2);
    for (field, path) in [
        ("probe_source_sha256", "probes/states_probe.cpp"),
        ("build_script_sha256", "build_states_probe.sh"),
        ("generator_sha256", "generate_states.py"),
    ] {
        assert_eq!(
            data["provenance"][field],
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(common::root().join(path)).unwrap())
            )
        );
    }
    let mut baseline = None;
    for (i, e) in data["native"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        // Rational inputs are specified independently of observed C++ outputs.
        let v0 = RightWeight::from_integers([0, if i == 0 { 1 } else { 25 }, 1, 1]);
        let v1 = RightWeight::new([
            Rational::ZERO,
            Rational::new(1, 6).unwrap(),
            Rational::new(-1, 6).unwrap(),
            Rational::ZERO,
        ]);
        let twist = v0.add(&v1);
        let generators = [v0, v1, RightWeight::zero()];
        let legacy = solve_kinematics(
            [1, 1, 0],
            twist.clone(),
            &generators,
            TheorySpec::NonSusy,
            TachyonSearch::LegacyWindow,
            SolverLimits::default(),
        )
        .unwrap();
        let complete = solve_kinematics(
            [1, 1, 0],
            twist,
            &generators,
            TheorySpec::NonSusy,
            TachyonSearch::Complete,
            SolverLimits::default(),
        )
        .unwrap();
        assert_eq!(legacy.twist(), &vector(&e["twist"], 6));
        assert_eq!(legacy.modes().vacuum().left(), &number(&e["a_left"], 72));
        assert_eq!(legacy.modes().vacuum().right(), &number(&e["a_right"], 72));
        modes(legacy.modes().left(), &e["left_oscillators"]);
        modes(legacy.modes().right(), &e["right_oscillators"]);
        shells(legacy.right_shells(), &e["right_solutions"]);
        assert_eq!(
            legacy.right_movers().len(),
            e["right_movers"].as_array().unwrap().len()
        );
        for (a, e) in legacy
            .right_movers()
            .iter()
            .zip(e["right_movers"].as_array().unwrap())
        {
            half(a, e, true);
        }
        assert_eq!(
            legacy.tachyon_shells().len(),
            e["tachyonic_right_solutions"].as_array().unwrap().len()
        );
        for (si, (a, e)) in legacy
            .tachyon_shells()
            .iter()
            .zip(e["tachyonic_right_solutions"].as_array().unwrap())
            .enumerate()
        {
            shells(std::slice::from_ref(a), &serde_json::json!([e["solution"]]));
            for m in e["masses"].as_array().unwrap() {
                assert_eq!(legacy.tachyon_masses()[si], number(m, 72));
            }
        }
        assert_eq!(
            legacy.tachyon_movers().len(),
            e["tachyonic_right_movers"].as_array().unwrap().len()
        );
        for (a, e) in legacy
            .tachyon_movers()
            .iter()
            .zip(e["tachyonic_right_movers"].as_array().unwrap())
        {
            half(a, e, true);
        }
        if i == 0 {
            assert_eq!(legacy, complete);
            assert_eq!(complete.tachyon_shells().len(), 3);
            assert_eq!(
                complete
                    .tachyon_shells()
                    .iter()
                    .filter(|s| s.excitation().is_excited())
                    .count(),
                2
            );
            baseline = Some(complete);
        } else {
            assert!(legacy.tachyon_shells().is_empty());
            let base = baseline.as_ref().unwrap();
            assert_eq!(complete.tachyon_shells(), base.tachyon_shells());
            assert_eq!(complete.tachyon_masses(), base.tachyon_masses());
            assert_eq!(complete.tachyon_movers(), base.tachyon_movers());
        }
    }
}
