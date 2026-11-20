//! SUSY-only invariant polynomials and coupling selection, through verified cubic order.
mod invariants;
mod selection;
use crate::{
    analysis::Computation,
    geometry::{ModelError, OrbifoldGroup, TheorySpec},
    math_core::MathError,
    physics_engine::{
        PhysicsError, SolvedSectors, SolverLimits, Spectrum, TachyonSearch, calculate_spectrum,
        solve_sectors,
    },
};
pub use invariants::{InvariantReport, Monomial, analyze_monomials, legacy_fi_charge_test};
pub use selection::{
    CouplingReport, DiscreteRule, FTerm, GammaPolicy, RuleCharge, RuleCheck, SelectionRules,
    Superpotential, analyze_couplings, enumerate_superpotential,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SusyError {
    Math(MathError),
    Model(ModelError),
    Physics(PhysicsError),
    InvalidField,
    InvalidMonomial,
    InvalidRule,
    ResourceLimit,
    UnsupportedOrder,
}
impl From<MathError> for SusyError {
    fn from(e: MathError) -> Self {
        Self::Math(e)
    }
}
impl From<ModelError> for SusyError {
    fn from(e: ModelError) -> Self {
        Self::Model(e)
    }
}
impl From<PhysicsError> for SusyError {
    fn from(e: PhysicsError) -> Self {
        Self::Physics(e)
    }
}
impl std::fmt::Display for SusyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SusyError {}

#[derive(Clone, Copy, Debug)]
pub struct AnalysisLimits {
    pub character_weights: usize,
    pub character_work: usize,
    pub space_conjugations: usize,
    pub candidates: usize,
}
impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            character_weights: 100_000,
            character_work: 1_000_000,
            space_conjugations: 1_000_000,
            candidates: 100_000,
        }
    }
}

/// Construction proves the theory and preserved supercharges. The owned spectrum
/// is computed from that very model; unrelated spectra cannot be attached.
#[derive(Clone, Debug)]
pub struct SusySpectrum {
    model: OrbifoldGroup,
    spectrum: Spectrum,
}
impl SusySpectrum {
    pub fn compute(
        model: &OrbifoldGroup,
        limits: SolverLimits,
    ) -> Result<Computation<Self>, SusyError> {
        if model.theory() != TheorySpec::Susy {
            return Ok(Computation::NotApplicable(
                "Non-SUSY models have no chiral-superfield superpotential",
            ));
        }
        if model.supercharges().is_empty() {
            return Ok(Computation::NotApplicable(
                "No invariant spacetime supercharge",
            ));
        }
        Self::from_solved(
            &solve_sectors(model, TachyonSearch::Complete, limits)?,
            limits,
        )
    }
    pub fn from_solved(
        solved: &SolvedSectors,
        limits: SolverLimits,
    ) -> Result<Computation<Self>, SusyError> {
        if !Self::applicable(solved) {
            return Ok(Self::unavailable());
        }
        Ok(Self::from_spectrum(
            solved,
            &calculate_spectrum(solved, limits)?,
        ))
    }
    fn applicable(solved: &SolvedSectors) -> bool {
        let model = solved.model();
        model.theory() == TheorySpec::Susy && !model.supercharges().is_empty()
    }
    fn unavailable() -> Computation<Self> {
        Computation::NotApplicable(
            "Supersymmetric analysis requires preserved supercharges and the SUSY theory",
        )
    }
    pub(crate) fn from_spectrum(solved: &SolvedSectors, spectrum: &Spectrum) -> Computation<Self> {
        #[cfg(test)]
        crate::execution::tests::SUSY_BUILDS.with(|count| count.set(count.get() + 1));
        if !Self::applicable(solved) {
            return Self::unavailable();
        }
        Computation::Computed(Self {
            model: solved.model().clone(),
            spectrum: spectrum.clone(),
        })
    }
    pub fn model(&self) -> &OrbifoldGroup {
        &self.model
    }
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }
}
