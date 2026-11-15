//! Rational arithmetic, linear algebra and Euclidean lattice/root data.

mod rational;
pub use rational::{Phase, Rational};
mod linear;
pub use linear::{
    GaugeVector, LinearSolution, Matrix, RightWeight, TorusVector, Vector, weyl_reflect,
};
mod lattice;
pub use lattice::{
    DnClass, GaugeLattice, GaugeWeightClass, dn_class, dn_roots, e8_roots, so8_class,
};
mod characters;
mod lie;
pub use characters::{Character, CharacterAlgebra, Weight, adams, multiply_characters};
pub use lie::{
    Algebra, LieFactor, RootSystem, cartan_matrix, dynkin_labels, fundamental_weights,
    highest_weight, root_closure,
};

/// A mathematical input or shape error; arithmetic never falls back to floats.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MathError {
    ZeroDenominator,
    DivisionByZero,
    InvalidRational,
    NonIntegral,
    DimensionMismatch,
    SingularMatrix,
    InconsistentSystem,
    InvalidMetric,
    InvalidRootSystem,
    IrregularChamber,
    CapacityOverflow,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MathError {}
