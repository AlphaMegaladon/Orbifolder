use super::{AnalysisLimits, SusyError, SusySpectrum};
use crate::{
    math_core::{Character, CharacterAlgebra, Rational, Weight, adams},
    physics_engine::{Multiplet, SusyMultiplet},
};
use num_bigint::BigInt;
use num_traits::{One, Zero};
use std::collections::BTreeMap;

/// Stable field indices with positive integral exponents, validated on a SUSY view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Monomial {
    fields: Vec<(usize, u32)>,
    degree: u32,
}
impl Monomial {
    pub fn new(view: &SusySpectrum, indices: &[usize]) -> Result<Self, SusyError> {
        if indices.is_empty() {
            return Err(SusyError::InvalidMonomial);
        }
        if indices.len() > 3 {
            return Err(SusyError::UnsupportedOrder);
        }
        let mut fields = BTreeMap::new();
        for &i in indices {
            let field = view
                .spectrum()
                .fields()
                .get(i)
                .ok_or(SusyError::InvalidField)?;
            if field.multiplet() != Multiplet::Susy(SusyMultiplet::LeftChiral) {
                return Err(SusyError::InvalidField);
            }
            *fields.entry(i).or_insert(0) += 1;
        }
        Ok(Self {
            fields: fields.into_iter().collect(),
            degree: indices.len() as u32,
        })
    }
    pub fn fields(&self) -> &[(usize, u32)] {
        &self.fields
    }
    pub fn degree(&self) -> u32 {
        self.degree
    }
    pub fn expanded(&self) -> Vec<usize> {
        self.fields
            .iter()
            .flat_map(|&(i, n)| std::iter::repeat_n(i, n as usize))
            .collect()
    }
    pub fn multiply(&self, other: &Self, view: &SusySpectrum) -> Result<Self, SusyError> {
        let mut fields = self.expanded();
        fields.extend(other.expanded());
        Self::new(view, &fields)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvariantReport {
    pub(super) monomial: Monomial,
    pub(super) charge_sum: Vec<Rational>,
    /// Singlets in the unrestricted tensor product, as checked by the native C++.
    pub(super) tensor_singlets: BigInt,
    /// Singlets after symmetrizing repeated complete field representations.
    pub(super) polynomial_singlets: BigInt,
}
impl InvariantReport {
    pub fn monomial(&self) -> &Monomial {
        &self.monomial
    }
    pub fn charge_sum(&self) -> &[Rational] {
        &self.charge_sum
    }
    pub fn tensor_singlets(&self) -> &BigInt {
        &self.tensor_singlets
    }
    pub fn polynomial_singlets(&self) -> &BigInt {
        &self.polynomial_singlets
    }
    pub fn native_gauge_invariant(&self) -> bool {
        self.charge_sum.iter().all(Rational::is_zero) && !self.tensor_singlets.is_zero()
    }
    /// Holomorphic invariants give D-flat directions at zero FI; this is not a VEV solver.
    pub fn gauge_invariant(&self) -> bool {
        self.charge_sum.iter().all(Rational::is_zero) && !self.polynomial_singlets.is_zero()
    }
}
/// The legacy FI-compensating charge criterion, explicitly distinct from Q=0.
/// Only charge 0 may be nonzero and must oppose FI when demand=true.
pub fn legacy_fi_charge_test(charges: &[Rational], fi: &Rational, demand: bool) -> bool {
    charges.iter().enumerate().all(|(i, q)| {
        q.is_zero() || (i == 0 && !fi.is_zero() && (!demand || q * fi < Rational::ZERO))
    })
}

pub(super) struct Invariants<'a> {
    view: &'a SusySpectrum,
    algebras: Vec<CharacterAlgebra>,
    characters: BTreeMap<(usize, Weight), Character>,
    cached_weights: usize,
    limits: AnalysisLimits,
}
impl<'a> Invariants<'a> {
    pub fn new(view: &'a SusySpectrum, limits: AnalysisLimits) -> Result<Self, SusyError> {
        let algebras = view
            .spectrum()
            .basis()
            .root_system()
            .factors()
            .iter()
            .map(CharacterAlgebra::new)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            view,
            algebras,
            characters: BTreeMap::new(),
            cached_weights: 0,
            limits,
        })
    }
    fn character(&mut self, g: usize, w: &Weight) -> Result<Character, SusyError> {
        let key = (g, w.clone());
        if !self.characters.contains_key(&key) {
            let c = self.algebras[g].character(w, self.limits.character_weights)?;
            self.cached_weights = self
                .cached_weights
                .checked_add(c.len())
                .ok_or(SusyError::ResourceLimit)?;
            if self.cached_weights > self.limits.character_work {
                return Err(SusyError::ResourceLimit);
            }
            self.characters.insert(key.clone(), c);
        }
        Ok(self.characters[&key].clone())
    }
    fn singlets(&mut self, g: usize, labels: &[Weight]) -> Result<BigInt, SusyError> {
        let a = &self.algebras[g];
        match labels.len() {
            1 => Ok(BigInt::from(labels[0].iter().all(BigInt::is_zero))),
            2 => Ok(BigInt::from(
                a.dual(&labels[0], self.limits.character_work)? == labels[1],
            )),
            3 => {
                let dual = a.dual(&labels[2], self.limits.character_work)?;
                let c = self.character(g, &labels[1])?;
                let product = self.algebras[g].tensor_character(
                    &labels[0],
                    &c,
                    self.limits.character_work,
                )?;
                Ok(product.get(&dual).cloned().unwrap_or_default())
            }
            _ => Err(SusyError::UnsupportedOrder),
        }
    }
    pub fn analyze(&mut self, monomial: &Monomial) -> Result<InvariantReport, SusyError> {
        let ids = monomial.expanded();
        // Revalidate even a monomial obtained from another view.
        Monomial::new(self.view, &ids)?;
        let fields = self.view.spectrum().fields();
        let mut charge_sum =
            vec![Rational::ZERO; self.view.spectrum().basis().u1_generators().len()];
        for &i in &ids {
            for (q, x) in charge_sum.iter_mut().zip(fields[i].u1_charges()) {
                *q = &*q + x;
            }
        }
        let repeated = monomial.fields.iter().find(|(_, n)| *n > 1).copied();
        let mut tensor = BigInt::one();
        let mut transposition = BigInt::one();
        let mut cycle = BigInt::one();
        for g in 0..self.algebras.len() {
            let labels = ids
                .iter()
                .map(|&i| fields[i].representations()[g].dynkin_labels().to_vec())
                .collect::<Vec<_>>();
            tensor *= self.singlets(g, &labels)?;
            if let Some((i, n)) = repeated {
                let w = fields[i].representations()[g].dynkin_labels().to_vec();
                let character = self.character(g, &w)?;
                let zero = vec![BigInt::zero(); self.algebras[g].rank()];
                let spectator = if ids.len() == 2 {
                    zero.clone()
                } else {
                    let j = ids.iter().find(|&&j| j != i).copied().unwrap_or(i);
                    fields[j].representations()[g].dynkin_labels().to_vec()
                };
                let product = self.algebras[g].tensor_character(
                    &spectator,
                    &adams(&character, 2),
                    self.limits.character_work,
                )?;
                transposition *= product.get(&zero).cloned().unwrap_or_default();
                if n == 3 {
                    cycle *= self.algebras[g]
                        .singlet_coefficient(&adams(&character, 3), self.limits.character_work)?;
                }
            }
        }
        // Permutations act simultaneously on all gauge factors. Symmetrizing
        // each simple factor separately would incorrectly remove invariants.
        let polynomial = match repeated {
            Some((_, 3)) => {
                let n = &tensor + transposition * 3 + cycle * 2;
                if &n % 6 != BigInt::zero() {
                    return Err(SusyError::InvalidMonomial);
                }
                n / 6
            }
            Some(_) => {
                let n = &tensor + transposition;
                if &n % 2 != BigInt::zero() {
                    return Err(SusyError::InvalidMonomial);
                }
                n / 2
            }
            None => tensor.clone(),
        };
        if polynomial < BigInt::zero() || tensor < BigInt::zero() {
            return Err(SusyError::InvalidMonomial);
        }
        Ok(InvariantReport {
            monomial: monomial.clone(),
            charge_sum,
            tensor_singlets: tensor,
            polynomial_singlets: polynomial,
        })
    }
}
pub fn analyze_monomials(
    view: &SusySpectrum,
    monomials: &[Monomial],
    limits: AnalysisLimits,
) -> Result<Vec<InvariantReport>, SusyError> {
    if monomials.len() > limits.candidates {
        return Err(SusyError::ResourceLimit);
    }
    let mut engine = Invariants::new(view, limits)?;
    monomials.iter().map(|m| engine.analyze(m)).collect()
}
