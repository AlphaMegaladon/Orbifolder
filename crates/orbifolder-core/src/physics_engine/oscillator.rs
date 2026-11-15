use super::{PhysicsError, SolverLimits};
use crate::{
    geometry::TheorySpec,
    math_core::{Rational, RightWeight},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VacuumEnergy {
    left: Rational,
    right: Rational,
}
impl VacuumEnergy {
    /// eta_i = v_i mod 1, delta = sum eta_i(1-eta_i)/2.
    pub fn from_twist(twist: &RightWeight) -> Self {
        let half = Rational::new(1, 2).expect("nonzero");
        let delta = twist.coordinates().iter().fold(Rational::ZERO, |s, v| {
            let eta = v.modulo_one();
            s + &eta * (Rational::ONE - &eta) * &half
        });
        Self {
            left: &delta - Rational::ONE,
            right: delta - half,
        }
    }
    pub fn left(&self) -> &Rational {
        &self.left
    }
    pub fn right(&self) -> &Rational {
        &self.right
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mover {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Oscillator {
    direction: usize,
    conjugate: bool,
    frequency: Rational,
}
impl Oscillator {
    pub fn direction(&self) -> usize {
        self.direction
    }
    pub fn conjugate(&self) -> bool {
        self.conjugate
    }
    pub fn frequency(&self) -> &Rational {
        &self.frequency
    }
    pub fn energy(&self) -> Rational {
        -&self.frequency
    }
    pub fn transformation(&self, twist: &RightWeight) -> Rational {
        if self.conjugate {
            -&twist.coordinates()[self.direction]
        } else {
            twist.coordinates()[self.direction].clone()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Excitation {
    indices: Vec<usize>,
    energy: Rational,
    target: Rational,
}
impl Excitation {
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }
    pub fn energy(&self) -> &Rational {
        &self.energy
    }
    /// Squared momentum budget for the mass supplied to `excitations`.
    /// For right tachyons the native-compatible shell stores the zero-mass
    /// ceiling here; its actual negative mass is stored in `tachyon_masses`.
    pub fn target(&self) -> &Rational {
        &self.target
    }
    pub fn is_excited(&self) -> bool {
        !self.indices.is_empty()
    }
    pub fn transformation(&self, modes: &[Oscillator], twist: &RightWeight) -> Rational {
        self.indices
            .iter()
            .fold(Rational::ZERO, |s, &i| s + modes[i].transformation(twist))
    }
    pub fn contribution(&self, modes: &[Oscillator]) -> RightWeight {
        let mut v = std::array::from_fn(|_| Rational::ZERO);
        for &i in &self.indices {
            let o = &modes[i];
            v[o.direction] = &v[o.direction] + Rational::from(if o.conjugate { 1 } else { -1 });
        }
        RightWeight::new(v)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectorModes {
    vacuum: VacuumEnergy,
    left: Vec<Oscillator>,
    right: Vec<Oscillator>,
}
impl SectorModes {
    /// Native matter/gravity oscillator catalog. Cartan gauge oscillators are
    /// excluded by the original programs. Modes above the mass budget remain
    /// in the catalog for stable oracle indices, but never enter a Fock state.
    pub fn new(
        twist: &RightWeight,
        generators: &[RightWeight],
        factors: [u32; 3],
        theory: TheorySpec,
    ) -> Self {
        let mut left = Vec::new();
        let mut right = Vec::new();
        if factors == [0; 3] {
            let mut directions = Vec::new();
            for v in generators {
                for direction in 0..4 {
                    if theory == TheorySpec::Susy
                        && (direction == 0 || v.coordinates()[direction].is_integer())
                    {
                        continue;
                    }
                    if !directions.contains(&direction) {
                        directions.push(direction);
                    }
                }
            }
            for direction in directions {
                for conjugate in [false, true] {
                    left.push(Oscillator {
                        direction,
                        conjugate,
                        frequency: Rational::from(-1),
                    });
                }
            }
        } else {
            for direction in 1..4 {
                let eta = twist.coordinates()[direction].modulo_one();
                if eta.is_zero() {
                    continue;
                }
                for m in [-1, 0] {
                    for conjugate in [false, true] {
                        for mover in [Mover::Left, Mover::Right] {
                            let sign = if (mover == Mover::Left) == conjugate {
                                1
                            } else {
                                -1
                            };
                            let frequency = Rational::from(m) + Rational::from(sign) * &eta;
                            if frequency < Rational::ZERO {
                                let mode = Oscillator {
                                    direction,
                                    conjugate,
                                    frequency,
                                };
                                match mover {
                                    Mover::Left => left.push(mode),
                                    Mover::Right => right.push(mode),
                                }
                            }
                        }
                    }
                }
            }
        }
        Self {
            vacuum: VacuumEnergy::from_twist(twist),
            left,
            right,
        }
    }
    pub fn vacuum(&self) -> &VacuumEnergy {
        &self.vacuum
    }
    pub fn left(&self) -> &[Oscillator] {
        &self.left
    }
    pub fn right(&self) -> &[Oscillator] {
        &self.right
    }
    pub fn excitations(
        &self,
        mover: Mover,
        mass: &Rational,
        limits: SolverLimits,
    ) -> Result<Vec<Excitation>, PhysicsError> {
        let (modes, a) = match mover {
            Mover::Left => (&self.left, &self.vacuum.left),
            Mover::Right => (&self.right, &self.vacuum.right),
        };
        let budget = mass - a;
        let mut out = Vec::new();
        let mut nodes = 0;
        let mut pending = Vec::new();
        if budget >= Rational::ZERO {
            pending.push((Vec::new(), Rational::ZERO, 0));
        }
        // Explicit DFS stack: even tiny fractional frequencies cannot overflow
        // the native call stack. Reverse pushes preserve ascending Fock order.
        while let Some((indices, energy, start)) = pending.pop() {
            nodes += 1;
            if nodes > limits.search_nodes || out.len() >= limits.solutions {
                return Err(PhysicsError::ResourceLimit);
            }
            out.push(Excitation {
                indices: indices.clone(),
                target: Rational::from(2) * (&budget - &energy),
                energy: energy.clone(),
            });
            for i in (start..modes.len()).rev() {
                let next = &energy + modes[i].energy();
                if next <= budget {
                    if pending.len() + nodes >= limits.search_nodes {
                        return Err(PhysicsError::ResourceLimit);
                    }
                    let mut next_indices = indices.clone();
                    next_indices.push(i);
                    pending.push((next_indices, next, i));
                }
            }
        }
        Ok(out)
    }
}
