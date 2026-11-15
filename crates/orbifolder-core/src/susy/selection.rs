use super::{
    AnalysisLimits, InvariantReport, Monomial, SusyError, SusySpectrum, invariants::Invariants,
};
use crate::{
    analysis::Computation,
    geometry::{ElementSpec, SpaceGroup, SpaceGroupElement},
    math_core::{Rational, RightWeight},
    physics_engine::Field,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuleCharge {
    /// Native internal q_sh + (N - Nbar), in the scalar superfield picture.
    R(Box<[Rational; 4]>),
    /// Canonical (m,n,k,l1,...,l6), not the legacy eight-slot numbering.
    NonR(Box<[Rational; 9]>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscreteRule {
    order: u32,
    target: Rational,
    charge: RuleCharge,
}
impl DiscreteRule {
    pub fn new(order: u32, target: Rational, charge: RuleCharge) -> Result<Self, SusyError> {
        if order == 0 {
            return Err(SusyError::InvalidRule);
        }
        match &charge {
            RuleCharge::R(op) if !op[0].is_zero() => return Err(SusyError::InvalidRule),
            RuleCharge::NonR(_) if !target.is_zero() => return Err(SusyError::InvalidRule),
            _ => {}
        }
        Ok(Self {
            order,
            target,
            charge,
        })
    }
    pub fn order(&self) -> u32 {
        self.order
    }
    pub fn target(&self) -> &Rational {
        &self.target
    }
    pub fn operator(&self) -> &RuleCharge {
        &self.charge
    }
    fn element_charge(&self, g: &SpaceGroupElement) -> Result<Rational, SusyError> {
        let RuleCharge::NonR(op) = &self.charge else {
            return Err(SusyError::InvalidRule);
        };
        let factors = g.factors();
        let mut sum = Rational::ZERO;
        for i in 0..3 {
            sum = sum + &op[i] * Rational::from(i128::from(factors[i]));
        }
        for i in 0..6 {
            sum = sum + &op[i + 3] * &g.translation().coordinates()[i];
        }
        Ok(sum)
    }
    /// Non-R operators must descend to characters of the actual space group,
    /// including rotational conjugation and finite affine section carries.
    pub fn validate_space_group(&self, space: &SpaceGroup, cap: usize) -> Result<(), SusyError> {
        let RuleCharge::NonR(op) = &self.charge else {
            return Ok(());
        };
        let modulus = Rational::from(i128::from(self.order));
        for generator in space.generators() {
            for j in 0..6 {
                let mut diff = Rational::ZERO;
                for i in 0..6 {
                    diff = diff
                        + &op[i + 3]
                            * (generator.rotation.get(i, j).unwrap()
                                - Rational::from(i128::from(i == j)));
                }
                if !diff.checked_div(&modulus)?.is_integer() {
                    return Err(SusyError::InvalidRule);
                }
            }
        }
        let mut work = 0_usize;
        for &p in space.points() {
            let g = space.element(ElementSpec {
                factors: p,
                translation: space.section_translation(p)?.clone(),
            })?;
            for i in 0..3 {
                if space.orders()[i] == 1 {
                    continue;
                }
                work = work.checked_add(1).ok_or(SusyError::ResourceLimit)?;
                if work > cap {
                    return Err(SusyError::ResourceLimit);
                }
                let mut f = [0; 3];
                f[i] = 1;
                let h = space.element(ElementSpec {
                    factors: f,
                    translation: space.section_translation(f)?.clone(),
                })?;
                let difference = self.element_charge(&space.multiply(&g, &h)?)?
                    - self.element_charge(&g)?
                    - self.element_charge(&h)?;
                if !difference.checked_div(&modulus)?.is_integer() {
                    return Err(SusyError::InvalidRule);
                }
            }
        }
        Ok(())
    }
    pub fn field_charge(&self, field: &Field) -> Result<Rational, SusyError> {
        match &self.charge {
            RuleCharge::R(op) => {
                let q = field
                    .scalar_charge()
                    .ok_or(SusyError::InvalidField)?
                    .add(field.oscillator_contribution());
                Ok(op
                    .iter()
                    .zip(q.coordinates())
                    .fold(Rational::ZERO, |s, (a, b)| s + a * b))
            }
            RuleCharge::NonR(_) => self.element_charge(field.element()),
        }
    }
    pub fn accepts(&self, sum: &Rational) -> Result<bool, SusyError> {
        Ok((sum - &self.target)
            .checked_div(&Rational::from(i128::from(self.order)))?
            .is_integer())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GammaPolicy {
    /// C++ stores these phases, but its coupling sum check is commented out.
    #[default]
    StoredNotApplied,
}
#[derive(Clone, Debug, Default)]
pub struct SelectionRules {
    pub discrete: Vec<DiscreteRule>,
    pub gamma: GammaPolicy,
}
impl SelectionRules {
    /// Explicit internal R_i=-(q_i + oscillator_i), target 1 modulo N_i.
    /// Orders must come from the chosen symmetry contract, not inferred flags.
    pub fn internal_r(orders: [u32; 3]) -> Result<Vec<DiscreteRule>, SusyError> {
        orders
            .into_iter()
            .enumerate()
            .map(|(i, n)| {
                let mut op = std::array::from_fn(|_| Rational::ZERO);
                op[i + 1] = Rational::from(-1);
                DiscreteRule::new(n, Rational::ONE, RuleCharge::R(Box::new(op)))
            })
            .collect()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleCheck {
    rule: DiscreteRule,
    charge_sum: Rational,
    passed: bool,
}
impl RuleCheck {
    pub fn rule(&self) -> &DiscreteRule {
        &self.rule
    }
    pub fn charge_sum(&self) -> &Rational {
        &self.charge_sum
    }
    pub fn passed(&self) -> bool {
        self.passed
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CouplingReport {
    invariant: InvariantReport,
    point_group: bool,
    space_group: bool,
    /// Extra legacy filter for a common fixed torus without an untwisted field.
    native_fixed_torus: bool,
    discrete: Vec<RuleCheck>,
    gamma_sums: Vec<Rational>,
    gamma: Computation<bool>,
}
impl CouplingReport {
    pub fn invariant(&self) -> &InvariantReport {
        &self.invariant
    }
    pub fn point_group(&self) -> bool {
        self.point_group
    }
    pub fn space_group(&self) -> bool {
        self.space_group
    }
    pub fn native_fixed_torus(&self) -> bool {
        self.native_fixed_torus
    }
    pub fn discrete(&self) -> &[RuleCheck] {
        &self.discrete
    }
    pub fn gamma_sums(&self) -> &[Rational] {
        &self.gamma_sums
    }
    pub fn gamma(&self) -> &Computation<bool> {
        &self.gamma
    }
    pub fn native_precoupling(&self) -> bool {
        self.invariant.native_gauge_invariant() && self.discrete.iter().all(|c| c.passed)
    }
    /// Native candidate semantics: tensor singlet and configured symmetries.
    pub fn native_allowed(&self) -> bool {
        self.invariant.monomial.degree() >= 3
            && self.native_precoupling()
            && self.point_group
            && self.space_group
            && self.native_fixed_torus
    }
    /// Polynomial invariant plus the configured exact selection rules.
    /// No numerical coupling strength or unconfigured symmetry is inferred.
    pub fn polynomial_allowed(&self) -> bool {
        self.invariant.gauge_invariant()
            && self.discrete.iter().all(|c| c.passed)
            && self.point_group
            && self.space_group
    }
}

pub fn analyze_couplings(
    view: &SusySpectrum,
    monomials: &[Monomial],
    rules: &SelectionRules,
    limits: AnalysisLimits,
) -> Result<Vec<CouplingReport>, SusyError> {
    if monomials.len() > limits.candidates {
        return Err(SusyError::ResourceLimit);
    }
    let mut engine = Invariants::new(view, limits)?;
    let space = view.model().geometry().space_group();
    for rule in &rules.discrete {
        rule.validate_space_group(space, limits.space_conjugations)?;
    }
    let fields = view.spectrum().fields();
    let mut reports = Vec::with_capacity(monomials.len());
    for monomial in monomials {
        let invariant = engine.analyze(monomial)?;
        let ids = monomial.expanded();
        let elements = ids
            .iter()
            .map(|&i| fields[i].element().clone())
            .collect::<Vec<_>>();
        let orders = space.orders();
        let mut powers = [0_u64; 3];
        for g in &elements {
            for i in 0..3 {
                powers[i] = (powers[i] + u64::from(g.factors()[i])) % u64::from(orders[i]);
            }
        }
        let point_group = powers == [0; 3];
        let space_group =
            space.conjugacy_product_contains_identity(&elements, limits.space_conjugations)?;
        let untwisted = elements.iter().any(|g| g.factors() == [0; 3]);
        let twists = elements
            .iter()
            .map(|g| {
                g.factors()
                    .iter()
                    .zip(space.generators())
                    .fold(RightWeight::zero(), |s, (&p, g)| {
                        s.add(&g.twist.scale(&Rational::from(i128::from(p))))
                    })
            })
            .collect::<Vec<_>>();
        let native_fixed_torus =
            untwisted || !(1..4).any(|i| twists.iter().all(|t| t.coordinates()[i].is_integer()));
        let mut discrete = Vec::new();
        for rule in &rules.discrete {
            let mut charge_sum = Rational::ZERO;
            for &i in &ids {
                charge_sum = charge_sum + rule.field_charge(&fields[i])?;
            }
            let passed = rule.accepts(&charge_sum)?;
            discrete.push(RuleCheck {
                rule: rule.clone(),
                charge_sum,
                passed,
            });
        }
        let mut gamma_sums = vec![Rational::ZERO; fields[ids[0]].gamma().len()];
        for &i in &ids {
            if fields[i].gamma().len() != gamma_sums.len() {
                return Err(SusyError::InvalidField);
            }
            for (sum, q) in gamma_sums.iter_mut().zip(fields[i].gamma()) {
                *sum = &*sum + q;
            }
        }
        reports.push(CouplingReport {
            invariant,
            point_group,
            space_group,
            native_fixed_torus,
            discrete,
            gamma_sums,
            gamma: Computation::NotApplicable(
                "Native Gamma/W coupling constraint is stored but not applied",
            ),
        });
    }
    Ok(reports)
}

/// Symbolic superpotential: each admitted monomial has an independent coefficient
/// label. This proves selection only; it does not assign a nonzero string amplitude.
#[derive(Clone, Debug)]
pub struct Superpotential {
    candidates: Vec<CouplingReport>,
    max_degree: u32,
    field_pool: Vec<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FTerm {
    coefficient_label: usize,
    prefactor: u32,
    powers: Vec<(usize, u32)>,
}
impl FTerm {
    pub fn coefficient_label(&self) -> usize {
        self.coefficient_label
    }
    pub fn prefactor(&self) -> u32 {
        self.prefactor
    }
    pub fn powers(&self) -> &[(usize, u32)] {
        &self.powers
    }
}
impl Superpotential {
    pub fn candidates(&self) -> &[CouplingReport] {
        &self.candidates
    }
    pub fn max_degree(&self) -> u32 {
        self.max_degree
    }
    pub fn field_pool(&self) -> &[usize] {
        &self.field_pool
    }
    pub fn allowed(&self) -> impl Iterator<Item = (usize, &Monomial)> {
        self.candidates
            .iter()
            .enumerate()
            .filter(|(_, r)| r.polynomial_allowed())
            .map(|(i, r)| (i, &r.invariant.monomial))
    }
    /// Formal derivative of the symbolic polynomial (F_i); zero terms are absent.
    /// Tuples are coefficient label, integral prefactor and remaining exponents.
    pub fn f_terms(&self, field: usize) -> Vec<FTerm> {
        self.allowed()
            .filter_map(|(label, m)| {
                let power = m.fields().iter().find(|&&(i, _)| i == field)?.1;
                let rest = m
                    .fields()
                    .iter()
                    .filter_map(|&(i, n)| {
                        let n = if i == field { n - 1 } else { n };
                        (n > 0).then_some((i, n))
                    })
                    .collect();
                Some(FTerm {
                    coefficient_label: label,
                    prefactor: power,
                    powers: rest,
                })
            })
            .collect()
    }
}
pub fn enumerate_superpotential(
    view: &SusySpectrum,
    pool: &[usize],
    max_degree: u32,
    rules: &SelectionRules,
    limits: AnalysisLimits,
) -> Result<Superpotential, SusyError> {
    if !(1..=3).contains(&max_degree) {
        return Err(SusyError::UnsupportedOrder);
    }
    let sorted = pool.iter().copied().collect::<BTreeSet<_>>();
    if sorted.len() != pool.len() {
        return Err(SusyError::InvalidField);
    }
    for &i in pool {
        Monomial::new(view, &[i])?;
    }
    let pool = sorted.into_iter().collect::<Vec<_>>();
    let mut monomials = Vec::new();
    for (a, &i) in pool.iter().enumerate() {
        monomials.push(Monomial::new(view, &[i])?);
        if monomials.len() > limits.candidates {
            return Err(SusyError::ResourceLimit);
        }
        if max_degree >= 2 {
            for (b, &j) in pool.iter().enumerate().skip(a) {
                monomials.push(Monomial::new(view, &[i, j])?);
                if monomials.len() > limits.candidates {
                    return Err(SusyError::ResourceLimit);
                }
                if max_degree == 3 {
                    for &k in &pool[b..] {
                        if monomials.len() == limits.candidates {
                            return Err(SusyError::ResourceLimit);
                        }
                        monomials.push(Monomial::new(view, &[i, j, k])?);
                    }
                }
            }
        }
    }
    Ok(Superpotential {
        candidates: analyze_couplings(view, &monomials, rules, limits)?,
        max_degree,
        field_pool: pool,
    })
}
