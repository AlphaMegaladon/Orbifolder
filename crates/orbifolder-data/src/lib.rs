#![deny(unsafe_code)]
//! Exact in-memory transport and adapters. No physical calculation is implemented here.
mod compact;
mod input;
pub use compact::compact_spectrum;
mod output;
mod presets;
pub use input::*;
pub use output::*;
pub use presets::*;
