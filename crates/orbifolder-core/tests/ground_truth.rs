//! Differential mathematical tests only. No Rust geometry or state solver.
mod common;
use num_integer::Integer;
use num_traits::ToPrimitive;
use orbifolder_core::math_core::{
    DnClass, GaugeLattice, GaugeVector, Matrix, Rational, RootSystem, Vector, cartan_matrix,
    dynkin_labels, highest_weight, so8_class,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CASES: [&str; 7] = [
    "susy_z3",
    "susy_z6_ii",
    "non_susy_z3",
    "non_susy_z6_ii",
    "susy_z3xz3_wl",
    "non_susy_z3xz3_wl",
    "non_susy_tachyonic",
];

fn checksum(path: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(std::fs::read(common::root().join(path)).unwrap())
    )
}

#[test]
fn seven_fixtures_and_supplement_are_integrity_bound() {
    let oracle = common::json("math_oracle.json");
    assert_eq!(oracle["cases"].as_object().unwrap().len(), CASES.len());
    assert_eq!(
        std::fs::read_dir(common::root().join("fixtures"))
            .unwrap()
            .filter(|p| p
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|x| x == "json"))
            .count(),
        CASES.len()
    );
    for case in CASES {
        assert_eq!(
            oracle["cases"][case]["fixture_sha256"],
            checksum(&format!("fixtures/{case}.json"))
        );
    }
    for (field, path) in [
        ("probe_source_sha256", "probes/math_probe.cpp"),
        ("build_script_sha256", "build_math_probes.sh"),
        ("generator_sha256", "generate_math.py"),
    ] {
        assert_eq!(oracle["provenance"][field], checksum(path));
    }
}

fn inputs(data: &Value) -> (Vec<GaugeVector>, i64) {
    let text = data["input"]["model_text"].as_str().unwrap();
    let rows = text
        .split("Shifts and Wilsonlines:\n")
        .nth(1)
        .unwrap()
        .lines()
        .take_while(|line| !line.starts_with("end model"));
    let mut bound = 1_i64;
    let vectors = rows
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let entries = line
                .split_whitespace()
                .map(|x| x.parse::<Rational>().unwrap())
                .collect::<Vec<_>>();
            for x in &entries {
                bound = bound.lcm(&x.denominator().to_i64().unwrap());
            }
            Vector::from_slice(&entries).unwrap()
        })
        .collect();
    (vectors, 2 * bound) // unshifted parent momenta are half-integral.
}

fn native_gauge_roots(data: &Value, bound: i64) -> BTreeSet<GaugeVector> {
    let mut roots = BTreeSet::new();
    for state in data["sectors"][0]["fixed_branes"][0]["states"]
        .as_array()
        .unwrap()
    {
        if !state["left"]["eigenphases"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| common::number(x, bound).is_integer())
        {
            continue;
        }
        for index in state["left"]["weight_indices"].as_array().unwrap() {
            let weight = common::vector::<16>(
                &state["left_solution"]["weights"][index.as_u64().unwrap() as usize],
                2,
            );
            if weight.norm_squared() == Rational::from(2) {
                roots.insert(weight);
            }
        }
    }
    roots
}

fn state_for_field<'a>(data: &'a Value, field: &Value, tachyon: bool) -> &'a Value {
    let index = field["source_index"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as usize)
        .collect::<Vec<_>>();
    let brane = &data["sectors"][index[0]]["fixed_branes"][index[1]];
    if !tachyon {
        return &brane["states"][index[2]];
    }
    let ground = brane["tachyonic_states"].as_array().unwrap().len();
    if index[2] < ground {
        &brane["tachyonic_states"][index[2]]
    } else {
        &brane["excited_tachyonic_states"][index[2] - ground]
    }
}

fn verify_case(case: &str) {
    let data = common::json(&format!("fixtures/{case}.json"));
    let supplement = common::json("math_oracle.json");
    let (constraints, bound) = inputs(&data);
    let parent_native = data["math"]["parent_e8xe8_roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| common::vector::<16>(x, 2))
        .collect::<Vec<_>>();
    assert_eq!(parent_native.len(), 480);
    assert_eq!(
        parent_native.iter().cloned().collect::<BTreeSet<_>>(),
        GaugeLattice::E8xE8.roots().into_iter().collect()
    );
    let arithmetic = &data["math"]["rational_arithmetic"];
    let a = common::number(&arithmetic["a"], 1);
    let b = common::number(&arithmetic["b"], 1);
    assert_eq!(&a + &b, common::number(&arithmetic["sum"], 1));
    assert_eq!(&a * &b, common::number(&arithmetic["product"], 1));
    assert_eq!(
        a.checked_div(&b).unwrap(),
        common::number(&arithmetic["quotient"], 1)
    );
    for sample in data["math"]["so8_samples"].as_array().unwrap() {
        let class = so8_class(&common::vector(&sample["weight"], 2));
        let s = match class {
            Some(DnClass::Vector) => 1,
            Some(DnClass::Spinor) => 2,
            _ => 0,
        };
        if data["variant"] == "susy" {
            assert_eq!(sample["ns_r_class"], s);
        } else {
            assert_eq!(sample["so8_s_class"], s);
            let c = match class {
                Some(DnClass::Vector) => 1,
                Some(DnClass::Cospinor) => 3,
                _ => 0,
            };
            assert_eq!(sample["so8_c_class"], c);
        }
    }
    let native_constraints = data["orbifold_group"]["shifts"]
        .as_array()
        .unwrap()
        .iter()
        .chain(data["orbifold_group"]["wilson_lines"].as_array().unwrap())
        .map(|v| common::vector::<16>(v, bound))
        .collect::<Vec<_>>();
    assert_eq!(
        constraints, native_constraints,
        "exact saved inputs must agree with observed embedding"
    );
    let roots = GaugeLattice::E8xE8.surviving_roots(&constraints);
    assert_eq!(
        roots.iter().cloned().collect::<BTreeSet<_>>(),
        native_gauge_roots(&data, bound),
        "native projected gauge-root set"
    );
    // The legacy chamber is a saved basis convention, supplied only by this adapter.
    let chamber = Vector::from_integers([23, 6, 5, 4, 3, 2, 1, 0, 23, 6, 5, 4, 3, 2, 1, 0]);
    let system = RootSystem::new(roots, &chamber).unwrap();
    let native_factors = data["gauge_group"]["factors"].as_array().unwrap();
    assert_eq!(system.factors().len(), native_factors.len());
    let mut bases = Vec::new();
    for (fi, native) in native_factors.iter().enumerate() {
        let native_basis = native["simple_roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| common::vector::<16>(x, 2))
            .collect::<Vec<_>>();
        let expected_set = native_basis.iter().cloned().collect::<BTreeSet<_>>();
        let matches = system
            .factors()
            .iter()
            .filter(|f| f.simple_roots().iter().cloned().collect::<BTreeSet<_>>() == expected_set)
            .collect::<Vec<_>>();
        assert_eq!(
            matches.len(),
            1,
            "exact simple-root equality before any basis permutation"
        );
        let factor = matches[0];
        assert_eq!(
            factor.algebra().to_string(),
            native["algebra"].as_str().unwrap()
        );
        assert_eq!(
            factor.algebra().rank(),
            native["rank"].as_u64().unwrap() as usize
        );
        // Align only by exact vector identity, preserving the C++ Dynkin node numbering.
        let aligned = native_basis
            .iter()
            .map(|a| {
                factor
                    .simple_roots()
                    .iter()
                    .find(|b| *b == a)
                    .unwrap()
                    .clone()
            })
            .collect::<Vec<_>>();
        let cpp = &supplement["cases"][case]["factors"][fi];
        assert_eq!(cpp["algebra"], native["algebra"]);
        let cartan = cartan_matrix(&aligned).unwrap();
        assert_eq!(cartan, common::matrix(&cpp["cartan"], 1));
        assert!(cartan.is_positive_definite());
        for (i, root) in parent_native.iter().enumerate() {
            let expected = cpp["parent_root_dynkin"][i]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_i64().unwrap().into())
                .collect::<Vec<_>>();
            assert_eq!(dynkin_labels(root, &aligned).unwrap(), expected);
        }
        bases.push(aligned);
    }
    let all_simple = bases.iter().flatten().cloned().collect::<Vec<_>>();
    let u1 = data["gauge_group"]["u1_generators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| common::vector::<16>(v, 1))
        .collect::<Vec<_>>();
    for generator in &u1 {
        assert!(system.roots().iter().all(|r| r.dot(generator).is_zero()));
    }
    let basis = all_simple
        .iter()
        .chain(&u1)
        .map(|v| v.coordinates().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(Matrix::from_rows(basis).unwrap().rank().unwrap(), 16);
    let fields = data["spectrum"]["fields"].as_array().unwrap();
    let massless = fields.len()
        - data["tachyonic_spectrum"]["fields"]
            .as_array()
            .unwrap()
            .len();
    for (i, field) in fields.iter().enumerate() {
        let state = state_for_field(&data, field, i >= massless);
        let weights = field["weight_indices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|index| {
                common::vector::<16>(
                    &state["left_solution"]["weights"][index.as_u64().unwrap() as usize],
                    bound,
                )
            })
            .collect::<Vec<_>>();
        let highest = highest_weight(&weights, &all_simple).unwrap();
        for (fi, basis) in bases.iter().enumerate() {
            let expected = field["highest_weights"][fi]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_i64().unwrap().into())
                .collect::<Vec<_>>();
            assert_eq!(
                dynkin_labels(&highest, basis).unwrap(),
                expected,
                "{case}: field {i}"
            );
        }
        for (j, generator) in u1.iter().enumerate() {
            let charge = common::number(&field["u1_charges"][j], bound);
            assert!(
                weights.iter().all(|weight| weight.dot(generator) == charge),
                "{case}: field {i}, U1 {j}"
            );
        }
    }
    let gram = common::matrix(&data["geometry"]["exact_metric"]["gram_matrix"], 1);
    assert!(gram.is_positive_definite());
    assert_eq!(
        gram.multiply(&gram.inverse().unwrap()).unwrap(),
        Matrix::identity(6).unwrap()
    );
    for entry in data["geometry"]["rotation_actions"].as_array().unwrap() {
        let action = common::matrix(&entry["matrix"], 1);
        assert!(gram.is_isometry(&action).unwrap());
        let order = data["geometry"]["orders"]
            .as_array()
            .unwrap()
            .iter()
            .zip(entry["generator"]["factors"].as_array().unwrap())
            .fold(1_u64, |acc, (n, m)| {
                let n = n.as_u64().unwrap();
                acc.lcm(&(n / n.gcd(&m.as_u64().unwrap())))
            });
        let mut power = Matrix::identity(6).unwrap();
        for _ in 0..order {
            power = power.multiply(&action).unwrap();
        }
        assert_eq!(power, Matrix::identity(6).unwrap());
    }
    eprintln!(
        "PASS {case}: 480 parent roots, {} gauge roots, rank {}, {} field highest weights and charges, exact Gram isometries",
        system.roots().len(),
        system.rank(),
        fields.len()
    );
}

macro_rules! oracle_case {
    ($name:ident) => {
        #[test]
        fn $name() {
            verify_case(stringify!($name));
        }
    };
}
oracle_case!(susy_z3);
oracle_case!(susy_z6_ii);
oracle_case!(non_susy_z3);
oracle_case!(non_susy_z6_ii);
oracle_case!(susy_z3xz3_wl);
oracle_case!(non_susy_z3xz3_wl);
oracle_case!(non_susy_tachyonic);
