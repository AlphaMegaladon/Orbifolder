use orbifolder_core::math_core::{
    Algebra, GaugeLattice, MathError, Rational, RootSystem, Vector, cartan_matrix, dn_roots,
    dynkin_labels, e8_roots, fundamental_weights, root_closure, weyl_reflect,
};

fn chamber<const N: usize>() -> Vector<N> {
    let mut weight = Rational::ONE;
    let mut coordinates = std::array::from_fn(|_| Rational::ZERO);
    for i in (0..N).rev() {
        coordinates[i] = weight.clone();
        weight = weight * Rational::from(3);
    }
    Vector::new(coordinates)
}

#[test]
fn classify_and_reconstruct_parent_systems() {
    let e8 = RootSystem::new(e8_roots(), &chamber()).unwrap();
    assert_eq!(e8.factors()[0].algebra(), &Algebra::E8);
    assert_eq!(e8.rank(), 8);
    let d8 = RootSystem::new(dn_roots::<8>().unwrap(), &chamber()).unwrap();
    assert_eq!(d8.factors()[0].algebra(), &Algebra::D(8));
    let d16 = RootSystem::new(GaugeLattice::D16Plus.roots(), &chamber()).unwrap();
    assert_eq!(d16.factors()[0].algebra(), &Algebra::D(16));
    for factor in e8.factors().iter().chain(d8.factors()) {
        assert!(factor.cartan_matrix().unwrap().is_positive_definite());
        assert_eq!(
            root_closure(factor.simple_roots(), 240).unwrap(),
            factor.roots()
        );
        for root in factor.roots() {
            for alpha in factor.simple_roots() {
                assert!(factor.roots().contains(&weyl_reflect(root, alpha).unwrap()));
            }
        }
    }
}

#[test]
fn fundamental_weights_and_invalid_systems() {
    assert_eq!(
        Algebra::A(usize::MAX).root_count(),
        Err(MathError::CapacityOverflow)
    );
    assert_eq!(
        Algebra::D(0).root_count(),
        Err(MathError::InvalidRootSystem)
    );
    let roots = vec![
        Vector::from_integers([1, -1, 0]),
        Vector::from_integers([0, 1, -1]),
    ];
    let weights = fundamental_weights(&roots).unwrap();
    assert_eq!(
        dynkin_labels(&weights[0], &roots).unwrap(),
        vec![1.into(), 0.into()]
    );
    assert_eq!(
        dynkin_labels(&weights[1], &roots).unwrap(),
        vec![0.into(), 1.into()]
    );
    assert_eq!(
        cartan_matrix(&roots).unwrap().determinant().unwrap(),
        Rational::from(3)
    );
    assert_eq!(
        RootSystem::new(vec![roots[0].clone()], &chamber()),
        Err(MathError::InvalidRootSystem)
    );
    assert_eq!(
        RootSystem::new(dn_roots::<4>().unwrap(), &Vector::zero()),
        Err(MathError::IrregularChamber)
    );
    assert!(root_closure(&roots, 2).is_err());
    assert_eq!(
        RootSystem::new(Vec::<Vector<4>>::new(), &chamber())
            .unwrap()
            .rank(),
        0
    );
}

#[test]
fn e7_is_the_orthogonal_e8_subsystem() {
    let roots = e8_roots()
        .into_iter()
        .filter(|v| (&v.coordinates()[0] + &v.coordinates()[1]).is_zero())
        .collect();
    let e7 = RootSystem::new(roots, &chamber()).unwrap();
    assert_eq!(e7.factors()[0].algebra(), &Algebra::E7);
    assert_eq!(e7.roots().len(), 126);
}
