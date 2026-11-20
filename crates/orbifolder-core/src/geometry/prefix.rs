//! Exact reachability for one affine embedding vector. The per-depth key is the
//! residual polynomial: constant residues and only the live Gram frontier.
use super::{EmbeddingSpec, Geometry, ModelError, embedding_orders};
use crate::math_core::{GaugeLattice, GaugeVector, Rational};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive};
use std::collections::{HashMap, HashSet};

const WIDTH: usize = 16;
const DEAD: u32 = u32::MAX;
const ACCEPT: u32 = u32::MAX - 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrefixError {
    Input(&'static str),
    Model(ModelError),
    ResourceLimit,
}

#[derive(Clone, Debug)]
pub struct PrefixSpec {
    pub slots: Vec<usize>,
    pub alphabet: Vec<Rational>,
    pub domains: Option<Vec<Vec<usize>>>,
    pub basis: Option<[GaugeVector; WIDTH]>,
    pub max_states: usize,
}

pub struct PrefixMasker {
    actions: HashMap<Rational, usize>,
    integer_actions: HashMap<i64, usize>,
    allowed: [Vec<bool>; WIDTH],
    nodes: Vec<u32>,
    root: u32,
    states: usize,
}

type Form = [Rational; WIDTH + 1];

#[derive(Clone)]
struct Affine {
    center: GaugeVector,
    directions: [GaugeVector; WIDTH],
}
impl Affine {
    fn scale(&self, value: &Rational) -> Self {
        Self {
            center: self.center.scale(value),
            directions: self.directions.each_ref().map(|row| row.scale(value)),
        }
    }
    fn coordinate(&self, index: usize) -> Form {
        std::array::from_fn(|slot| {
            if slot == WIDTH {
                &self.center
            } else {
                &self.directions[slot]
            }
            .coordinates()[index]
                .clone()
        })
    }
}

#[derive(Eq, Hash, PartialEq)]
struct Polynomial {
    modulus: u64,
    constant: u64,
    linear: [u64; WIDTH],
    diagonal: [u64; WIDTH],
    cross: [[u64; WIDTH]; WIDTH],
}
impl Polynomial {
    fn new(
        constant: Rational,
        linear: [Rational; WIDTH],
        diagonal: [Rational; WIDTH],
        cross: [[Rational; WIDTH]; WIDTH],
        modulus: Rational,
    ) -> Result<Self, PrefixError> {
        let coefficients = std::iter::once(&constant)
            .chain(&linear)
            .chain(&diagonal)
            .chain(cross.iter().flatten())
            .map(|term| {
                if term.is_zero() {
                    Rational::ZERO
                } else {
                    term.checked_div(&modulus)
                        .expect("positive modulus")
                        .modulo_one()
                }
            })
            .collect::<Vec<_>>();
        let denominator = coefficients
            .iter()
            .filter(|term| !term.is_zero())
            .fold(BigInt::from(1), |acc, term| acc.lcm(&term.denominator()));
        let modulus = denominator.to_u64().ok_or(PrefixError::ResourceLimit)?;
        let mut values = coefficients.into_iter().map(|term| {
            if term.is_zero() {
                0
            } else {
                (term.numerator() * (&denominator / term.denominator()))
                    .to_u64()
                    .expect("nonnegative bounded residue")
            }
        });
        let mut result = Self {
            modulus,
            constant: values.next().unwrap(),
            linear: std::array::from_fn(|_| values.next().unwrap()),
            diagonal: std::array::from_fn(|_| values.next().unwrap()),
            cross: std::array::from_fn(|_| std::array::from_fn(|_| values.next().unwrap())),
        };
        for index in 0..WIDTH {
            let diagonal = result.diagonal[index];
            if diagonal != 0 && modulus % 2 == 0 && diagonal == modulus / 2 {
                result.linear[index] = ((u128::from(result.linear[index]) + u128::from(diagonal))
                    % u128::from(modulus)) as u64;
                result.diagonal[index] = 0;
            }
        }
        Ok(result)
    }
    fn linear(form: Form, modulus: Rational) -> Result<Self, PrefixError> {
        let mut terms = form.into_iter();
        let linear = std::array::from_fn(|_| terms.next().unwrap());
        Self::new(
            terms.next().unwrap(),
            linear,
            std::array::from_fn(|_| Rational::ZERO),
            std::array::from_fn(|_| std::array::from_fn(|_| Rational::ZERO)),
            modulus,
        )
    }
    fn product(
        left: &Affine,
        right: &Affine,
        twist: &Rational,
        prefactor: BigInt,
    ) -> Result<Self, PrefixError> {
        let zero = GaugeVector::zero();
        let active_left = left.directions.each_ref().map(|row| row != &zero);
        let active_right = right.directions.each_ref().map(|row| row != &zero);
        let linear = std::array::from_fn(|index| {
            let right_term = if active_right[index] {
                left.center.dot(&right.directions[index])
            } else {
                Rational::ZERO
            };
            let left_term = if active_left[index] {
                left.directions[index].dot(&right.center)
            } else {
                Rational::ZERO
            };
            right_term + left_term
        });
        let diagonal = std::array::from_fn(|index| {
            if active_left[index] && active_right[index] {
                left.directions[index].dot(&right.directions[index])
            } else {
                Rational::ZERO
            }
        });
        let cross = std::array::from_fn(|index| {
            std::array::from_fn(|later| {
                if index >= later {
                    return Rational::ZERO;
                }
                let term = if active_left[index] && active_right[later] {
                    left.directions[index].dot(&right.directions[later])
                } else {
                    Rational::ZERO
                };
                let swapped = if active_left[later] && active_right[index] {
                    left.directions[later].dot(&right.directions[index])
                } else {
                    Rational::ZERO
                };
                term + swapped
            })
        });
        Self::new(
            left.center.dot(&right.center) - twist,
            linear,
            diagonal,
            cross,
            Rational::from_bigints(BigInt::from(2), prefactor).expect("positive embedding orders"),
        )
    }
}

fn lattice_forms(
    value: &Affine,
    lattice: GaugeLattice,
    output: &mut HashSet<Polynomial>,
) -> Result<(), PrefixError> {
    let width = if lattice == GaugeLattice::E8xE8 {
        8
    } else {
        WIDTH
    };
    let doubled = value.scale(&Rational::from(2));
    for start in (0..WIDTH).step_by(width) {
        let first = doubled.coordinate(start);
        output.insert(Polynomial::linear(first.clone(), Rational::from(1))?);
        let mut sum = first.clone();
        for index in start + 1..start + width {
            let coordinate = doubled.coordinate(index);
            sum = std::array::from_fn(|slot| &sum[slot] + &coordinate[slot]);
            output.insert(Polynomial::linear(
                std::array::from_fn(|slot| &coordinate[slot] - &first[slot]),
                Rational::from(2),
            )?);
        }
        output.insert(Polynomial::linear(sum, Rational::from(4))?);
    }
    Ok(())
}

struct Transition {
    source: Option<usize>,
    slope: Option<usize>,
    add: Vec<u64>,
    modulus: u64,
    close: bool,
}

struct Compiler {
    steps: [Vec<Transition>; WIDTH],
    multipliers: [Vec<u64>; WIDTH],
    alphalen: usize,
    domains: [Vec<usize>; WIDTH],
    memo: [HashMap<Vec<u64>, u32>; WIDTH],
    nodes: Vec<u32>,
    states: usize,
    max_states: usize,
}

fn residue(value: &BigInt, modulus: u64) -> u64 {
    value.mod_floor(&BigInt::from(modulus)).to_u64().unwrap()
}

impl Compiler {
    fn prepare(
        polys: HashSet<Polynomial>,
        alphabet: &[BigInt],
        domains: [Vec<usize>; WIDTH],
        max_states: usize,
    ) -> Result<(Self, u32), PrefixError> {
        let mut compiler = Self {
            steps: std::array::from_fn(|_| Vec::new()),
            multipliers: std::array::from_fn(|_| Vec::new()),
            alphalen: alphabet.len(),
            domains,
            memo: std::array::from_fn(|_| HashMap::new()),
            nodes: Vec::new(),
            states: 0,
            max_states,
        };
        let mut initial = Vec::new();
        let mut offsets = [0; WIDTH + 1];
        let mut polys = polys.into_iter().collect::<Vec<_>>();
        polys.sort_unstable_by(|left, right| {
            (
                &left.modulus,
                &left.constant,
                &left.linear,
                &left.diagonal,
                &left.cross,
            )
                .cmp(&(
                    &right.modulus,
                    &right.constant,
                    &right.linear,
                    &right.diagonal,
                    &right.cross,
                ))
        });
        for poly in polys {
            let used = |index: usize| {
                poly.linear[index] != 0
                    || poly.diagonal[index] != 0
                    || (0..index).any(|earlier| poly.cross[earlier][index] != 0)
                    || poly.cross[index].iter().any(|&value| value != 0)
            };
            let Some(last) = (0..WIDTH).rfind(|&index| used(index)) else {
                if poly.constant != 0 {
                    return Ok((compiler, DEAD));
                }
                continue;
            };
            let mut constant = offsets[0];
            offsets[0] += 1;
            initial.push(poly.constant);
            let mut live = [None; WIDTH];
            for depth in 0..=last {
                let close = depth == last;
                let step = &mut compiler.steps[depth];
                step.push(Transition {
                    source: Some(constant),
                    slope: live[depth],
                    modulus: poly.modulus,
                    close,
                    add: alphabet
                        .iter()
                        .map(|action| {
                            residue(
                                &((BigInt::from(poly.diagonal[depth]) * action
                                    + poly.linear[depth])
                                    * action),
                                poly.modulus,
                            )
                        })
                        .collect(),
                });
                compiler.multipliers[depth].extend(if live[depth].is_some() {
                    alphabet
                        .iter()
                        .map(|action| residue(action, poly.modulus))
                        .collect::<Vec<_>>()
                } else {
                    vec![0; alphabet.len()]
                });
                if close {
                    break;
                }
                constant = offsets[depth + 1];
                offsets[depth + 1] += 1;
                for (later, slot) in live.iter_mut().enumerate().take(last + 1).skip(depth + 1) {
                    let coefficient = poly.cross[depth][later];
                    if coefficient == 0 && slot.is_none() {
                        continue;
                    }
                    step.push(Transition {
                        source: *slot,
                        slope: None,
                        modulus: poly.modulus,
                        close: false,
                        add: alphabet
                            .iter()
                            .map(|action| {
                                residue(&(BigInt::from(coefficient) * action), poly.modulus)
                            })
                            .collect(),
                    });
                    compiler.multipliers[depth].extend(vec![0; alphabet.len()]);
                    *slot = Some(offsets[depth + 1]);
                    offsets[depth + 1] += 1;
                }
            }
        }
        let root = compiler.visit(0, initial)?;
        Ok((compiler, root))
    }

    fn next(&self, depth: usize, state: &[u64], action: usize) -> Option<Vec<u64>> {
        let mut next = Vec::with_capacity(self.steps[depth].len());
        for (index, step) in self.steps[depth].iter().enumerate() {
            let base = u128::from(step.source.map_or(0, |position| state[position]));
            let modulus = u128::from(step.modulus);
            let slope = step.slope.map_or(0, |position| {
                (u128::from(state[position])
                    * u128::from(self.multipliers[depth][index * self.alphalen + action]))
                    % modulus
            });
            let value = ((base + slope + u128::from(step.add[action])) % modulus) as u64;
            if step.close {
                if value != 0 {
                    return None;
                }
            } else {
                next.push(value);
            }
        }
        Some(next)
    }

    fn visit(&mut self, depth: usize, state: Vec<u64>) -> Result<u32, PrefixError> {
        if depth == WIDTH {
            return Ok(ACCEPT);
        }
        if let Some(&node) = self.memo[depth].get(&state) {
            return Ok(node);
        }
        self.states += 1;
        if self.states > self.max_states {
            return Err(PrefixError::ResourceLimit);
        }
        let mut children = vec![DEAD; self.alphalen];
        for index in 0..self.domains[depth].len() {
            let action = self.domains[depth][index];
            if let Some(next) = self.next(depth, &state, action) {
                children[action] = self.visit(depth + 1, next)?;
            }
        }
        let node = if children.iter().all(|&value| value == DEAD) {
            DEAD
        } else {
            let offset = u32::try_from(self.nodes.len()).map_err(|_| PrefixError::ResourceLimit)?;
            if offset >= ACCEPT {
                return Err(PrefixError::ResourceLimit);
            }
            self.nodes.extend(children);
            offset
        };
        self.memo[depth].insert(state, node);
        Ok(node)
    }
}

impl PrefixMasker {
    pub fn prepare(
        geometry: &Geometry,
        embedding: &EmbeddingSpec,
        spec: PrefixSpec,
    ) -> Result<Self, PrefixError> {
        let PrefixSpec {
            slots,
            alphabet,
            domains,
            basis,
            max_states,
        } = spec;
        if slots.is_empty()
            || slots.iter().any(|&slot| slot >= 9)
            || slots.iter().collect::<HashSet<_>>().len() != slots.len()
            || alphabet.is_empty()
            || max_states == 0
            || alphabet
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != alphabet.len()
        {
            return Err(PrefixError::Input(
                "distinct slots and alphabet and a positive state limit required",
            ));
        }
        super::embedding::embedding_support(geometry, embedding).map_err(PrefixError::Model)?;
        let domains: [Vec<usize>; WIDTH] = domains
            .unwrap_or_else(|| vec![(0..alphabet.len()).collect(); WIDTH])
            .try_into()
            .map_err(|_| PrefixError::Input("expected sixteen coordinate domains"))?;
        if domains.iter().any(|domain| {
            domain.is_empty()
                || domain.iter().any(|&action| action >= alphabet.len())
                || domain.iter().collect::<HashSet<_>>().len() != domain.len()
        }) {
            return Err(PrefixError::Input(
                "each coordinate requires distinct alphabet indices",
            ));
        }
        let denominator = alphabet
            .iter()
            .fold(BigInt::from(1), |acc, value| acc.lcm(&value.denominator()));
        let actions = alphabet
            .iter()
            .map(|value| value.numerator() * (&denominator / value.denominator()))
            .collect::<Vec<_>>();
        let scale = Rational::from_bigints(BigInt::from(1), denominator)
            .expect("positive alphabet denominator");
        let affine = basis.is_some();
        let basis = basis
            .unwrap_or_else(|| {
                std::array::from_fn(|index| {
                    GaugeVector::from_integers(std::array::from_fn(|coordinate| {
                        i128::from(coordinate == index)
                    }))
                })
            })
            .map(|vector| vector.scale(&scale));
        let rows = embedding
            .shifts
            .iter()
            .chain(&embedding.wilson_lines)
            .enumerate()
            .map(|(index, center)| Affine {
                center: if !affine && slots.contains(&index) {
                    GaugeVector::zero()
                } else {
                    center.clone()
                },
                directions: if slots.contains(&index) {
                    basis.clone()
                } else {
                    std::array::from_fn(|_| GaugeVector::zero())
                },
            })
            .collect::<Vec<_>>();
        let mut polys = HashSet::new();
        let orders = embedding_orders(geometry);
        for (index, row) in rows.iter().enumerate() {
            if index >= 3 && geometry.wilson_relations().orders()[index - 3].is_none() {
                let max = Rational::from(actions.iter().map(|action| action.abs()).max().unwrap());
                for coordinate in 0..WIDTH {
                    let form = row.coordinate(coordinate);
                    let abs = |value: &Rational| {
                        if value < &Rational::ZERO {
                            -value
                        } else {
                            value.clone()
                        }
                    };
                    let bound = form[..WIDTH]
                        .iter()
                        .fold(abs(&form[WIDTH]), |sum, value| sum + abs(value) * &max);
                    polys.insert(Polynomial::linear(form, bound + Rational::from(1))?);
                }
            }
            lattice_forms(
                &row.scale(&Rational::from(orders[index].clone())),
                embedding.lattice,
                &mut polys,
            )?;
        }
        for relation in geometry.wilson_relations().relations() {
            let weights = relation.coordinates();
            let combine = |pick: &dyn Fn(&Affine) -> GaugeVector| {
                (0..6).fold(GaugeVector::zero(), |sum, index| {
                    sum.add(&pick(&rows[index + 3]).scale(&weights[index]))
                })
            };
            lattice_forms(
                &Affine {
                    center: combine(&|row| row.center.clone()),
                    directions: std::array::from_fn(|index| {
                        combine(&|row| row.directions[index].clone())
                    }),
                },
                embedding.lattice,
                &mut polys,
            )?;
        }
        let generators = geometry.space_group().generators();
        for left in 0..9 {
            for right in left..9 {
                let twist = if right < 3 {
                    generators[left].twist.dot(&generators[right].twist)
                } else {
                    Rational::ZERO
                };
                polys.insert(Polynomial::product(
                    &rows[left],
                    &rows[right],
                    &twist,
                    orders[left].gcd(&orders[right]),
                )?);
            }
        }
        let (compiler, root) = Compiler::prepare(polys, &actions, domains, max_states)?;
        let integer_actions = alphabet
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                if value.is_integer() {
                    value.numerator().to_i64().map(|value| (value, index))
                } else {
                    None
                }
            })
            .collect();
        Ok(Self {
            allowed: compiler.domains.map(|domain| {
                let mut allowed = vec![false; alphabet.len()];
                for index in domain {
                    allowed[index] = true;
                }
                allowed
            }),
            actions: alphabet
                .into_iter()
                .enumerate()
                .map(|(index, value)| (value, index))
                .collect(),
            integer_actions,
            nodes: compiler.nodes,
            root,
            states: compiler.states,
        })
    }

    pub fn action_index(&self, action: &Rational) -> Option<usize> {
        self.actions.get(action).copied()
    }
    pub fn integer_action_index(&self, action: i64) -> Option<usize> {
        self.integer_actions.get(&action).copied()
    }
    pub fn states(&self) -> usize {
        self.states
    }
    fn lookup(&self, prefix: &[usize]) -> Result<u32, PrefixError> {
        if prefix.len() > WIDTH {
            return Err(PrefixError::Input("prefix exceeds sixteen coordinates"));
        }
        let mut node = self.root;
        for (depth, &action) in prefix.iter().enumerate() {
            if self.allowed[depth].get(action) != Some(&true) {
                return Err(PrefixError::Input("action outside coordinate domain"));
            }
            if node != DEAD {
                node = self.nodes[node as usize + action];
            }
        }
        Ok(node)
    }
    pub fn mask(&self, prefix: &[usize]) -> Result<Vec<bool>, PrefixError> {
        if prefix.len() >= WIDTH {
            return Err(PrefixError::Input("mask needs an incomplete prefix"));
        }
        let node = self.lookup(prefix)?;
        Ok(if node == DEAD {
            vec![false; self.actions.len()]
        } else {
            self.nodes[node as usize..node as usize + self.actions.len()]
                .iter()
                .map(|&child| child != DEAD)
                .collect()
        })
    }
    pub fn is_feasible(&self, prefix: &[usize]) -> Result<bool, PrefixError> {
        Ok(self.lookup(prefix)? != DEAD)
    }
}
