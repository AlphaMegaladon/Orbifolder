use super::{AnomalyError, traces::validate_basis};
use crate::math_core::{GaugeVector, Matrix, Rational, RootSystem};
use num_bigint::BigInt;
use num_integer::Integer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BasisRotation {
    generators: Vec<GaugeVector>,
    transformation: Matrix,
    gram: Matrix,
    trace_vector: GaugeVector,
    anomalous: Option<usize>,
}
impl BasisRotation {
    pub fn generators(&self) -> &[GaugeVector] {
        &self.generators
    }
    /// U_new=A U_old, Q_new=A Q_old. Full Gram: G_new=A G_old A^T.
    pub fn transformation(&self) -> &Matrix {
        &self.transformation
    }
    pub fn gram(&self) -> &Matrix {
        &self.gram
    }
    pub fn trace_vector(&self) -> &GaugeVector {
        &self.trace_vector
    }
    pub fn anomalous_index(&self) -> Option<usize> {
        self.anomalous
    }
    pub fn norm_squared(&self, i: usize) -> Option<&Rational> {
        self.gram.get(i, i)
    }
}
pub fn canonical_u1_basis(
    system: &RootSystem<16>,
    old: &[GaugeVector],
    trace: &[Rational],
) -> Result<BasisRotation, AnomalyError> {
    validate_basis(system, old)?;
    if old.len() != trace.len() {
        return Err(AnomalyError::InvalidBasis);
    }
    let n = old.len();
    let old_gram = Matrix::new(
        n,
        n,
        old.iter()
            .flat_map(|a| old.iter().map(move |b| a.dot(b)))
            .collect(),
    )?;
    let coefficients = old_gram.solve(trace)?.particular().to_vec();
    let vector = old
        .iter()
        .zip(&coefficients)
        .fold(GaugeVector::zero(), |s, (u, c)| s.add(&u.scale(c)));
    let mut generators = Vec::new();
    let anomalous = (!vector.norm_squared().is_zero()).then_some(0);
    if anomalous.is_some() {
        generators.push(vector.scale(&Rational::new(1, 12)?));
    }
    for candidate in old {
        let mut next = candidate.clone();
        for u in &generators {
            next = next.sub(&u.scale(&next.dot(u).checked_div(&u.norm_squared())?));
        }
        if !next.norm_squared().is_zero() {
            generators.push(primitive(&next)?);
        }
    }
    if generators.len() != n {
        return Err(AnomalyError::InvalidBasis);
    }
    let old_matrix = Matrix::new(
        n,
        16,
        old.iter()
            .flat_map(|u| u.coordinates().iter().cloned())
            .collect(),
    )?;
    let transformation = Matrix::new(
        n,
        n,
        generators
            .iter()
            .map(|u| {
                old_matrix
                    .transpose()
                    .solve(u.coordinates())
                    .map(|s| s.particular().to_vec())
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect(),
    )?;
    let gram = Matrix::new(
        n,
        n,
        generators
            .iter()
            .flat_map(|a| generators.iter().map(move |b| a.dot(b)))
            .collect(),
    )?;
    Ok(BasisRotation {
        generators,
        transformation,
        gram,
        trace_vector: vector,
        anomalous,
    })
}
fn primitive(v: &GaugeVector) -> Result<GaugeVector, AnomalyError> {
    let denominator = v
        .coordinates()
        .iter()
        .fold(BigInt::from(1), |d, x| d.lcm(&x.denominator()));
    let entries = v
        .coordinates()
        .iter()
        .map(|x| (x * Rational::from(denominator.clone())).to_integer())
        .collect::<Result<Vec<_>, _>>()?;
    let gcd = entries.iter().fold(BigInt::from(0), |g, x| g.gcd(x));
    let sign = if entries
        .iter()
        .find(|x| **x != BigInt::from(0))
        .is_some_and(|x| x < &BigInt::from(0))
    {
        -1
    } else {
        1
    };
    Ok(v.scale(
        &(Rational::from(denominator) * Rational::from(sign)).checked_div(&Rational::from(gcd))?,
    ))
}
