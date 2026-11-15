use num_bigint::BigInt;
use orbifolder_core::{
    geometry::{ElementSpec, GeneratorSpec, Limits, ModelError, SpaceGroup},
    math_core::{
        CharacterAlgebra, MathError, Matrix, Rational, RightWeight, RootSystem, TorusVector,
        Vector, adams, multiply_characters, root_closure,
    },
    susy::{DiscreteRule, RuleCharge, SusyError, legacy_fi_charge_test},
};
use proptest::prelude::*;

fn su2() -> CharacterAlgebra {
    let simple = [Vector::from_integers([1, -1])];
    let system = RootSystem::new(
        root_closure(&simple, 2).unwrap(),
        &Vector::from_integers([1, 0]),
    )
    .unwrap();
    CharacterAlgebra::new(&system.factors()[0]).unwrap()
}
fn su3() -> CharacterAlgebra {
    let simple = [
        Vector::from_integers([1, -1, 0]),
        Vector::from_integers([0, 1, -1]),
    ];
    let system = RootSystem::new(
        root_closure(&simple, 6).unwrap(),
        &Vector::from_integers([3, 1, 0]),
    )
    .unwrap();
    CharacterAlgebra::new(&system.factors()[0]).unwrap()
}
fn z3() -> SpaceGroup {
    let identity = GeneratorSpec {
        order: 1,
        rotation: Matrix::identity(6).unwrap(),
        translation: TorusVector::zero(),
        twist: RightWeight::zero(),
    };
    let rotation = Matrix::new(
        6,
        6,
        (0..36)
            .map(|n| {
                let i = n / 6;
                let j = n % 6;
                Rational::from(if i / 2 != j / 2 {
                    0
                } else {
                    [[0, -1], [1, -1]][i % 2][j % 2]
                })
            })
            .collect(),
    )
    .unwrap();
    let gram = Matrix::new(
        6,
        6,
        (0..36)
            .map(|n| {
                let i = n / 6;
                let j = n % 6;
                Rational::from(if i / 2 != j / 2 {
                    0
                } else {
                    if i == j { 2 } else { -1 }
                })
            })
            .collect(),
    )
    .unwrap();
    SpaceGroup::new(
        gram,
        [
            GeneratorSpec {
                order: 3,
                rotation,
                translation: TorusVector::zero(),
                twist: RightWeight::new([
                    Rational::ZERO,
                    Rational::new(1, 3).unwrap(),
                    Rational::new(1, 3).unwrap(),
                    Rational::new(-2, 3).unwrap(),
                ]),
            },
            identity.clone(),
            identity,
        ],
        Limits::default(),
    )
    .unwrap()
}

proptest! {
    #[test]
    fn su2_character_and_clebsch_gordan(a in 0_u32..10,b in 0_u32..10){
        let g=su2();let x=g.character(&[a.into()],1000).unwrap();
        prop_assert_eq!(x.values().sum::<BigInt>(),BigInt::from(a+1));
        let one = BigInt::from(1);
        for k in 0..=a{prop_assert_eq!(x.get(&vec![BigInt::from(a)-BigInt::from(2*k)]),Some(&one));}
        let p=g.tensor_product(&[a.into()],&[b.into()],1000).unwrap();
        let expected=(a.abs_diff(b)..=a+b).step_by(2).map(|n|(vec![BigInt::from(n)],BigInt::from(1))).collect();
        prop_assert_eq!(p,expected);
    }
}
#[test]
fn character_errors_and_big_integer_dimension() {
    let g = su2();
    let huge = BigInt::from(1) << 300_usize;
    assert_eq!(g.dimension(std::slice::from_ref(&huge)).unwrap(), &huge + 1);
    assert_eq!(g.character(&[huge], 2), Err(MathError::CapacityOverflow));
    assert_eq!(
        g.character(&[(-1).into()], 100),
        Err(MathError::InvalidRootSystem)
    );
    assert_eq!(g.character(&[], 100), Err(MathError::DimensionMismatch));
    assert_eq!(
        g.character(&[1.into()], 0),
        Err(MathError::CapacityOverflow)
    );
    let c = g.character(&[2.into()], 100).unwrap();
    assert_eq!(adams(&c, 0), [(vec![0.into()], 3.into())].into());
    assert_eq!(
        multiply_characters(&c, &c, 2),
        Err(MathError::CapacityOverflow)
    );
}
#[test]
fn dual_tensor_and_symmetric_cubic_invariants() {
    let g = su3();
    let fundamental = vec![1.into(), 0.into()];
    let dual = g.dual(&fundamental, 100).unwrap();
    assert_eq!(g.dual(&dual, 100).unwrap(), fundamental);
    let c = g.character(&fundamental, 100).unwrap();
    let pair = multiply_characters(&c, &c, 100).unwrap();
    let cube = multiply_characters(&pair, &c, 1000).unwrap();
    assert_eq!(g.singlet_coefficient(&cube, 1000).unwrap(), 1.into());
    // Independent explicit symmetric cube basis of the three fundamental weights.
    let basis = c.keys().collect::<Vec<_>>();
    let mut sym = std::collections::BTreeMap::new();
    for i in 0..3 {
        for j in i..3 {
            for k in j..3 {
                let w = (0..2)
                    .map(|a| &basis[i][a] + &basis[j][a] + &basis[k][a])
                    .collect();
                *sym.entry(w).or_insert(BigInt::from(0)) += 1;
            }
        }
    }
    assert_eq!(g.singlet_coefficient(&sym, 1000).unwrap(), 0.into());
    let adjoint = g.character(&[1.into(), 1.into()], 100).unwrap();
    let tensor = g
        .tensor_product(&[1.into(), 1.into()], &[1.into(), 1.into()], 1000)
        .unwrap();
    assert_eq!(tensor.get(&vec![1.into(), 1.into()]), Some(&2.into()));
    assert_eq!(adjoint.values().sum::<BigInt>(), 8.into());
}
#[test]
fn exact_fractional_r_and_fi_contracts() {
    let r = DiscreteRule::new(
        3,
        Rational::ONE,
        RuleCharge::R(Box::new([
            Rational::ZERO,
            Rational::from(-1),
            Rational::ZERO,
            Rational::ZERO,
        ])),
    )
    .unwrap();
    assert!(r.accepts(&Rational::from(-2)).unwrap());
    assert!(r.accepts(&Rational::from(4)).unwrap());
    assert!(!r.accepts(&Rational::new(1, 3).unwrap()).unwrap());
    assert!(!r.accepts(&Rational::ZERO).unwrap());
    assert!(
        DiscreteRule::new(
            0,
            Rational::ONE,
            RuleCharge::R(Box::new(std::array::from_fn(|_| Rational::ZERO)))
        )
        .is_err()
    );
    assert!(legacy_fi_charge_test(
        &[Rational::new(-1, 1_000_000).unwrap()],
        &Rational::ONE,
        true
    ));
    assert!(!legacy_fi_charge_test(
        &[Rational::new(1, 1_000_000).unwrap()],
        &Rational::ONE,
        true
    ));
    assert!(!legacy_fi_charge_test(
        &[Rational::ZERO, Rational::new(-1, 1_000_000).unwrap()],
        &Rational::ONE,
        false
    ));
}
#[test]
fn space_group_classes_and_selection_are_conjugation_invariant() {
    let space = z3();
    let a = space
        .element(ElementSpec {
            factors: [1, 0, 0],
            translation: TorusVector::zero(),
        })
        .unwrap();
    let b = space
        .element(ElementSpec {
            factors: [1, 0, 0],
            translation: TorusVector::from_integers([1, 0, 0, 0, 0, 0]),
        })
        .unwrap();
    assert!(
        !space
            .conjugacy_product_contains_identity(&[a.clone(), a.clone(), b.clone()], 1000)
            .unwrap()
    );
    let mut allowed = vec![b.clone(), b.clone(), b];
    assert!(
        space
            .conjugacy_product_contains_identity(&allowed, 1000)
            .unwrap()
    );
    let h = space
        .element(ElementSpec {
            factors: [2, 0, 0],
            translation: TorusVector::from_integers([17, -23, 5, 0, 0, 0]),
        })
        .unwrap();
    allowed[1] = space.conjugate(&h, &allowed[1]).unwrap();
    assert!(
        space
            .conjugacy_product_contains_identity(&allowed, 1000)
            .unwrap()
    );
    allowed.reverse();
    assert!(
        space
            .conjugacy_product_contains_identity(&allowed, 1000)
            .unwrap()
    );
    assert_eq!(
        space.conjugacy_product_contains_identity(&allowed, 0),
        Err(ModelError::ResourceLimit)
    );
    assert!(
        !space
            .conjugacy_product_contains_identity(&[a], 1000)
            .unwrap()
    );
    assert!(space.conjugacy_product_contains_identity(&[], 0).unwrap());
}
#[test]
fn non_r_operator_must_descend_to_conjugacy_classes() {
    let space = z3();
    let mut op = std::array::from_fn(|_| Rational::ZERO);
    op[3] = Rational::ONE;
    let r = DiscreteRule::new(3, Rational::ZERO, RuleCharge::NonR(Box::new(op))).unwrap();
    assert_eq!(
        r.validate_space_group(&space, 100),
        Err(SusyError::InvalidRule)
    );
    let mut op = std::array::from_fn(|_| Rational::ZERO);
    op[0] = Rational::ONE;
    let r = DiscreteRule::new(3, Rational::ZERO, RuleCharge::NonR(Box::new(op))).unwrap();
    assert!(r.validate_space_group(&space, 100).is_ok());
    assert_eq!(
        r.validate_space_group(&space, 0),
        Err(SusyError::ResourceLimit)
    );
}
