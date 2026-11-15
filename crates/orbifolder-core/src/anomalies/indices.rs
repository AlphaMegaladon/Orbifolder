use super::{AnomalyError, Computation};
use crate::math_core::{Algebra, GaugeVector, LieFactor, Rational, fundamental_weights};
use crate::physics_engine::representation_dimension;
use num_bigint::BigInt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepresentationIndices {
    pub dimension: BigInt,
    pub quadratic_casimir: Rational,
    /// T(fundamental SU(N))=1/2, T(vector SO(2N))=1.
    pub quadratic: Rational,
    /// A(fundamental SU(N>=3))=1; absence of a cubic invariant is explicit.
    pub cubic: Computation<Rational>,
}
impl RepresentationIndices {
    pub fn cubic_coefficient(&self) -> Rational {
        match &self.cubic {
            Computation::Computed(x) => x.clone(),
            _ => Rational::ZERO,
        }
    }
}

pub fn representation_indices(
    factor: &LieFactor<16>,
    labels: &[BigInt],
) -> Result<RepresentationIndices, AnomalyError> {
    if labels.len() != factor.algebra().rank() || labels.iter().any(|x| x < &BigInt::from(0)) {
        return Err(AnomalyError::InvalidMatter);
    }
    let fundamental = fundamental_weights(factor.simple_roots())?;
    let highest = fundamental
        .iter()
        .zip(labels)
        .fold(GaugeVector::zero(), |s, (w, n)| {
            s.add(&w.scale(&Rational::from(n.clone())))
        });
    let rho = fundamental
        .iter()
        .fold(GaugeVector::zero(), |s, w| s.add(w));
    let dimension = representation_dimension(factor, &highest)?;
    let casimir = (highest.dot(&highest.add(&rho.scale(&Rational::from(2)))))
        .checked_div(&Rational::from(2))?;
    let algebra_dimension = BigInt::from(factor.algebra().root_count()? + factor.algebra().rank());
    let quadratic = (Rational::from(dimension.clone()) * &casimir)
        .checked_div(&Rational::from(algebra_dimension))?;
    let cubic = match factor.algebra() {
        Algebra::A(rank) if *rank >= 2 => {
            let order = a_chain(factor)?;
            let n = rank + 1;
            let rows = (0..n)
                .map(|i| {
                    order[i.min(*rank)..]
                        .iter()
                        .fold(BigInt::from(0), |s, &j| s + &labels[j])
                })
                .collect::<Vec<_>>();
            let cubic_weight = shifted_cube(&rows)?;
            let mut fund = vec![BigInt::from(0); n];
            fund[0] = BigInt::from(1);
            Computation::Computed(
                (Rational::from(dimension.clone()) * cubic_weight)
                    .checked_div(&(Rational::from(BigInt::from(n)) * shifted_cube(&fund)?))?,
            )
        }
        _ => Computation::NotApplicable("No invariant symmetric cubic tensor on this Lie algebra"),
    };
    Ok(RepresentationIndices {
        dimension,
        quadratic_casimir: casimir,
        quadratic,
        cubic,
    })
}

/// Orient A_n at the lexicographically smallest endpoint, independent of the
/// stored Dynkin numbering. This fixes the fundamental/conjugate sign convention.
fn a_chain(factor: &LieFactor<16>) -> Result<Vec<usize>, AnomalyError> {
    let simple = factor.simple_roots();
    let adjacency = simple
        .iter()
        .map(|a| {
            simple
                .iter()
                .enumerate()
                .filter_map(|(j, b)| (a.dot(b) == Rational::from(-1)).then_some(j))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let head = (0..simple.len())
        .filter(|&i| adjacency[i].len() == 1)
        .min_by_key(|&i| &simple[i])
        .ok_or(AnomalyError::InvalidMatter)?;
    let mut order = Vec::new();
    let mut current = head;
    loop {
        order.push(current);
        let Some(next) = adjacency[current].iter().find(|&&j| !order.contains(&j)) else {
            break;
        };
        current = *next;
    }
    if order.len() != simple.len() {
        return Err(AnomalyError::InvalidMatter);
    }
    Ok(order)
}
fn shifted_cube(rows: &[BigInt]) -> Result<Rational, AnomalyError> {
    let n = rows.len();
    let sum = rows.iter().fold(BigInt::from(0), |s, x| s + x);
    let average = Rational::from(sum).checked_div(&Rational::from(BigInt::from(n)))?;
    let mut result = Rational::ZERO;
    for (i, row) in rows.iter().enumerate() {
        let rho = Rational::from(BigInt::from(n) + BigInt::from(1) - BigInt::from(2 * (i + 1)))
            .checked_div(&Rational::from(2))?;
        let x = Rational::from(row.clone()) - &average + rho;
        result = result + &x * &x * &x;
    }
    Ok(result)
}
