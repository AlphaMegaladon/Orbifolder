//! Shared applicability status for optional, explicitly scoped model analyses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Computation<T> {
    Computed(T),
    NotApplicable(&'static str),
    Unsupported(&'static str),
}
