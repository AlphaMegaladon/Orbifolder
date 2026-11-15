use orbifolder_core::math_core::{MathError, Matrix, Rational, Vector, weyl_reflect};
use proptest::prelude::*;

#[test]
fn big_rational_elimination_and_thread_safe_values() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Rational>();
    send_sync::<Vector<16>>();
    send_sync::<Matrix>();
    send_sync::<orbifolder_core::math_core::RootSystem<16>>();
    let large = Rational::from(i128::MAX) + Rational::ONE;
    let matrix = Matrix::from_rows(vec![
        vec![large.clone(), Rational::ONE],
        vec![Rational::ONE, large.clone()],
    ])
    .unwrap();
    assert_eq!(
        matrix.determinant().unwrap(),
        &large * &large - Rational::ONE
    );
    assert_eq!(
        matrix.multiply(&matrix.inverse().unwrap()).unwrap(),
        Matrix::identity(2).unwrap()
    );
}

fn matrix(rows: &[&[i128]]) -> Matrix {
    Matrix::from_rows(
        rows.iter()
            .map(|r| r.iter().copied().map(Rational::from).collect())
            .collect(),
    )
    .unwrap()
}

#[test]
fn rank_inverse_solve_and_nullspace() {
    let a = matrix(&[&[0, 2, 1], &[1, -2, -3], &[3, -1, 2]]);
    assert_eq!(a.determinant().unwrap(), Rational::from(-17));
    assert_eq!(
        a.multiply(&a.inverse().unwrap()).unwrap(),
        Matrix::identity(3).unwrap()
    );
    let rhs = vec![1.into(), 2.into(), 3.into()];
    let solution = a.solve(&rhs).unwrap();
    assert!(solution.nullspace().is_empty());
    assert_eq!(a.apply(solution.particular()).unwrap(), rhs);
    let singular = matrix(&[&[1, 2, 3], &[2, 4, 6]]);
    assert_eq!(singular.rank().unwrap(), 1);
    for v in singular.nullspace().unwrap() {
        assert!(singular.apply(&v).unwrap().iter().all(Rational::is_zero));
    }
    assert_eq!(
        singular.solve(&[1.into(), 3.into()]),
        Err(MathError::InconsistentSystem)
    );
    assert_eq!(
        matrix(&[&[1, 2], &[2, 4]]).inverse(),
        Err(MathError::SingularMatrix)
    );
}

#[test]
fn shapes_metrics_and_empty_linear_maps() {
    assert!(Vector::<4>::from_slice(&[Rational::ONE]).is_err());
    assert!(Matrix::from_rows(vec![vec![Rational::ONE], vec![]]).is_err());
    assert!(Matrix::zeros(usize::MAX, 2).is_err());
    assert_eq!(Matrix::zeros(0, 3).unwrap().nullspace().unwrap().len(), 3);
    assert_eq!(
        Matrix::identity(0).unwrap().determinant().unwrap(),
        Rational::ONE
    );
    let gram = matrix(&[&[4, -2], &[-2, 4]]);
    let rotation = matrix(&[&[0, -1], &[1, -1]]);
    assert!(gram.is_positive_definite());
    assert!(gram.is_isometry(&rotation).unwrap());
    assert!(!matrix(&[&[1, 2], &[2, 1]]).is_positive_definite());
    assert!(!matrix(&[&[1, 1], &[0, 1]]).is_positive_definite());
    assert_eq!(
        weyl_reflect(&Vector::<4>::zero(), &Vector::zero()),
        Err(MathError::InvalidRootSystem)
    );
}

proptest! {
    #[test]
    fn weyl_is_involution_and_preserves_norm(v in prop::array::uniform4(-1000i64..1000), a in prop::array::uniform4(-10i64..10)) {
        prop_assume!(a.iter().any(|&x| x!=0));
        let v=Vector::new(v.map(Rational::from)); let a=Vector::new(a.map(Rational::from));
        let reflected=weyl_reflect(&v,&a).unwrap();
        prop_assert_eq!(reflected.norm_squared(),v.norm_squared());
        prop_assert_eq!(weyl_reflect(&reflected,&a).unwrap(),v);
    }
    #[test]
    fn triangular_determinant_and_inverse(a in 1i64..40,b in -100i64..100,c in 1i64..40) {
        let m=matrix(&[&[i128::from(a),i128::from(b)],&[0,i128::from(c)]]);
        prop_assert_eq!(m.determinant().unwrap(), Rational::from(a)*Rational::from(c));
        prop_assert_eq!(m.multiply(&m.inverse().unwrap()).unwrap(),Matrix::identity(2).unwrap());
    }
}
