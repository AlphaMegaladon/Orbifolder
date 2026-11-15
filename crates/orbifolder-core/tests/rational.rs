use num_bigint::BigInt;
use num_rational::BigRational;
use orbifolder_core::math_core::{MathError, Phase, Rational};
use proptest::prelude::*;

fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).unwrap()
}

#[test]
fn signs_zero_and_extreme_construction() {
    assert_eq!(r(-6, -8), r(3, 4));
    assert_eq!(r(0, i128::MIN), Rational::ZERO);
    assert_eq!(r(i128::MIN, i128::MIN), Rational::ONE);
    assert_eq!(r(1, i128::MIN).denominator(), BigInt::from(1) << 127);
    assert_eq!(Rational::new(1, 0), Err(MathError::ZeroDenominator));
    assert_eq!(
        r(1, 2).checked_div(&Rational::ZERO),
        Err(MathError::DivisionByZero)
    );
    assert_eq!("1/0".parse::<Rational>(), Err(MathError::ZeroDenominator));
    assert!("1/2/3".parse::<Rational>().is_err());
    assert!("0.5".parse::<Rational>().is_err());
}

#[test]
fn promotion_demotion_and_cross_cancellation() {
    let largest = Rational::from(i128::MAX);
    let big = &largest + Rational::ONE;
    assert!(big.uses_bigint());
    assert_eq!(&big - Rational::ONE, largest);
    assert!(!(&big - Rational::ONE).uses_bigint());
    assert!((-Rational::from(i128::MIN)).uses_bigint());
    assert_eq!(r(i128::MAX, 2) * r(2, i128::MAX), Rational::ONE);
    assert_eq!(r(1, i128::MAX) + r(i128::MAX - 1, i128::MAX), Rational::ONE);
    assert!(r(i128::MAX, i128::MAX - 1) > r(i128::MAX - 1, i128::MAX));
    assert_eq!(r(i128::MIN, 1).abs(), big);
}

#[test]
fn negative_and_large_phases() {
    assert_eq!(Phase::new(r(-7, 3)).value(), &r(2, 3));
    assert!(Phase::new(r(-3, 1)).is_trivial());
    assert!((Phase::new(r(-1, 3)) + Phase::new(r(1, 3))).is_trivial());
    assert_eq!(r(-7, 3).floor(), BigInt::from(-3));
    let n: BigInt = BigInt::from(1) << 180;
    let big = Rational::from_bigints(-n, 7.into()).unwrap();
    assert!(big.uses_bigint());
    let phase = Phase::new(big);
    assert!(phase.value() >= &Rational::ZERO && phase.value() < &Rational::ONE);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(160))]
    #[test]
    fn checked_arithmetic_matches_unbounded_reference(a in any::<i128>(), b in 1..i128::MAX,
                                                    c in any::<i128>(), d in 1..i128::MAX) {
        let x=r(a,b); let y=r(c,d);
        let bx=BigRational::new(a.into(),b.into());
        let by=BigRational::new(c.into(),d.into());
        let same = |x: Rational,y: BigRational| {
            prop_assert_eq!(x.numerator(), y.numer().clone());
            prop_assert_eq!(x.denominator(), y.denom().clone());
            Ok(())
        };
        same(&x+&y, &bx+&by)?;
        same(&x-&y, &bx-&by)?;
        same(&x*&y, &bx*&by)?;
        if c!=0 { same(x.checked_div(&y).unwrap(), &bx/&by)?; }
        prop_assert_eq!(x.cmp(&y), bx.cmp(&by));
        let p=x.modulo_one();
        prop_assert!(p>=Rational::ZERO && p<Rational::ONE);
        prop_assert!((&x-&p).is_integer());
        prop_assert_eq!(p.modulo_one(),p);
    }
}
