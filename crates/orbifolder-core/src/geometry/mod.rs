//! Exact space groups and gauge embeddings; no state or mass enumeration.
mod character;
mod embedding;
mod integer;
mod space;
pub use embedding::{
    DiscreteTorsion, EmbeddingSpec, LocalElement, ModularCheck, OrbifoldGroup, TheorySpec,
    TorsionPolicy, embed, invariant_supercharges,
};
pub use space::{
    Centralizer, ElementSpec, FixedBrane, FixedLocus, GeneratorSpec, Geometry, GeometrySpec,
    Limits, Sector, SpaceGroup, SpaceGroupElement, WilsonRelations,
};

use crate::math_core::MathError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelError {
    Math(MathError),
    InvalidMetric,
    InvalidRotation,
    InvalidOrder,
    InvalidElement,
    InvalidRelations,
    InvalidPresentation,
    IncompleteConstructions,
    DuplicateConjugacyClass,
    NoFixedLocus,
    InvalidEmbeddingOrder,
    WilsonRelation,
    ModularInvariance {
        left: usize,
        right: usize,
        value: crate::math_core::Rational,
    },
    UnsupportedEmbedding,
    UnsupportedGaugeLattice,
    InvalidTorsion,
    UnsupportedTorsionProjection,
    ResourceLimit,
}
impl From<MathError> for ModelError {
    fn from(value: MathError) -> Self {
        Self::Math(value)
    }
}
impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ModelError {}
