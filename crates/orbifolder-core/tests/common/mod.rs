#![allow(dead_code)]
use orbifolder_core::math_core::{Matrix, Rational, Vector};
use serde_json::Value;
use std::path::PathBuf;

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ground_truth")
}
pub fn json(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(root().join(path)).unwrap()).unwrap()
}

/// Test adapter only: unique bounded rational reconstruction of observed C++
/// doubles. The bound is supplied from exact input data, never inferred by the core.
pub fn number(value: &Value, bound: i64) -> Rational {
    if value.is_object() {
        return format!("{}/{}", value["num"], value["den"])
            .parse()
            .unwrap();
    }
    let x = value.as_f64().unwrap();
    assert!(x.is_finite());
    // At most 256 rounding steps for these saved low-order linear combinations.
    let budget = 256.0 * f64::EPSILON * (1.0 + x.abs());
    let mut candidates = std::collections::BTreeSet::new();
    for d in 1..=bound {
        let n = (x * d as f64).round();
        if (n / d as f64 - x).abs() <= budget {
            assert!(n.abs() < (i64::MAX as f64));
            candidates.insert(Rational::new(n as i128, i128::from(d)).unwrap());
        }
    }
    assert_eq!(
        candidates.len(),
        1,
        "no unique bounded rational for {x}, bound={bound}"
    );
    candidates.pop_first().unwrap()
}
pub fn vector<const N: usize>(value: &Value, bound: i64) -> Vector<N> {
    Vector::from_slice(
        &value
            .as_array()
            .unwrap()
            .iter()
            .map(|x| number(x, bound))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}
pub fn matrix(value: &Value, bound: i64) -> Matrix {
    Matrix::from_rows(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                r.as_array()
                    .unwrap()
                    .iter()
                    .map(|x| number(x, bound))
                    .collect()
            })
            .collect(),
    )
    .unwrap()
}
