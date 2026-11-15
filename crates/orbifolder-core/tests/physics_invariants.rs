use num_bigint::BigInt;
use orbifolder_core::{
    geometry::TheorySpec,
    math_core::{DnClass, GaugeLattice, GaugeVector, Rational, RightWeight, RootSystem, so8_class},
    physics_engine::{
        Mover, PhysicsError, SectorModes, ShellLattice, SolverLimits, VacuumEnergy,
        enumerate_shell, representation_dimension,
    },
};
use proptest::prelude::*;
use std::collections::BTreeSet;

fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).unwrap()
}
fn twist() -> RightWeight {
    RightWeight::new([r(0, 1), r(1, 3), r(1, 3), r(-2, 3)])
}

#[test]
fn vacuum_energies_use_fractional_parts_including_negative_lifts() {
    let e = VacuumEnergy::from_twist(&twist());
    assert_eq!(e.left(), &r(-2, 3));
    assert_eq!(e.right(), &r(-1, 6));
    let lifted = twist().add(&RightWeight::from_integers([0, -3, 20, 1]));
    assert_eq!(e, VacuumEnergy::from_twist(&lifted));
    let untwisted = VacuumEnergy::from_twist(&RightWeight::zero());
    assert_eq!(untwisted.left(), &r(-1, 1));
    assert_eq!(untwisted.right(), &r(-1, 2));
}

#[test]
fn fock_states_include_repetitions_and_exact_budget_boundary() {
    let modes = SectorModes::new(&twist(), &[twist()], [1, 0, 0], TheorySpec::Susy);
    let excitations = modes
        .excitations(Mover::Left, &Rational::ZERO, SolverLimits::default())
        .unwrap();
    let actual = excitations
        .iter()
        .map(|e| e.indices().to_vec())
        .collect::<BTreeSet<_>>();
    let mut expected = BTreeSet::from([Vec::new()]);
    for i in 0..modes.left().len() {
        if modes.left()[i].energy() <= r(2, 3) {
            expected.insert(vec![i]);
        }
        for j in i..modes.left().len() {
            if modes.left()[i].energy() + modes.left()[j].energy() <= r(2, 3) {
                expected.insert(vec![i, j]);
            }
        }
    }
    assert_eq!(actual, expected);
    assert!(
        excitations
            .iter()
            .any(|e| e.indices().len() == 2 && e.indices()[0] == e.indices()[1])
    );
    assert!(excitations.iter().any(|e| e.target().is_zero()));
    for e in excitations {
        let energy = e
            .indices()
            .iter()
            .fold(Rational::ZERO, |s, &i| s + modes.left()[i].energy());
        assert_eq!(&energy, e.energy());
        assert_eq!(e.target(), &(r(4, 3) - Rational::from(2) * energy));
        assert_eq!(e.is_excited(), !e.indices().is_empty());
    }
}

#[test]
fn shells_recover_all_parent_roots_and_reject_nearby_float_shells() {
    for lattice in [
        GaugeLattice::E8xE8,
        GaugeLattice::D16Plus,
        GaugeLattice::D8xD8,
    ] {
        let roots = enumerate_shell(
            &GaugeVector::zero(),
            &Rational::from(2),
            ShellLattice::Gauge(lattice),
            SolverLimits::default(),
        )
        .unwrap();
        assert_eq!(roots, lattice.roots());
        assert!(
            enumerate_shell(
                &GaugeVector::zero(),
                &r(2_000_001, 1_000_000),
                ShellLattice::Gauge(lattice),
                SolverLimits::default()
            )
            .unwrap()
            .is_empty()
        );
    }
}

#[test]
fn enormous_integral_lattice_shifts_do_not_limit_the_exact_search() {
    let integer = BigInt::from(10).pow(55);
    let mut coordinates = std::array::from_fn(|_| Rational::ZERO);
    coordinates[0] = Rational::from(integer.clone());
    coordinates[1] = Rational::from(integer);
    let shift = GaugeVector::new(coordinates);
    assert!(shift.coordinates()[0].uses_bigint());
    let roots = enumerate_shell(
        &shift,
        &Rational::from(2),
        ShellLattice::Gauge(GaugeLattice::E8xE8),
        SolverLimits::default(),
    )
    .unwrap();
    assert_eq!(roots, GaugeLattice::E8xE8.roots());
}

#[test]
fn weyl_dimension_formula_has_no_small_representation_table() {
    let alpha = GaugeVector::from_integers([1, -1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let system = RootSystem::new(
        vec![alpha.clone(), alpha.scale(&Rational::from(-1))],
        &alpha,
    )
    .unwrap();
    let n = BigInt::from(10).pow(45);
    let highest = alpha.scale(
        &Rational::from(n.clone())
            .checked_div(&Rational::from(2))
            .unwrap(),
    );
    assert_eq!(
        representation_dimension(&system.factors()[0], &highest).unwrap(),
        n + 1
    );
    assert!(
        representation_dimension(&system.factors()[0], &highest.scale(&Rational::from(-1)))
            .is_err()
    );
}

#[test]
fn exhausted_budgets_are_errors_and_result_types_are_thread_safe() {
    let limits = SolverLimits {
        search_nodes: 0,
        solutions: 0,
    };
    assert_eq!(
        enumerate_shell(
            &RightWeight::zero(),
            &Rational::ONE,
            ShellLattice::RightSo8,
            limits
        ),
        Err(PhysicsError::ResourceLimit)
    );
    let modes = SectorModes::new(&twist(), &[twist()], [1, 0, 0], TheorySpec::Susy);
    assert_eq!(
        modes.excitations(Mover::Left, &Rational::ZERO, limits),
        Err(PhysicsError::ResourceLimit)
    );
    assert!(
        enumerate_shell(
            &RightWeight::zero(),
            &Rational::ONE,
            ShellLattice::Gauge(GaugeLattice::E8xE8),
            SolverLimits::default()
        )
        .is_err()
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<orbifolder_core::physics_engine::SolvedSectors>();
    send_sync::<orbifolder_core::physics_engine::Spectrum>();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn translated_so8_shell_equals_independent_cartesian_search(entries in prop::array::uniform4(-2_i32..=2), target in 0_i32..=12) {
        let shift = RightWeight::new(entries.map(|n|r(i128::from(n),3)));
        let target = r(i128::from(target),6);
        let actual = enumerate_shell(&shift,&target,ShellLattice::RightSo8,SolverLimits::default()).unwrap().into_iter().collect::<BTreeSet<_>>();
        let mut expected = BTreeSet::new();
        for half in [false,true] {
            for a in -3..=3 {for b in -3..=3 {for c in -3..=3 {for d in -3..=3 {
                let p = RightWeight::new([a,b,c,d].map(|n|r(2*n+i128::from(half),2)));
                if matches!(so8_class(&p),Some(DnClass::Vector|DnClass::Spinor)) {
                    let shifted = p.add(&shift);
                    if shifted.norm_squared()==target {expected.insert(shifted);}
                }
            }}}}
        }
        prop_assert_eq!(actual,expected);
    }
}
