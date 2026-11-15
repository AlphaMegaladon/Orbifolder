use super::{AnomalyError, representation_indices};
use crate::math_core::{Algebra, GaugeVector, Matrix, Rational, RootSystem};
use num_bigint::BigInt;
use num_integer::Integer;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Handedness {
    Left,
    Right,
}
/// Independent Weyl fermions. Unlike the core's CPT-complete spectrum view,
/// right-handed entries here are independent and contribute with a minus sign.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatterField {
    labels: Vec<Vec<BigInt>>,
    charges: Vec<Rational>,
    handedness: Handedness,
    multiplicity: BigInt,
}
impl MatterField {
    pub fn new(
        labels: Vec<Vec<BigInt>>,
        charges: Vec<Rational>,
        handedness: Handedness,
        multiplicity: BigInt,
    ) -> Result<Self, AnomalyError> {
        if multiplicity < BigInt::from(0) {
            return Err(AnomalyError::InvalidMatter);
        }
        Ok(Self {
            labels,
            charges,
            handedness,
            multiplicity,
        })
    }
    pub fn labels(&self) -> &[Vec<BigInt>] {
        &self.labels
    }
    pub fn charges(&self) -> &[Rational] {
        &self.charges
    }
    pub fn handedness(&self) -> Handedness {
        self.handedness
    }
    pub fn multiplicity(&self) -> &BigInt {
        &self.multiplicity
    }
    pub(super) fn transformed(&self, rotation: &Matrix) -> Result<Self, AnomalyError> {
        Ok(Self {
            charges: rotation.apply(&self.charges)?,
            ..self.clone()
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnomalyTraces {
    gravitational: Vec<Rational>,
    mixed: Vec<Vec<Rational>>,
    cubic_gauge: Vec<Rational>,
    cubic_abelian: Vec<Rational>,
    u1_count: usize,
    su2_doublets: Vec<Option<BigInt>>,
    su2_parity: Vec<Option<u8>>,
}
impl AnomalyTraces {
    pub fn gravitational(&self) -> &[Rational] {
        &self.gravitational
    }
    /// Rows are U(1) directions, columns simple gauge factors; normalization T(R).
    pub fn mixed(&self) -> &[Vec<Rational>] {
        &self.mixed
    }
    pub fn gauge_cubic(&self) -> &[Rational] {
        &self.cubic_gauge
    }
    pub fn abelian_cubic(&self, i: usize, j: usize, k: usize) -> Option<&Rational> {
        let n = self.u1_count;
        if i >= n || j >= n || k >= n {
            return None;
        }
        self.cubic_abelian.get((i * n + j) * n + k)
    }
    pub fn su2_doublets(&self) -> &[Option<BigInt>] {
        &self.su2_doublets
    }
    pub fn su2_parity(&self) -> &[Option<u8>] {
        &self.su2_parity
    }
    pub fn u1_count(&self) -> usize {
        self.u1_count
    }
    /// Rank of gravitational/mixed gauge anomaly covectors, independent of basis.
    /// Do not include cubic tensor slices: even a single GS direction has
    /// nonzero C_Aii=C_iAi, so that symmetric tensor's slice rank can be full.
    /// Additional purely abelian obstructions are checked by GS factorization.
    pub fn independent_abelian_directions(&self) -> Result<usize, AnomalyError> {
        let n = self.u1_count;
        if n == 0 {
            return Ok(0);
        }
        let mut columns = vec![self.gravitational.clone()];
        for (g, _) in self.cubic_gauge.iter().enumerate() {
            columns.push((0..n).map(|i| self.mixed[i][g].clone()).collect());
        }
        Ok(Matrix::from_rows(columns)?.rank()?)
    }
}

pub(super) fn validate_basis(
    system: &RootSystem<16>,
    generators: &[GaugeVector],
) -> Result<(), AnomalyError> {
    if generators.len() > 16
        || generators
            .iter()
            .any(|u| system.roots().iter().any(|r| !u.dot(r).is_zero()))
    {
        return Err(AnomalyError::InvalidBasis);
    }
    let rows = Matrix::new(
        generators.len(),
        16,
        generators
            .iter()
            .flat_map(|u| u.coordinates().iter().cloned())
            .collect(),
    )?;
    if rows.rank()? != generators.len() {
        return Err(AnomalyError::InvalidBasis);
    }
    Ok(())
}
pub fn trace_matter(
    system: &RootSystem<16>,
    generators: &[GaugeVector],
    matter: &[MatterField],
) -> Result<AnomalyTraces, AnomalyError> {
    validate_basis(system, generators)?;
    let n = generators.len();
    let f = system.factors().len();
    let mut out = AnomalyTraces {
        gravitational: vec![Rational::ZERO; n],
        mixed: vec![vec![Rational::ZERO; f]; n],
        cubic_gauge: vec![Rational::ZERO; f],
        cubic_abelian: vec![Rational::ZERO; n * n * n],
        u1_count: n,
        su2_doublets: system
            .factors()
            .iter()
            .map(|f| (f.algebra() == &Algebra::A(1)).then(|| BigInt::from(0)))
            .collect(),
        su2_parity: vec![None; f],
    };
    let mut index_cache = BTreeMap::new();
    let mut witten = vec![BigInt::from(0); f];
    for field in matter {
        if field.labels.len() != f || field.charges.len() != n {
            return Err(AnomalyError::InvalidMatter);
        }
        let mut indices = Vec::with_capacity(f);
        for (gi, factor) in system.factors().iter().enumerate() {
            let key = (gi, field.labels[gi].clone());
            if !index_cache.contains_key(&key) {
                index_cache.insert(key.clone(), representation_indices(factor, &key.1)?);
            }
            indices.push(index_cache[&key].clone());
        }
        let total = indices
            .iter()
            .fold(field.multiplicity.clone(), |s, x| s * &x.dimension);
        let sign = if field.handedness == Handedness::Left {
            Rational::ONE
        } else {
            Rational::from(-1)
        };
        let signed_total = sign * Rational::from(total.clone());
        for g in 0..f {
            let spectators = &total / &indices[g].dimension;
            let signed = if field.handedness == Handedness::Left {
                Rational::from(spectators.clone())
            } else {
                -Rational::from(spectators.clone())
            };
            out.cubic_gauge[g] = &out.cubic_gauge[g] + &signed * indices[g].cubic_coefficient();
            for i in 0..n {
                out.mixed[i][g] =
                    &out.mixed[i][g] + &signed * &field.charges[i] * &indices[g].quadratic;
            }
            if let Some(doublets) = &mut out.su2_doublets[g] {
                if indices[g].dimension == BigInt::from(2) {
                    *doublets += &spectators;
                }
                // Witten parity counts both handednesses and all isospins.
                witten[g] +=
                    &spectators * (Rational::from(2) * &indices[g].quadratic).to_integer()?;
            }
        }
        for i in 0..n {
            out.gravitational[i] = &out.gravitational[i] + &signed_total * &field.charges[i];
            for j in 0..n {
                for k in 0..n {
                    let slot = (i * n + j) * n + k;
                    out.cubic_abelian[slot] = &out.cubic_abelian[slot]
                        + &signed_total * &field.charges[i] * &field.charges[j] * &field.charges[k];
                }
            }
        }
    }
    for (g, v) in witten.iter().enumerate() {
        if out.su2_doublets[g].is_some() {
            out.su2_parity[g] = Some(u8::from(v.is_odd()));
        }
    }
    Ok(out)
}
