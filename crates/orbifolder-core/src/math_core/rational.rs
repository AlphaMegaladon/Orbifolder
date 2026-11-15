use super::MathError;
use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::ToPrimitive;
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Repr {
    Small(i128, i128),
    Big(BigRational),
}

/// Canonical exact rational, with checked i128 arithmetic and BigInt promotion.
///
/// Denominators are positive, zero is 0/1, and results are demoted whenever they
/// fit again. No overflow, comparison or MIN-value operation uses a float.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Rational(Repr);

impl Rational {
    pub const ZERO: Self = Self(Repr::Small(0, 1));
    pub const ONE: Self = Self(Repr::Small(1, 1));

    pub fn new(numerator: i128, denominator: i128) -> Result<Self, MathError> {
        if denominator == 0 {
            return Err(MathError::ZeroDenominator);
        }
        if let Some(value) = Self::small(numerator, denominator) {
            return Ok(value);
        }
        Self::from_bigints(numerator.into(), denominator.into())
    }

    pub fn from_bigints(numerator: BigInt, denominator: BigInt) -> Result<Self, MathError> {
        if denominator == BigInt::from(0) {
            return Err(MathError::ZeroDenominator);
        }
        Ok(Self::compact(BigRational::new(numerator, denominator)))
    }

    fn small(mut n: i128, mut d: i128) -> Option<Self> {
        if d < 0 {
            n = n.checked_neg()?;
            d = d.checked_neg()?;
        }
        let gcd = i128::try_from(n.unsigned_abs().gcd(&d.unsigned_abs())).ok()?;
        Some(Self(Repr::Small(n / gcd, d / gcd)))
    }

    fn compact(value: BigRational) -> Self {
        match (value.numer().to_i128(), value.denom().to_i128()) {
            (Some(n), Some(d)) => Self(Repr::Small(n, d)),
            _ => Self(Repr::Big(value)),
        }
    }

    fn big(&self) -> BigRational {
        match &self.0 {
            Repr::Small(n, d) => BigRational::new((*n).into(), (*d).into()),
            Repr::Big(value) => value.clone(),
        }
    }

    pub fn numerator(&self) -> BigInt {
        match &self.0 {
            Repr::Small(n, _) => (*n).into(),
            Repr::Big(value) => value.numer().clone(),
        }
    }

    pub fn denominator(&self) -> BigInt {
        match &self.0 {
            Repr::Small(_, d) => (*d).into(),
            Repr::Big(value) => value.denom().clone(),
        }
    }

    pub fn uses_bigint(&self) -> bool {
        matches!(self.0, Repr::Big(_))
    }

    pub fn is_zero(&self) -> bool {
        match &self.0 {
            Repr::Small(n, _) => *n == 0,
            Repr::Big(value) => value.numer() == &BigInt::from(0),
        }
    }

    pub fn is_integer(&self) -> bool {
        match &self.0 {
            Repr::Small(_, d) => *d == 1,
            Repr::Big(value) => value.is_integer(),
        }
    }

    pub fn to_integer(&self) -> Result<BigInt, MathError> {
        if self.is_integer() {
            Ok(self.numerator())
        } else {
            Err(MathError::NonIntegral)
        }
    }

    pub fn is_even_integer(&self) -> bool {
        match &self.0 {
            Repr::Small(n, d) => *d == 1 && n % 2 == 0,
            Repr::Big(value) => value.is_integer() && value.numer().is_even(),
        }
    }

    pub fn floor(&self) -> BigInt {
        match &self.0 {
            Repr::Small(n, d) => n.div_floor(d).into(),
            Repr::Big(value) => value.numer().div_floor(value.denom()),
        }
    }

    pub fn modulo_one(&self) -> Self {
        match &self.0 {
            Repr::Small(n, d) => Self(Repr::Small(n.rem_euclid(*d), *d)),
            Repr::Big(value) => Self::compact(BigRational::new(
                value.numer().mod_floor(value.denom()),
                value.denom().clone(),
            )),
        }
    }

    pub fn abs(&self) -> Self {
        if self < &Self::ZERO {
            -self
        } else {
            self.clone()
        }
    }

    pub fn checked_div(&self, other: &Self) -> Result<Self, MathError> {
        if other.is_zero() {
            return Err(MathError::DivisionByZero);
        }
        if let Repr::Small(n, d) = other.0 {
            if let Some(inverse) = Self::small(d, n) {
                return Ok(self * &inverse);
            }
        }
        Ok(Self::compact(self.big() / other.big()))
    }

    fn add_exact(&self, other: &Self) -> Self {
        if let (Repr::Small(a, b), Repr::Small(c, d)) = (&self.0, &other.0) {
            let gcd = b.gcd(d);
            if let Some((n, den)) = a
                .checked_mul(d / gcd)
                .and_then(|x| c.checked_mul(b / gcd).and_then(|y| x.checked_add(y)))
                .zip((b / gcd).checked_mul(*d))
            {
                return Self::small(n, den).expect("positive reduced denominator");
            }
        }
        Self::compact(self.big() + other.big())
    }

    fn mul_exact(&self, other: &Self) -> Self {
        if let (Repr::Small(a, b), Repr::Small(c, d)) = (&self.0, &other.0) {
            // gcds cannot exceed the positive i128 denominators.
            let g1 = i128::try_from(a.unsigned_abs().gcd(&d.unsigned_abs())).expect("bounded gcd");
            let g2 = i128::try_from(c.unsigned_abs().gcd(&b.unsigned_abs())).expect("bounded gcd");
            if let Some((n, den)) = (a / g1)
                .checked_mul(c / g2)
                .zip((b / g2).checked_mul(d / g1))
            {
                return Self(Repr::Small(n, den));
            }
        }
        Self::compact(self.big() * other.big())
    }
}

impl Default for Rational {
    fn default() -> Self {
        Self::ZERO
    }
}
impl From<i128> for Rational {
    fn from(value: i128) -> Self {
        Self(Repr::Small(value, 1))
    }
}
impl From<i64> for Rational {
    fn from(value: i64) -> Self {
        Self::from(i128::from(value))
    }
}
impl From<i32> for Rational {
    fn from(value: i32) -> Self {
        Self::from(i128::from(value))
    }
}
impl From<BigInt> for Rational {
    fn from(value: BigInt) -> Self {
        Self::compact(BigRational::from_integer(value))
    }
}

impl FromStr for Rational {
    type Err = MathError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (n, d) = value.split_once('/').unwrap_or((value, "1"));
        Self::from_bigints(
            n.parse().map_err(|_| MathError::InvalidRational)?,
            d.parse().map_err(|_| MathError::InvalidRational)?,
        )
    }
}
impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Small(n, 1) => write!(f, "{n}"),
            Repr::Small(n, d) => write!(f, "{n}/{d}"),
            Repr::Big(value) => write!(f, "{value}"),
        }
    }
}
impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        if let (Repr::Small(a, b), Repr::Small(c, d)) = (&self.0, &other.0) {
            if let Some((left, right)) = a.checked_mul(*d).zip(c.checked_mul(*b)) {
                return left.cmp(&right);
            }
        }
        self.big().cmp(&other.big())
    }
}
impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Neg for &Rational {
    type Output = Rational;
    fn neg(self) -> Rational {
        if let Repr::Small(n, d) = self.0 {
            if let Some(n) = n.checked_neg() {
                return Rational(Repr::Small(n, d));
            }
        }
        Rational::compact(-self.big())
    }
}
impl Neg for Rational {
    type Output = Self;
    fn neg(self) -> Self {
        -&self
    }
}

macro_rules! binary {
    ($trait:ident, $method:ident, $exact:ident) => {
        impl $trait<&Rational> for &Rational {
            type Output = Rational;
            fn $method(self, other: &Rational) -> Rational {
                self.$exact(other)
            }
        }
        impl $trait<Rational> for Rational {
            type Output = Rational;
            fn $method(self, other: Rational) -> Rational {
                self.$exact(&other)
            }
        }
        impl $trait<&Rational> for Rational {
            type Output = Rational;
            fn $method(self, other: &Rational) -> Rational {
                self.$exact(other)
            }
        }
        impl $trait<Rational> for &Rational {
            type Output = Rational;
            fn $method(self, other: Rational) -> Rational {
                self.$exact(&other)
            }
        }
    };
}
impl Rational {
    fn sub_exact(&self, other: &Self) -> Self {
        self + -other
    }
}
binary!(Add, add, add_exact);
binary!(Sub, sub, sub_exact);
binary!(Mul, mul, mul_exact);

/// An element of Q/Z. Only scalar phases are reduced modulo one.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Phase(Rational);
impl Phase {
    pub fn new(value: Rational) -> Self {
        Self(value.modulo_one())
    }
    pub fn value(&self) -> &Rational {
        &self.0
    }
    pub fn is_trivial(&self) -> bool {
        self.0.is_zero()
    }
}
impl Add for Phase {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.0 + other.0)
    }
}
impl Sub for Phase {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self::new(self.0 - other.0)
    }
}
impl Neg for Phase {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.0)
    }
}
