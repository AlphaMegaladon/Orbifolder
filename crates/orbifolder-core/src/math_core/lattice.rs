use super::{GaugeVector, MathError, Rational, RightWeight, Vector};
use num_bigint::BigInt;
use num_integer::Integer;

/// Cosets of D_even in its weight lattice. S is (+1/2,...,+1/2)+D_even.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DnClass {
    Zero,
    Vector,
    Spinor,
    Cospinor,
}

impl DnClass {
    /// Discriminant group of even-rank D_n: Z2 x Z2.
    pub fn compose(self, other: Self) -> Self {
        use DnClass::{Cospinor, Spinor, Vector, Zero};
        match (self, other) {
            (Zero, x) | (x, Zero) => x,
            (Vector, Vector) | (Spinor, Spinor) | (Cospinor, Cospinor) => Zero,
            (Vector, Spinor) | (Spinor, Vector) => Cospinor,
            (Vector, Cospinor) | (Cospinor, Vector) => Spinor,
            (Spinor, Cospinor) | (Cospinor, Spinor) => Vector,
        }
    }
}

/// Rejects mixed or non-weight coordinates; supports even D_n, n>=4.
pub fn dn_class(coordinates: &[Rational]) -> Result<Option<DnClass>, MathError> {
    if coordinates.len() < 4 || coordinates.len() % 2 != 0 {
        return Err(MathError::DimensionMismatch);
    }
    let integral = coordinates.iter().all(Rational::is_integer);
    let half = Rational::new(1, 2)?;
    let half_integral = coordinates.iter().all(|x| (x - &half).is_integer());
    if !integral && !half_integral {
        return Ok(None);
    }
    let sum = coordinates.iter().fold(Rational::ZERO, |s, x| s + x);
    let even = if integral {
        sum.is_even_integer()
    } else {
        coordinates
            .iter()
            .fold(BigInt::from(0), |s, x| s + x.floor())
            .is_even()
    };
    Ok(Some(match (integral, even) {
        (true, true) => DnClass::Zero,
        (true, false) => DnClass::Vector,
        (false, true) => DnClass::Spinor,
        (false, false) => DnClass::Cospinor,
    }))
}

pub fn so8_class(weight: &RightWeight) -> Option<DnClass> {
    dn_class(weight.coordinates()).expect("fixed even dimension")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GaugeLattice {
    E8xE8,
    D16Plus,
    D8xD8,
}

/// Ambient D-weight class, kept distinct from membership in an extended lattice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GaugeWeightClass {
    D16(DnClass),
    D8Pair(DnClass, DnClass),
}

impl GaugeLattice {
    pub fn weight_class(self, vector: &GaugeVector) -> Option<GaugeWeightClass> {
        let x = vector.coordinates();
        if self == Self::D16Plus {
            dn_class(x)
                .expect("dimension 16")
                .map(GaugeWeightClass::D16)
        } else {
            let a = dn_class(&x[..8]).expect("dimension 8")?;
            let b = dn_class(&x[8..]).expect("dimension 8")?;
            Some(GaugeWeightClass::D8Pair(a, b))
        }
    }
    pub fn contains(self, vector: &GaugeVector) -> bool {
        use DnClass::{Spinor, Zero};
        matches!(
            (self, self.weight_class(vector)),
            (Self::D16Plus, Some(GaugeWeightClass::D16(Zero | Spinor)))
                | (
                    Self::E8xE8,
                    Some(GaugeWeightClass::D8Pair(Zero | Spinor, Zero | Spinor))
                )
                | (Self::D8xD8, Some(GaugeWeightClass::D8Pair(Zero, Zero)))
        )
    }
    pub fn same_coset(self, left: &GaugeVector, right: &GaugeVector) -> bool {
        self.contains(&left.sub(right))
    }
    /// Smallest positive k such that k*v lies in the lattice, without factoring
    /// arbitrarily large denominators. Every admissible coordinate is half-integral.
    pub fn coset_order(self, vector: &GaugeVector) -> BigInt {
        let two = Rational::from(2);
        let bound = vector
            .coordinates()
            .iter()
            .fold(BigInt::from(1), |k, x| k.lcm(&(x * &two).denominator()));
        for multiplier in [1_i32, 2, 4] {
            let order = &bound * multiplier;
            if self.contains(&vector.scale(&Rational::from(order.clone()))) {
                return order;
            }
        }
        unreachable!("four times a half-integral vector has even integral coordinates")
    }
    pub fn roots(self) -> Vec<GaugeVector> {
        match self {
            Self::D16Plus => dn_roots::<16>().expect("dimension 16"),
            Self::E8xE8 | Self::D8xD8 => {
                let roots = if self == Self::E8xE8 {
                    e8_roots()
                } else {
                    dn_roots::<8>().expect("dimension 8")
                };
                let mut result = Vec::with_capacity(2 * roots.len());
                for offset in [0, 8] {
                    for root in &roots {
                        result.push(Vector::new(std::array::from_fn(|i| {
                            if (offset..offset + 8).contains(&i) {
                                root.coordinates()[i - offset].clone()
                            } else {
                                Rational::ZERO
                            }
                        })));
                    }
                }
                result.sort();
                result
            }
        }
    }
    /// Exact lattice-character kernel. These are linear congruences, not sector
    /// projection or modular-invariance rules.
    pub fn surviving_roots(self, constraints: &[GaugeVector]) -> Vec<GaugeVector> {
        self.roots()
            .into_iter()
            .filter(|root| constraints.iter().all(|v| root.dot(v).is_integer()))
            .collect()
    }
}

/// D_n roots ±e_i±e_j; spinor cosets do not add norm-two roots to D16+.
pub fn dn_roots<const N: usize>() -> Result<Vec<Vector<N>>, MathError> {
    if N < 4 {
        return Err(MathError::DimensionMismatch);
    }
    let mut roots = Vec::new();
    for i in 0..N {
        for j in i + 1..N {
            for a in [-1, 1] {
                for b in [-1, 1] {
                    let mut coordinates = std::array::from_fn(|_| Rational::ZERO);
                    coordinates[i] = Rational::from(a);
                    coordinates[j] = Rational::from(b);
                    roots.push(Vector::new(coordinates));
                }
            }
        }
    }
    roots.sort();
    Ok(roots)
}

pub fn e8_roots() -> Vec<Vector<8>> {
    let mut roots = dn_roots::<8>().expect("dimension 8");
    for mask in 0_u16..256 {
        if mask.count_ones() % 2 != 0 {
            continue;
        }
        roots.push(Vector::new(std::array::from_fn(|i| {
            Rational::new(if mask & (1 << i) == 0 { 1 } else { -1 }, 2)
                .expect("nonzero denominator")
        })));
    }
    roots.sort();
    roots
}
