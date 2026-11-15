use num_bigint::BigInt;
use orbifolder_core::{
    anomalies::{
        AnomalyCheck, AnomalyOptions, Computation, Handedness, MatterField, analyze_matter,
        representation_indices,
    },
    geometry::TheorySpec,
    math_core::{
        GaugeLattice, GaugeVector, Rational, RootSystem, Vector, dynkin_labels, highest_weight,
        root_closure,
    },
};
mod common;

fn unit(i: usize) -> GaugeVector {
    Vector::from_integers(std::array::from_fn(|j| i128::from(i == j)))
}
fn groups(su2: bool, two: bool) -> RootSystem<16> {
    let mut simple = vec![unit(0).sub(&unit(1))];
    if !su2 {
        simple.push(unit(1).sub(&unit(2)));
    }
    if two {
        simple.push(unit(3).sub(&unit(4)));
        simple.push(unit(4).sub(&unit(5)));
    }
    let chamber = GaugeVector::from_integers([6, 5, 4, 3, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    RootSystem::new(
        root_closure(
            &simple,
            if two {
                12
            } else if su2 {
                2
            } else {
                6
            },
        )
        .unwrap(),
        &chamber,
    )
    .unwrap()
}
fn matter(labels: &[&[i32]], q: &[i32], hand: Handedness) -> MatterField {
    MatterField::new(
        labels
            .iter()
            .map(|l| l.iter().map(|x| BigInt::from(*x)).collect())
            .collect(),
        q.iter().map(|x| Rational::from(*x)).collect(),
        hand,
        BigInt::from(1),
    )
    .unwrap()
}
fn failed(
    report: &orbifolder_core::anomalies::AnomalyReport,
    kind: impl Fn(&AnomalyCheck) -> bool,
) -> bool {
    report
        .checks()
        .iter()
        .any(|c| kind(c.kind()) && matches!(c.status(), Computation::Computed(false)))
}

#[test]
fn isolated_su3_cubic_anomaly_and_handedness_cancellation() {
    let system = groups(false, false);
    let left = matter(&[&[1, 0]], &[], Handedness::Left);
    let report = analyze_matter(
        &system,
        &[],
        std::slice::from_ref(&left),
        TheorySpec::Susy,
        Default::default(),
    )
    .unwrap();
    assert!(!report.is_anomaly_free());
    assert!(failed(&report, |c| matches!(
        c,
        AnomalyCheck::GaugeCubic { .. }
    )));
    assert_eq!(report.raw().gauge_cubic()[0].abs(), Rational::ONE);
    let right = matter(&[&[1, 0]], &[], Handedness::Right);
    assert!(
        analyze_matter(
            &system,
            &[],
            &[left, right],
            TheorySpec::NonSusy,
            Default::default()
        )
        .unwrap()
        .is_anomaly_free()
    );
    let paired = [
        matter(&[&[1, 0]], &[], Handedness::Left),
        matter(&[&[0, 1]], &[], Handedness::Left),
    ];
    assert!(
        analyze_matter(&system, &[], &paired, TheorySpec::Susy, Default::default())
            .unwrap()
            .is_anomaly_free()
    );
}

#[test]
fn unequal_gauge_factor_gs_ratios_are_reported_individually() {
    let system = groups(false, true);
    let fields = [
        matter(&[&[1, 0], &[0, 0]], &[1], Handedness::Left),
        matter(&[&[0, 1], &[0, 0]], &[1], Handedness::Left),
        matter(&[&[0, 0], &[1, 0]], &[2], Handedness::Left),
        matter(&[&[0, 0], &[0, 1]], &[2], Handedness::Left),
    ];
    let report = analyze_matter(
        &system,
        &[unit(14)],
        &fields,
        TheorySpec::Susy,
        Default::default(),
    )
    .unwrap();
    assert!(report.raw().gauge_cubic().iter().all(Rational::is_zero));
    assert_ne!(
        report.canonical().mixed()[0][0],
        report.canonical().mixed()[0][1]
    );
    assert!(!report.is_anomaly_free());
    for g in 0..2 {
        assert!(failed(
            &report,
            |c| matches!(c,AnomalyCheck::MixedUniversal{u1:0,factor} if *factor==g)
        ));
    }
}

#[test]
fn second_independent_mixed_anomaly_survives_traceless_rotation() {
    let system = groups(false, true);
    let fields = [
        matter(&[&[1, 0], &[0, 0]], &[1, 1], Handedness::Left),
        matter(&[&[0, 1], &[0, 0]], &[1, 1], Handedness::Left),
        matter(&[&[0, 0], &[1, 0]], &[1, -1], Handedness::Left),
        matter(&[&[0, 0], &[0, 1]], &[1, -1], Handedness::Left),
    ];
    let report = analyze_matter(
        &system,
        &[unit(14), unit(15)],
        &fields,
        TheorySpec::NonSusy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(report.canonical().gravitational()[1], Rational::ZERO);
    assert_eq!(
        report.canonical().independent_abelian_directions().unwrap(),
        2
    );
    assert!(failed(&report, |c| matches!(
        c,
        AnomalyCheck::IndependentAbelianDirections
    )));
    assert!(failed(&report, |c| matches!(
        c,
        AnomalyCheck::TracelessMixed { u1: 1, .. }
    )));
    assert!(!report.is_anomaly_free());
}

#[test]
fn single_gs_direction_allows_nonzero_symmetric_tensor_slices() {
    let system = RootSystem::new(Vec::new(), &GaugeVector::zero()).unwrap();
    let a = unit(14).scale(&Rational::from(2));
    let b = unit(11)
        .add(&unit(12))
        .add(&unit(13))
        .scale(&Rational::from(2));
    let fields = [
        matter(&[], &[1, 1], Handedness::Left),
        matter(&[], &[1, -1], Handedness::Left),
    ];
    let report = analyze_matter(
        &system,
        &[a, b],
        &fields,
        TheorySpec::Susy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        report.raw().abelian_cubic(0, 1, 1),
        Some(&Rational::from(2))
    );
    assert_eq!(
        report.raw().abelian_cubic(1, 0, 1),
        Some(&Rational::from(2))
    );
    assert_eq!(report.raw().independent_abelian_directions().unwrap(), 1);
    assert!(report.is_anomaly_free());
    assert_eq!(report.rotation().anomalous_index(), Some(0));
    assert!(matches!(
        report.fi().supersymmetric_coefficient(),
        Computation::Computed(_)
    ));
}

#[test]
fn pure_abelian_obstruction_with_zero_gravitational_trace_is_detected() {
    let system = RootSystem::new(Vec::new(), &GaugeVector::zero()).unwrap();
    let fields = [
        matter(&[], &[1], Handedness::Left),
        matter(&[], &[1], Handedness::Left),
        matter(&[], &[-2], Handedness::Left),
    ];
    let report = analyze_matter(
        &system,
        &[unit(14)],
        &fields,
        TheorySpec::NonSusy,
        Default::default(),
    )
    .unwrap();
    assert!(report.raw().gravitational()[0].is_zero());
    assert_eq!(report.rotation().anomalous_index(), None);
    assert!(failed(&report, |c| matches!(
        c,
        AnomalyCheck::AbelianUniversal { indices: [0, 0, 0] }
    )));
    assert!(!report.is_anomaly_free());
    assert!(matches!(
        report.fi().supersymmetric_coefficient(),
        Computation::NotApplicable(_)
    ));
}

#[test]
fn witten_anomaly_counts_all_isospins_and_both_handednesses() {
    let system = groups(true, false);
    for label in [1, 5] {
        let field = matter(&[&[label]], &[], Handedness::Right);
        let report = analyze_matter(
            &system,
            &[],
            &[field],
            TheorySpec::NonSusy,
            Default::default(),
        )
        .unwrap();
        assert!(failed(&report, |c| matches!(
            c,
            AnomalyCheck::Witten { factor: 0 }
        )));
    }
    let even = [
        matter(&[&[1]], &[], Handedness::Left),
        matter(&[&[1]], &[], Handedness::Right),
    ];
    let report = analyze_matter(&system, &[], &even, TheorySpec::Susy, Default::default()).unwrap();
    assert!(report.is_anomaly_free());
    assert_eq!(report.raw().su2_doublets()[0], Some(BigInt::from(2)));
}

#[test]
fn reject_invalid_levels_basis_and_matter_shapes() {
    let system = groups(false, false);
    let field = matter(&[&[1, 0]], &[1], Handedness::Left);
    assert!(
        analyze_matter(
            &system,
            &[unit(14)],
            std::slice::from_ref(&field),
            TheorySpec::Susy,
            AnomalyOptions {
                non_abelian_levels: Some(vec![Rational::ZERO])
            }
        )
        .is_err()
    );
    assert!(
        analyze_matter(
            &system,
            &[unit(0)],
            std::slice::from_ref(&field),
            TheorySpec::Susy,
            Default::default()
        )
        .is_err()
    );
    assert!(
        analyze_matter(
            &system,
            &[unit(14), unit(14)],
            std::slice::from_ref(&field),
            TheorySpec::Susy,
            Default::default()
        )
        .is_err()
    );
    let bad = matter(&[&[1]], &[1], Handedness::Left);
    assert!(
        analyze_matter(
            &system,
            &[unit(14)],
            &[bad],
            TheorySpec::Susy,
            Default::default()
        )
        .is_err()
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<orbifolder_core::anomalies::AnomalyReport>();
}

#[test]
fn exceptional_indices_match_native_tables_and_have_no_cubic_invariant() {
    let oracle = common::json("anomalies_oracle.json");
    let native = &oracle["cases"]["susy_z3"]["native"]["exceptional_quadratic"];
    let roots = GaugeLattice::E8xE8
        .roots()
        .into_iter()
        .filter(|r| r.coordinates()[8..].iter().all(Rational::is_zero))
        .collect::<Vec<_>>();
    let chamber = GaugeVector::from_integers([23, 6, 5, 4, 3, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let e8 = RootSystem::new(roots.clone(), &chamber).unwrap();
    let e7 = RootSystem::new(
        roots
            .into_iter()
            .filter(|r| r.dot(&unit(0).add(&unit(1))).is_zero())
            .collect(),
        &chamber,
    )
    .unwrap();
    for (system, name) in [(&e7, "E7_adj"), (&e8, "E8_adj")] {
        let f = &system.factors()[0];
        let highest = highest_weight(f.roots(), f.simple_roots()).unwrap();
        let indices =
            representation_indices(f, &dynkin_labels(&highest, f.simple_roots()).unwrap()).unwrap();
        assert_eq!(
            Rational::from(2) * indices.quadratic,
            common::number(&native[name], 1)
        );
        assert!(matches!(indices.cubic, Computation::NotApplicable(_)));
    }
    let f = &e7.factors()[0];
    let mut found = false;
    for i in 0..7 {
        let mut labels = vec![BigInt::from(0); 7];
        labels[i] = BigInt::from(1);
        let indices = representation_indices(f, &labels).unwrap();
        if indices.dimension == BigInt::from(56) {
            assert_eq!(
                Rational::from(2) * indices.quadratic,
                common::number(&native["E7_56"], 1)
            );
            found = true;
        }
    }
    assert!(found);
}
