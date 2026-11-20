//! Exact four-dimensional perturbative anomalies and Green--Schwarz factorization.
mod basis;
mod indices;
mod report;
mod traces;
use crate::math_core::MathError;
use crate::physics_engine::PhysicsError;
pub use basis::{BasisRotation, canonical_u1_basis};
pub use indices::{RepresentationIndices, representation_indices};
pub(crate) use report::analyze_spectrum_report;
pub use report::{
    AnomalyAnalysis, AnomalyCheck, AnomalyOptions, AnomalyReport, CheckResult, FiData,
    analyze_anomalies, analyze_matter,
};
pub use traces::{AnomalyTraces, Handedness, MatterField, trace_matter};

pub use crate::analysis::Computation;
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnomalyError {
    Math(MathError),
    Physics(PhysicsError),
    InvalidMatter,
    InvalidBasis,
    InvalidLevels,
    UnsupportedMultiplet,
}
impl From<MathError> for AnomalyError {
    fn from(e: MathError) -> Self {
        Self::Math(e)
    }
}
impl From<PhysicsError> for AnomalyError {
    fn from(e: PhysicsError) -> Self {
        Self::Physics(e)
    }
}
impl std::fmt::Display for AnomalyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for AnomalyError {}
