mod common;
#[path = "common/geometry_input.rs"]
mod geometry_input;
use num_bigint::BigInt;
use orbifolder_core::{
    geometry::{Geometry, TheorySpec, embed},
    math_core::{Character, CharacterAlgebra, Weight, adams, multiply_characters},
    physics_engine::{SolverLimits, TachyonSearch, calculate_spectrum, solve_sectors},
};
use serde_json::Value;
use std::collections::BTreeSet;

fn labels(v: &Value, permutation: &[usize]) -> Weight {
    let mut out = vec![BigInt::from(0); permutation.len()];
    for (i, &j) in permutation.iter().enumerate() {
        out[j] = v[i].as_i64().unwrap().into();
    }
    out
}
fn character(v: &Value, p: &[usize]) -> Character {
    v.as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                labels(&w["weight"], p),
                w["multiplicity"].as_i64().unwrap().into(),
            )
        })
        .collect()
}
fn scaled_sum(a: &Character, b: &Character, n: i32) -> Character {
    let mut out = a.clone();
    for (w, m) in b {
        *out.entry(w.clone()).or_default() += m * n;
    }
    for m in out.values_mut() {
        assert_eq!(&*m % 2, BigInt::from(0));
        *m /= 2;
    }
    out.retain(|_, m| m != &BigInt::from(0));
    out
}
fn verify(case: &str) {
    let fixture = common::json(&format!("fixtures/{case}.json"));
    let oracle = common::json("couplings_oracle.json");
    let geometry = Geometry::build(geometry_input::from_fixture(&fixture)).unwrap();
    let group = embed(
        geometry,
        geometry_input::embedding("susy", fixture["input"]["model_text"].as_str().unwrap()),
        TheorySpec::Susy,
    )
    .unwrap();
    let spectrum = calculate_spectrum(
        &solve_sectors(&group, TachyonSearch::Complete, SolverLimits::default()).unwrap(),
        SolverLimits::default(),
    )
    .unwrap();
    let factors = spectrum.basis().root_system().factors();
    let aligned = fixture["gauge_group"]["factors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|native| {
            let roots = native["simple_roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| common::vector::<16>(x, 2))
                .collect::<Vec<_>>();
            let f = factors
                .iter()
                .find(|f| {
                    f.simple_roots().iter().collect::<BTreeSet<_>>() == roots.iter().collect()
                })
                .unwrap();
            let p = roots
                .iter()
                .map(|r| f.simple_roots().iter().position(|x| x == r).unwrap())
                .collect::<Vec<_>>();
            (CharacterAlgebra::new(f).unwrap(), p)
        })
        .collect::<Vec<_>>();
    let native = &oracle["cases"][case]["native"]["characters"];
    let mut rust = Vec::new();
    for record in native["characters"].as_array().unwrap() {
        let (a, p) = &aligned[record["factor"].as_u64().unwrap() as usize];
        let l = labels(&record["highest"], p);
        let c = a.character(&l, 100_000).unwrap();
        assert_eq!(
            c,
            character(&record["weights"], p),
            "{case} character {l:?}"
        );
        assert_eq!(
            a.dimension(&l).unwrap().to_string(),
            record["dimension"].to_string()
        );
        rust.push(c);
    }
    for pair in native["pairs"].as_array().unwrap() {
        let i = pair["left"].as_u64().unwrap() as usize;
        let j = pair["right"].as_u64().unwrap() as usize;
        let record = &native["characters"][i];
        let (a, p) = &aligned[record["factor"].as_u64().unwrap() as usize];
        let product = multiply_characters(&rust[i], &rust[j], 1_000_000).unwrap();
        assert_eq!(
            product,
            character(&pair["tensor_character"], p),
            "{case} pair {i} {j}"
        );
        let irreps = a
            .tensor_product(
                &labels(&record["highest"], p),
                &labels(&native["characters"][j]["highest"], p),
                100_000,
            )
            .unwrap();
        let mut reconstructed = Character::new();
        for (w, m) in &irreps {
            for (v, n) in a.character(w, 100_000).unwrap() {
                *reconstructed.entry(v).or_default() += m * n;
            }
        }
        assert_eq!(
            product, reconstructed,
            "{case} tensor reconstruction {i} {j}"
        );
        if pair.get("symmetric_character").is_some() {
            assert_eq!(
                scaled_sum(&product, &adams(&rust[i], 2), 1),
                character(&pair["symmetric_character"], p)
            );
            assert_eq!(
                scaled_sum(&product, &adams(&rust[i], 2), -1),
                character(&pair["antisymmetric_character"], p)
            );
        }
        if let Some(cube) = pair.get("symmetric_cube_character") {
            let mut c = multiply_characters(&product, &rust[i], 1_000_000).unwrap();
            for (w, m) in multiply_characters(&rust[i], &adams(&rust[i], 2), 1_000_000).unwrap() {
                *c.entry(w).or_default() += m * 3;
            }
            for (w, m) in adams(&rust[i], 3) {
                *c.entry(w).or_default() += m * 2;
            }
            for m in c.values_mut() {
                assert_eq!(&*m % 6, BigInt::from(0));
                *m /= 6;
            }
            c.retain(|_, m| m != &BigInt::from(0));
            assert_eq!(c, character(cube, p));
        }
        if let Some(decomposition) = pair.get("tensor") {
            let mut c = Character::new();
            for irrep in decomposition.as_array().unwrap() {
                for (w, m) in character(irrep, p) {
                    *c.entry(w).or_default() += m;
                }
            }
            assert_eq!(product, c);
        }
    }
}
#[test]
fn z3_characters() {
    verify("susy_z3");
}
#[test]
fn z6_characters() {
    verify("susy_z6_ii");
}
#[test]
fn z3xz3_characters() {
    verify("susy_z3xz3_wl");
}
