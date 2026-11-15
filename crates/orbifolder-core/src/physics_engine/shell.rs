use super::{Excitation, PhysicsError, SolverLimits};
use crate::math_core::{DnClass, GaugeLattice, Rational, Vector, dn_class};
use num_bigint::BigInt;
use num_traits::ToPrimitive;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellLattice {
    Gauge(GaugeLattice),
    RightSo8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MassShell<const N: usize> {
    pub(super) excitation: Excitation,
    pub(super) weights: Vec<Vector<N>>,
}
impl<const N: usize> MassShell<N> {
    pub fn excitation(&self) -> &Excitation {
        &self.excitation
    }
    pub fn weights(&self) -> &[Vector<N>] {
        &self.weights
    }
}

/// Enumerate a translated Euclidean mass shell with exact branch-and-bound.
/// At depth j, remaining >= sum_{i>=j}(p_i+shift_i)^2; this gives a complete
/// coordinate interval, including arbitrarily large integral shifts.
pub fn enumerate_shell<const N: usize>(
    shift: &Vector<N>,
    target: &Rational,
    lattice: ShellLattice,
    limits: SolverLimits,
) -> Result<Vec<Vector<N>>, PhysicsError> {
    enumerate_ball(shift, target, lattice, true, limits)
}

pub(super) fn enumerate_ball<const N: usize>(
    shift: &Vector<N>,
    bound: &Rational,
    lattice: ShellLattice,
    shell_only: bool,
    limits: SolverLimits,
) -> Result<Vec<Vector<N>>, PhysicsError> {
    let block = match lattice {
        ShellLattice::RightSo8 if N == 4 => 4,
        ShellLattice::Gauge(GaugeLattice::D16Plus) if N == 16 => 16,
        ShellLattice::Gauge(_) if N == 16 => 8,
        _ => {
            return Err(PhysicsError::Math(
                crate::math_core::MathError::DimensionMismatch,
            ));
        }
    };
    if bound < &Rational::ZERO {
        return Ok(Vec::new());
    }
    struct Search<'a, const N: usize> {
        shift: &'a Vector<N>,
        lattice: ShellLattice,
        block: usize,
        shell: bool,
        limits: SolverLimits,
        nodes: usize,
        out: Vec<Vector<N>>,
        p: [Rational; N],
    }
    impl<const N: usize> Search<'_, N> {
        fn walk(&mut self, j: usize, remaining: Rational, half: bool) -> Result<(), PhysicsError> {
            self.nodes += 1;
            if self.nodes > self.limits.search_nodes {
                return Err(PhysicsError::ResourceLimit);
            }
            if j == N {
                if !self.shell || remaining.is_zero() {
                    if self.out.len() >= self.limits.solutions {
                        return Err(PhysicsError::ResourceLimit);
                    }
                    self.out.push(Vector::new(std::array::from_fn(|i| {
                        &self.p[i] + &self.shift.coordinates()[i]
                    })));
                }
                return Ok(());
            }
            if j % self.block == 0 {
                self.coordinate(j, remaining.clone(), false)?;
                if !matches!(self.lattice, ShellLattice::Gauge(GaugeLattice::D8xD8)) {
                    self.coordinate(j, remaining, true)?;
                }
                Ok(())
            } else {
                self.coordinate(j, remaining, half)
            }
        }
        fn coordinate(
            &mut self,
            j: usize,
            remaining: Rational,
            half: bool,
        ) -> Result<(), PhysicsError> {
            let offset = Rational::new(i128::from(half), 2)?;
            let center = &self.shift.coordinates()[j] + &offset;
            // floor(sqrt(floor(remaining))) + 1 is a rigorous upper bound.
            let radius = remaining.floor().sqrt() + BigInt::from(1);
            let floor = center.floor();
            // Enumerate the small residual interval around -floor(center),
            // retaining its possibly enormous integral part as a BigInt.
            let lo = (-&radius - BigInt::from(1))
                .to_i64()
                .ok_or(PhysicsError::ResourceLimit)?;
            let hi = radius.to_i64().ok_or(PhysicsError::ResourceLimit)?;
            if i128::from(hi) - i128::from(lo) > self.limits.search_nodes as i128 {
                return Err(PhysicsError::ResourceLimit);
            }
            for n in lo..=hi {
                let shifted = Rational::from(n) + center.modulo_one();
                let square = &shifted * &shifted;
                if square > remaining {
                    continue;
                }
                self.p[j] = Rational::from(BigInt::from(n) - &floor) + &offset;
                if (j + 1) % self.block == 0 {
                    let class = dn_class(&self.p[j + 1 - self.block..=j])?;
                    let allowed = match self.lattice {
                        ShellLattice::RightSo8 => {
                            matches!(class, Some(DnClass::Vector | DnClass::Spinor))
                        }
                        _ => matches!(class, Some(DnClass::Zero | DnClass::Spinor)),
                    };
                    if !allowed {
                        continue;
                    }
                }
                self.walk(j + 1, &remaining - square, half)?;
            }
            Ok(())
        }
    }
    let mut search = Search {
        shift,
        lattice,
        block,
        shell: shell_only,
        limits,
        nodes: 0,
        out: Vec::new(),
        p: std::array::from_fn(|_| Rational::ZERO),
    };
    search.walk(0, bound.clone(), false)?;
    search.out.sort();
    Ok(search.out)
}
