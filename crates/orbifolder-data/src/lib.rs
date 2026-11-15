#![deny(unsafe_code)]
//! Exact in-memory transport and adapters. No physical calculation is implemented here.
mod input;
mod output;
mod presets;
pub use input::*;
pub use output::*;
pub use presets::*;
