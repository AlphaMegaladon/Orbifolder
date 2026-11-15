//! Exact ADE characters and Racah--Speiser tensor decomposition in Dynkin coordinates.
use super::{LieFactor, MathError, Matrix, Rational, dynkin_labels};
use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub type Weight = Vec<BigInt>;
pub type Character = BTreeMap<Weight, BigInt>;

/// Immutable algebra data. Every enumeration fails explicitly at its supplied cap.
#[derive(Clone, Debug)]
pub struct CharacterAlgebra {
    cartan: Vec<Weight>,
    metric: Matrix,
    positive: Vec<Weight>,
}
fn sum(a: &[BigInt], b: &[BigInt]) -> Weight {
    a.iter().zip(b).map(|(a, b)| a + b).collect()
}
fn difference(a: &[BigInt], b: &[BigInt]) -> Weight {
    a.iter().zip(b).map(|(a, b)| a - b).collect()
}
impl CharacterAlgebra {
    pub fn new<const N: usize>(factor: &LieFactor<N>) -> Result<Self, MathError> {
        let c = factor.cartan_matrix()?;
        let cartan = (0..c.rows())
            .map(|i| {
                (0..c.cols())
                    .map(|j| c.get(i, j).unwrap().to_integer())
                    .collect()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let metric = c.inverse()?;
        let mut algebra = Self {
            cartan,
            metric,
            positive: Vec::new(),
        };
        let rho = vec![BigInt::one(); algebra.rank()];
        for r in factor.roots() {
            let labels = dynkin_labels(r, factor.simple_roots())?;
            if algebra.dot(&labels, &rho)? > Rational::ZERO {
                algebra.positive.push(labels);
            }
        }
        Ok(algebra)
    }
    pub fn rank(&self) -> usize {
        self.cartan.len()
    }
    fn validate(&self, w: &[BigInt]) -> Result<(), MathError> {
        if w.len() != self.rank() {
            return Err(MathError::DimensionMismatch);
        }
        Ok(())
    }
    fn dot(&self, a: &[BigInt], b: &[BigInt]) -> Result<Rational, MathError> {
        self.validate(a)?;
        self.validate(b)?;
        let mut result = Rational::ZERO;
        for (i, a) in a.iter().enumerate() {
            if a.is_zero() {
                continue;
            }
            for (j, b) in b.iter().enumerate() {
                if !b.is_zero() {
                    result = result + Rational::from(a * b) * self.metric.get(i, j).unwrap();
                }
            }
        }
        Ok(result)
    }
    /// Weyl dimension formula; independent of the character enumeration.
    pub fn dimension(&self, highest: &[BigInt]) -> Result<BigInt, MathError> {
        self.validate(highest)?;
        if highest.iter().any(|x| x < &BigInt::zero()) {
            return Err(MathError::InvalidRootSystem);
        }
        let rho = vec![BigInt::one(); self.rank()];
        let shifted = sum(highest, &rho);
        let mut d = Rational::ONE;
        for r in &self.positive {
            d = d * self.dot(&shifted, r)?.checked_div(&self.dot(&rho, r)?)?;
        }
        d.to_integer()
    }
    /// Freudenthal recursion on the finite norm ball in lambda + root lattice.
    /// Missing weights have multiplicity zero, never a floating-point tolerance.
    pub fn character(&self, highest: &[BigInt], cap: usize) -> Result<Character, MathError> {
        self.dimension(highest)?;
        let bound = self.dot(highest, highest)?;
        let mut known = BTreeSet::from([highest.to_vec()]);
        let mut queue = VecDeque::from([highest.to_vec()]);
        if cap == 0 {
            return Err(MathError::CapacityOverflow);
        }
        while let Some(w) = queue.pop_front() {
            for alpha in &self.cartan {
                let v = difference(&w, alpha);
                if !known.contains(&v) && self.dot(&v, &v)? <= bound {
                    if known.len() == cap {
                        return Err(MathError::CapacityOverflow);
                    }
                    known.insert(v.clone());
                    queue.push_back(v);
                }
            }
        }
        let rho = vec![BigInt::one(); self.rank()];
        let mut ordered = known
            .into_iter()
            .map(|w| Ok((self.dot(&difference(highest, &w), &rho)?, w)))
            .collect::<Result<Vec<_>, MathError>>()?;
        ordered.sort();
        let shifted = sum(highest, &rho);
        let norm = self.dot(&shifted, &shifted)?;
        let mut result = Character::from([(highest.to_vec(), BigInt::one())]);
        for (_, w) in ordered {
            if w == highest {
                continue;
            }
            let denominator = &norm - self.dot(&sum(&w, &rho), &sum(&w, &rho))?;
            if denominator <= Rational::ZERO {
                return Err(MathError::InvalidRootSystem);
            }
            let mut numerator = Rational::ZERO;
            for alpha in &self.positive {
                let mut v = sum(&w, alpha);
                // Norm balls intersect a root ray in an interval; starting inside
                // ensures the first exit terminates the complete sum.
                while self.dot(&v, &v)? <= bound {
                    if let Some(m) = result.get(&v) {
                        numerator = numerator
                            + Rational::from(2) * self.dot(&v, alpha)? * Rational::from(m.clone());
                    }
                    v = sum(&v, alpha);
                }
            }
            let m = numerator.checked_div(&denominator)?.to_integer()?;
            if m < BigInt::zero() {
                return Err(MathError::InvalidRootSystem);
            }
            if !m.is_zero() {
                result.insert(w, m);
            }
        }
        if result.values().sum::<BigInt>() != self.dimension(highest)? {
            return Err(MathError::InvalidRootSystem);
        }
        Ok(result)
    }
    fn dominant(&self, mut w: Weight, cap: usize) -> Result<(Weight, i32), MathError> {
        self.validate(&w)?;
        let mut sign = 1;
        for _ in 0..cap {
            let Some(i) = w.iter().position(|x| x < &BigInt::zero()) else {
                return Ok((w, sign));
            };
            let coefficient = w[i].clone();
            for (j, x) in w.iter_mut().enumerate() {
                *x -= &coefficient * &self.cartan[i][j];
            }
            sign = -sign;
        }
        Err(MathError::CapacityOverflow)
    }
    pub fn dual(&self, highest: &[BigInt], cap: usize) -> Result<Weight, MathError> {
        self.dimension(highest)?;
        Ok(self.dominant(highest.iter().map(|x| -x).collect(), cap)?.0)
    }
    /// Also accepts virtual characters (signed coefficients), used by Adams operations.
    pub fn tensor_character(
        &self,
        highest: &[BigInt],
        weights: &Character,
        cap: usize,
    ) -> Result<Character, MathError> {
        self.dimension(highest)?;
        if weights.len() > cap {
            return Err(MathError::CapacityOverflow);
        }
        let rho = vec![BigInt::one(); self.rank()];
        let mut out = Character::new();
        for (w, m) in weights {
            self.validate(w)?;
            let (shifted, sign) = self.dominant(sum(&sum(highest, w), &rho), cap)?;
            if shifted.iter().any(BigInt::is_zero) {
                continue;
            }
            *out.entry(difference(&shifted, &rho)).or_default() += m * sign;
        }
        out.retain(|_, m| !m.is_zero());
        Ok(out)
    }
    pub fn tensor_product(
        &self,
        a: &[BigInt],
        b: &[BigInt],
        cap: usize,
    ) -> Result<Character, MathError> {
        let out = self.tensor_character(a, &self.character(b, cap)?, cap)?;
        if out.values().any(|x| x < &BigInt::zero()) {
            return Err(MathError::InvalidRootSystem);
        }
        Ok(out)
    }
    /// Project a (possibly virtual) weight character onto its trivial irrep.
    pub fn singlet_coefficient(&self, c: &Character, cap: usize) -> Result<BigInt, MathError> {
        let zero = vec![BigInt::zero(); self.rank()];
        Ok(self
            .tensor_character(&zero, c, cap)?
            .get(&zero)
            .cloned()
            .unwrap_or_default())
    }
}

pub fn adams(character: &Character, power: u32) -> Character {
    let mut out = Character::new();
    for (w, m) in character {
        *out.entry(w.iter().map(|x| x * power).collect())
            .or_default() += m;
    }
    out.retain(|_, m| !m.is_zero());
    out
}
/// Exact convolution for finite characters with a checked output/work budget.
pub fn multiply_characters(
    a: &Character,
    b: &Character,
    cap: usize,
) -> Result<Character, MathError> {
    if a.len().checked_mul(b.len()).is_none_or(|n| n > cap) {
        return Err(MathError::CapacityOverflow);
    }
    let mut out = Character::new();
    for (x, m) in a {
        for (y, n) in b {
            if x.len() != y.len() {
                return Err(MathError::DimensionMismatch);
            }
            *out.entry(sum(x, y)).or_default() += m * n;
        }
    }
    out.retain(|_, m| !m.is_zero());
    Ok(out)
}
