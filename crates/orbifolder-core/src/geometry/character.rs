//! Match the real torus character to the spatial spin twist, without trig/floats.
use super::ModelError;
use crate::math_core::{Matrix, Rational, RightWeight};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};
use std::collections::BTreeMap;

fn cyclotomic(n: u32, cache: &mut BTreeMap<u32, Vec<BigInt>>) -> Vec<BigInt> {
    if let Some(p) = cache.get(&n) {
        return p.clone();
    }
    let mut p = vec![BigInt::from(0); n as usize + 1];
    p[0] = BigInt::from(-1);
    p[n as usize] = BigInt::from(1);
    for divisor in 1..n {
        if n % divisor != 0 {
            continue;
        }
        let q = cyclotomic(divisor, cache);
        let mut quotient = vec![BigInt::from(0); p.len() - q.len() + 1];
        for i in (0..quotient.len()).rev() {
            let factor = p[i + q.len() - 1].clone();
            quotient[i] = factor.clone();
            for (j, a) in q.iter().enumerate() {
                p[i + j] -= &factor * a;
            }
        }
        debug_assert!(p.iter().all(Zero::is_zero));
        p = quotient;
    }
    cache.insert(n, p.clone());
    p
}
pub(super) fn consistent(rotation: &Matrix, twist: &RightWeight) -> Result<bool, ModelError> {
    if !twist.coordinates()[0].is_zero() {
        return Ok(false);
    }
    let mut roots: BTreeMap<u32, BTreeMap<u32, usize>> = BTreeMap::new();
    for v in &twist.coordinates()[1..] {
        for value in [v.clone(), -v] {
            let phase = value.modulo_one();
            let d = phase
                .denominator()
                .to_u32()
                .ok_or(ModelError::ResourceLimit)?;
            let a = phase
                .numerator()
                .to_u32()
                .ok_or(ModelError::ResourceLimit)?;
            *roots.entry(d).or_default().entry(a).or_default() += 1;
        }
    }
    let mut polynomial = vec![BigInt::from(1)];
    let mut cache = BTreeMap::new();
    for (d, counts) in roots {
        if d > 4096 {
            return Err(ModelError::ResourceLimit);
        }
        let multiplicity = *counts.values().next().unwrap();
        if (0..d)
            .filter(|a| a.gcd(&d) == 1)
            .any(|a| counts.get(&a) != Some(&multiplicity))
        {
            return Ok(false);
        }
        let factor = cyclotomic(d, &mut cache);
        for _ in 0..multiplicity {
            let mut product = vec![BigInt::from(0); polynomial.len() + factor.len() - 1];
            for (i, a) in polynomial.iter().enumerate() {
                for (j, b) in factor.iter().enumerate() {
                    product[i + j] += a * b;
                }
            }
            polynomial = product;
        }
    }
    if polynomial.len() != 7 {
        return Ok(false);
    }
    let mut traces = Vec::new();
    let mut power = Matrix::identity(6)?;
    for _ in 0..6 {
        power = power.multiply(rotation)?;
        traces.push((0..6).fold(Rational::ZERO, |s, i| s + power.get(i, i).unwrap()));
    }
    let mut coefficients = vec![Rational::ONE];
    for k in 1..=6 {
        let sum = (1..=k).fold(Rational::ZERO, |s, i| {
            s + &coefficients[k - i] * &traces[i - 1]
        });
        coefficients.push((-sum).checked_div(&Rational::from(k as i128))?);
    }
    Ok(coefficients
        .into_iter()
        .eq(polynomial.into_iter().rev().map(Rational::from)))
}
