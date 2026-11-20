//! Exact sector energies, oscillator Fock states and lattice mass shells.
//! All caches and resource budgets belong to a single invocation.

mod oscillator;
mod shell;
mod spectrum;
mod states;
pub use oscillator::{Excitation, Mover, Oscillator, SectorModes, VacuumEnergy};
pub use shell::{MassShell, ShellLattice, enumerate_shell};
pub use spectrum::{
    Field, GaugeBasis, Multiplet, Particle, Representation, Spectrum, SusyMultiplet,
    TachyonicSpectrum, calculate_spectrum, representation_dimension, spectrum_in_basis,
};
pub use states::{
    BraneStates, HalfState, ProjectedState, SectorKinematics, SectorStates, SolvedSectors,
    TachyonSearch, solve_kinematics, solve_sectors,
};
pub(crate) use states::{finish_prefiltered, tachyon_prefilter};

use crate::{geometry::ModelError, math_core::MathError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicsError {
    Math(MathError),
    Model(ModelError),
    ResourceLimit,
    InvalidEnergy,
    InvalidRepresentation,
    UnsupportedSupermultiplet,
}
impl From<MathError> for PhysicsError {
    fn from(error: MathError) -> Self {
        Self::Math(error)
    }
}
impl From<ModelError> for PhysicsError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}
impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PhysicsError {}

/// Hard caps fail explicitly; no successful result is silently truncated.
#[derive(Clone, Copy, Debug)]
pub struct SolverLimits {
    pub search_nodes: usize,
    pub solutions: usize,
}
impl Default for SolverLimits {
    fn default() -> Self {
        Self {
            search_nodes: 10_000_000,
            solutions: 1_000_000,
        }
    }
}
