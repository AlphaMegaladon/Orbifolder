use super::*;
use crate::{
    anomalies::AnomalyCheck,
    physics_engine::{Multiplet, SusyMultiplet},
};
use std::cell::Cell;

#[path = "../../tests/common/mod.rs"]
mod common;
#[path = "../../tests/common/geometry_input.rs"]
mod geometry_input;

thread_local! {
    pub(crate) static CANONICAL_BUILDS: Cell<usize> = const { Cell::new(0) };
    pub(crate) static SUSY_BUILDS: Cell<usize> = const { Cell::new(0) };
}

#[test]
fn anomaly_filter_stops_before_canonical_spectrum_and_susy() {
    let fixture = common::json("fixtures/susy_z3.json");
    let geometry = Geometry::build(geometry_input::from_fixture(&fixture)).unwrap();
    let mut request = ModelRequest {
        geometry: Arc::new(geometry),
        embedding: geometry_input::embedding(
            "susy",
            fixture["input"]["model_text"].as_str().unwrap(),
        ),
        theory: TheorySpec::Susy,
        options: ComputeOptions::default(),
        coupling_rules: SelectionRules::default(),
    };

    // The SUSY Z3 oracle's removed chiral field is one of three identical
    // untwisted representations at this source. Removing one spoils SU(3)^3.
    let without_one_chiral_field = |solved: &SolvedSectors, limits| {
        let spectrum = calculate_spectrum(solved, limits)?;
        let mut removed = false;
        let reduced = spectrum.select_fields(|field| {
            if !removed
                && field.source() == [0, 0, 1]
                && field.multiplet() == Multiplet::Susy(SusyMultiplet::LeftChiral)
            {
                removed = true;
                return false;
            }
            true
        });
        assert!(removed);
        Ok(reduced)
    };
    let run = |request: &ModelRequest| {
        evaluate_with_spectrum(request, without_one_chiral_field)
            .unwrap_or_else(|failure| panic!("{}: {}", failure.provenance.stage, failure.error))
    };
    let counts = || (CANONICAL_BUILDS.get(), SUSY_BUILDS.get());
    let before = counts();
    let Outcome::Complete(full) = run(&request) else {
        panic!("the default scan must retain the anomalous spectrum");
    };
    let report = full.anomalies().unwrap().report();
    assert!(report.checks().iter().any(|check| matches!(
        check.kind(),
        AnomalyCheck::GaugeCubic { .. }
    ) && check.status()
        == &Computation::Computed(false)));
    let after_full = counts();
    assert_eq!(after_full, (before.0 + 1, before.1 + 1));

    request.options.stop_on_anomaly = true;
    let Outcome::Filtered(hit) = run(&request) else {
        panic!("the failed cubic anomaly must trigger the filter");
    };
    assert_eq!(hit.reason, FilterReason::Anomaly);
    assert_eq!(hit.anomalies.unwrap().checks(), report.checks());
    assert_eq!(hit.provenance.anomalies, Some(false));
    assert_eq!(hit.provenance.stage, "anomalies");
    assert!(hit.provenance.massless_sectors_solved && hit.provenance.representations_assembled);
    assert!(!hit.provenance.canonical_spectrum_assembled && !hit.provenance.susy_view_computed);
    assert_eq!(counts(), after_full);
}
