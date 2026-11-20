#![deny(unsafe_code)]
//! Exact, immutable orbifold mathematics, geometry and state calculations.

// Allow unit tests to reuse the integration tests' fixture readers.
#[cfg(test)]
extern crate self as orbifolder_core;

pub mod analysis;
pub mod anomalies;
pub mod execution;
pub mod geometry;
pub mod math_core;
pub mod physics_engine;
pub mod susy;
