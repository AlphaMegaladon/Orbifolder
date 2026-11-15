use super::{MathError, Matrix, Rational, Vector, weyl_reflect};
use num_bigint::BigInt;
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Algebra {
    A(usize),
    D(usize),
    E6,
    E7,
    E8,
}
impl Algebra {
    pub fn rank(&self) -> usize {
        match self {
            Self::A(n) | Self::D(n) => *n,
            Self::E6 => 6,
            Self::E7 => 7,
            Self::E8 => 8,
        }
    }
    pub fn root_count(&self) -> Result<usize, MathError> {
        let n = self.rank();
        if n == 0 || matches!(self,Self::D(n) if *n<4) {
            return Err(MathError::InvalidRootSystem);
        }
        match self {
            Self::A(_) => n.checked_add(1).and_then(|next| n.checked_mul(next)),
            Self::D(_) => n.checked_mul(n - 1).and_then(|x| x.checked_mul(2)),
            Self::E6 => Some(72),
            Self::E7 => Some(126),
            Self::E8 => Some(240),
        }
        .ok_or(MathError::CapacityOverflow)
    }
}
impl std::fmt::Display for Algebra {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A(n) => write!(f, "A{n}"),
            Self::D(n) => write!(f, "D{n}"),
            Self::E6 => write!(f, "E6"),
            Self::E7 => write!(f, "E7"),
            Self::E8 => write!(f, "E8"),
        }
    }
}

pub fn cartan_matrix<const N: usize>(simple: &[Vector<N>]) -> Result<Matrix, MathError> {
    Matrix::from_rows(
        simple
            .iter()
            .map(|a| {
                simple
                    .iter()
                    .map(|b| (Rational::from(2) * a.dot(b)).checked_div(&b.norm_squared()))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
}

pub fn dynkin_labels<const N: usize>(
    weight: &Vector<N>,
    simple: &[Vector<N>],
) -> Result<Vec<BigInt>, MathError> {
    simple
        .iter()
        .map(|a| {
            (Rational::from(2) * weight.dot(a))
                .checked_div(&a.norm_squared())?
                .to_integer()
        })
        .collect()
}

pub fn fundamental_weights<const N: usize>(
    simple: &[Vector<N>],
) -> Result<Vec<Vector<N>>, MathError> {
    let inverse = cartan_matrix(simple)?.inverse()?;
    Ok((0..simple.len())
        .map(|i| {
            simple
                .iter()
                .enumerate()
                .fold(Vector::zero(), |sum, (j, a)| {
                    sum.add(&a.scale(inverse.get(i, j).expect("square inverse")))
                })
        })
        .collect())
}

/// Extract the unique dominant weight of an already supplied irreducible set.
/// No state enumeration or representation-physics solver is performed here.
pub fn highest_weight<const N: usize>(
    weights: &[Vector<N>],
    simple: &[Vector<N>],
) -> Result<Vector<N>, MathError> {
    let mut dominant = BTreeSet::new();
    for weight in weights {
        if dynkin_labels(weight, simple)?
            .iter()
            .all(|n| n >= &BigInt::from(0))
        {
            dominant.insert(weight.clone());
        }
    }
    if dominant.len() != 1 {
        return Err(MathError::InvalidRootSystem);
    }
    Ok(dominant.pop_first().expect("one dominant weight"))
}

/// Exact finite Weyl closure; an explicit cap prevents unbounded enumeration.
pub fn root_closure<const N: usize>(
    simple: &[Vector<N>],
    cap: usize,
) -> Result<Vec<Vector<N>>, MathError> {
    let mut known = simple.iter().cloned().collect::<BTreeSet<_>>();
    let mut queue = simple.iter().cloned().collect::<VecDeque<_>>();
    if known.len() > cap {
        return Err(MathError::CapacityOverflow);
    }
    while let Some(root) = queue.pop_front() {
        for alpha in simple {
            let reflected = weyl_reflect(&root, alpha)?;
            if known.insert(reflected.clone()) {
                if known.len() > cap {
                    return Err(MathError::CapacityOverflow);
                }
                queue.push_back(reflected);
            }
        }
    }
    Ok(known.into_iter().collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LieFactor<const N: usize> {
    algebra: Algebra,
    simple: Vec<Vector<N>>,
    roots: Vec<Vector<N>>,
}
impl<const N: usize> LieFactor<N> {
    pub fn algebra(&self) -> &Algebra {
        &self.algebra
    }
    pub fn simple_roots(&self) -> &[Vector<N>] {
        &self.simple
    }
    pub fn roots(&self) -> &[Vector<N>] {
        &self.roots
    }
    pub fn cartan_matrix(&self) -> Result<Matrix, MathError> {
        cartan_matrix(&self.simple)
    }
}

/// Reduced, simply-laced norm-two root system with an explicit regular chamber.
/// Roots, factors and bases are sorted deterministically and cannot be mutated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootSystem<const N: usize> {
    roots: Vec<Vector<N>>,
    factors: Vec<LieFactor<N>>,
}

impl<const N: usize> RootSystem<N> {
    pub fn new(roots: Vec<Vector<N>>, chamber: &Vector<N>) -> Result<Self, MathError> {
        let set = roots.iter().cloned().collect::<BTreeSet<_>>();
        if set.len() != roots.len() {
            return Err(MathError::InvalidRootSystem);
        }
        for root in &set {
            if root.norm_squared() != Rational::from(2)
                || !set.contains(&root.scale(&Rational::from(-1)))
            {
                return Err(MathError::InvalidRootSystem);
            }
            if root.dot(chamber).is_zero() {
                return Err(MathError::IrregularChamber);
            }
        }
        let positive = set
            .iter()
            .filter(|r| r.dot(chamber) > Rational::ZERO)
            .cloned()
            .collect::<BTreeSet<_>>();
        // Indecomposable positive roots are the chamber's simple roots.
        let simple = positive
            .iter()
            .filter(|a| !positive.iter().any(|b| positive.contains(&a.sub(b))))
            .cloned()
            .collect::<Vec<_>>();
        let cartan = cartan_matrix(&simple)?;
        if !cartan.is_positive_definite() {
            return Err(MathError::InvalidRootSystem);
        }
        let mut adjacency = vec![Vec::new(); simple.len()];
        for (i, a) in simple.iter().enumerate() {
            for (j, b) in simple.iter().enumerate() {
                if i == j {
                    continue;
                }
                let product = a.dot(b);
                if product == Rational::from(-1) {
                    adjacency[i].push(j);
                } else if !product.is_zero() {
                    return Err(MathError::InvalidRootSystem);
                }
            }
        }
        let mut remaining = (0..simple.len()).collect::<BTreeSet<_>>();
        let mut factors = Vec::new();
        let mut generated = BTreeSet::new();
        while let Some(start) = remaining.pop_first() {
            let mut queue = vec![start];
            let mut component = Vec::new();
            while let Some(node) = queue.pop() {
                component.push(node);
                for &neighbor in &adjacency[node] {
                    if remaining.remove(&neighbor) {
                        queue.push(neighbor);
                    }
                }
            }
            component.sort();
            let algebra = classify(&component, &adjacency)?;
            let basis = component
                .iter()
                .map(|&i| simple[i].clone())
                .collect::<Vec<_>>();
            let count = algebra.root_count()?;
            let factor_roots = root_closure(&basis, count)?;
            if factor_roots.len() != count {
                return Err(MathError::InvalidRootSystem);
            }
            generated.extend(factor_roots.iter().cloned());
            factors.push(LieFactor {
                algebra,
                simple: basis,
                roots: factor_roots,
            });
        }
        if generated != set {
            return Err(MathError::InvalidRootSystem);
        }
        Ok(Self {
            roots: set.into_iter().collect(),
            factors,
        })
    }
    pub fn roots(&self) -> &[Vector<N>] {
        &self.roots
    }
    pub fn factors(&self) -> &[LieFactor<N>] {
        &self.factors
    }
    pub fn rank(&self) -> usize {
        self.factors.iter().map(|f| f.simple.len()).sum()
    }
}

fn classify(component: &[usize], adjacency: &[Vec<usize>]) -> Result<Algebra, MathError> {
    let n = component.len();
    let edges: usize = component.iter().map(|&i| adjacency[i].len()).sum();
    if edges != 2 * (n - 1) {
        return Err(MathError::InvalidRootSystem);
    }
    let branches = component
        .iter()
        .copied()
        .filter(|&i| adjacency[i].len() > 2)
        .collect::<Vec<_>>();
    if branches.is_empty() {
        return Ok(Algebra::A(n));
    }
    if branches.len() != 1 || adjacency[branches[0]].len() != 3 {
        return Err(MathError::InvalidRootSystem);
    }
    let center = branches[0];
    let mut arms = Vec::new();
    for &neighbor in &adjacency[center] {
        let mut previous = center;
        let mut node = neighbor;
        let mut length = 1;
        while let Some(next) = adjacency[node].iter().copied().find(|&x| x != previous) {
            previous = node;
            node = next;
            length += 1;
        }
        arms.push(length);
    }
    arms.sort();
    match arms.as_slice() {
        [1, 1, _] => Ok(Algebra::D(n)),
        [1, 2, 2] => Ok(Algebra::E6),
        [1, 2, 3] => Ok(Algebra::E7),
        [1, 2, 4] => Ok(Algebra::E8),
        _ => Err(MathError::InvalidRootSystem),
    }
}
