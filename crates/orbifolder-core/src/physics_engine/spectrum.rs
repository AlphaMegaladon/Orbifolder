use super::{Excitation, PhysicsError, SolvedSectors, SolverLimits};
use crate::geometry::{SpaceGroupElement, TheorySpec};
use crate::math_core::{
    Algebra, DnClass, GaugeLattice, GaugeVector, LieFactor, Matrix, Rational, RightWeight,
    RootSystem, dynkin_labels, so8_class, weyl_reflect,
};
use num_bigint::BigInt;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SusyMultiplet {
    LeftChiral,
    RightChiral,
    Vector,
    ConjugateVector,
    HalfHyper,
    Gravity,
    ConjugateGravity,
    LeftModulus,
    RightModulus,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Particle {
    Gauge,
    ConjugateGauge,
    Scalar,
    ConjugateScalar,
    LeftFermion,
    RightFermion,
    Gravity,
    ConjugateGravity,
    Modulus,
    ConjugateModulus,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Multiplet {
    Susy(SusyMultiplet),
    Particle(Particle),
    Tachyon { excited: bool, conjugate: bool },
}

/// A basis convention, computed independently of any anomaly calculation.
/// The default U(1) basis is the rational RREF nullspace of the simple roots.
/// Changing this basis changes charge coordinates, not the physical spectrum.
#[derive(Clone, Debug)]
pub struct GaugeBasis {
    system: RootSystem<16>,
    u1: Vec<GaugeVector>,
}
impl GaugeBasis {
    pub fn from_solved(solved: &SolvedSectors) -> Result<Self, PhysicsError> {
        let model = solved.model();
        let constraints = model
            .shifts()
            .iter()
            .chain(model.wilson_lines())
            .cloned()
            .collect::<Vec<_>>();
        let roots = model.lattice().surviving_roots(&constraints);
        let chamber = match model.lattice() {
            GaugeLattice::D16Plus => {
                GaugeVector::from_integers([23, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0])
            }
            _ => GaugeVector::from_integers([23, 6, 5, 4, 3, 2, 1, 0, 23, 6, 5, 4, 3, 2, 1, 0]),
        };
        let system = RootSystem::new(roots, &chamber)?;
        let simple = system
            .factors()
            .iter()
            .flat_map(|f| f.simple_roots())
            .collect::<Vec<_>>();
        let matrix = Matrix::new(
            simple.len(),
            16,
            simple
                .iter()
                .flat_map(|r| r.coordinates().iter().cloned())
                .collect(),
        )?;
        let u1 = matrix
            .nullspace()?
            .into_iter()
            .map(|v| GaugeVector::from_slice(&v))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { system, u1 })
    }
    pub fn root_system(&self) -> &RootSystem<16> {
        &self.system
    }
    pub fn u1_generators(&self) -> &[GaugeVector] {
        &self.u1
    }
    /// Accept an explicit in-memory charge convention only if it spans exactly
    /// the abelian complement. No orthonormality or irrational normalization.
    pub fn with_u1_generators(self, u1: Vec<GaugeVector>) -> Result<Self, PhysicsError> {
        let rank = self.system.rank();
        if u1.len() != 16 - rank
            || u1
                .iter()
                .any(|u| self.system.roots().iter().any(|r| !u.dot(r).is_zero()))
        {
            return Err(PhysicsError::InvalidRepresentation);
        }
        let matrix = Matrix::new(
            u1.len(),
            16,
            u1.iter()
                .flat_map(|v| v.coordinates().iter().cloned())
                .collect(),
        )?;
        if matrix.rank()? != u1.len() {
            return Err(PhysicsError::InvalidRepresentation);
        }
        Ok(Self {
            system: self.system,
            u1,
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Representation {
    algebra: Algebra,
    dynkin: Vec<BigInt>,
    dimension: BigInt,
}
impl Representation {
    pub fn algebra(&self) -> &Algebra {
        &self.algebra
    }
    pub fn dynkin_labels(&self) -> &[BigInt] {
        &self.dynkin
    }
    /// Positive dimension; conjugacy is carried by the Dynkin labels.
    pub fn dimension(&self) -> &BigInt {
        &self.dimension
    }
}

/// Weyl's dimension formula over exact positive roots, valid for every ADE
/// highest weight. No hardcoded table of small representations.
pub fn representation_dimension(
    factor: &LieFactor<16>,
    highest: &GaugeVector,
) -> Result<BigInt, PhysicsError> {
    let labels = dynkin_labels(highest, factor.simple_roots())?;
    if labels.iter().any(|n| n < &BigInt::from(0)) {
        return Err(PhysicsError::InvalidRepresentation);
    }
    let simple_matrix = Matrix::new(
        factor.simple_roots().len(),
        16,
        factor
            .simple_roots()
            .iter()
            .flat_map(|v| v.coordinates().iter().cloned())
            .collect(),
    )?;
    let chamber = GaugeVector::from_slice(
        simple_matrix
            .solve(&vec![Rational::ONE; factor.simple_roots().len()])?
            .particular(),
    )?;
    let positive = factor
        .roots()
        .iter()
        .filter(|r| r.dot(&chamber) > Rational::ZERO)
        .collect::<Vec<_>>();
    let rho = positive
        .iter()
        .fold(GaugeVector::zero(), |s, r| s.add(r))
        .scale(&Rational::new(1, 2)?);
    let shifted = highest.add(&rho);
    let mut result = Rational::ONE;
    for root in positive {
        result = result * shifted.dot(root).checked_div(&rho.dot(root))?;
    }
    let dimension = result.to_integer()?;
    if dimension <= BigInt::from(0) {
        return Err(PhysicsError::InvalidRepresentation);
    }
    Ok(dimension)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    source: [usize; 3],
    element: SpaceGroupElement,
    weight_indices: Vec<usize>,
    highest: GaugeVector,
    representations: Vec<Representation>,
    charges: Vec<Rational>,
    multiplet: Multiplet,
    right_weights: Vec<RightWeight>,
    origin_classes: Vec<DnClass>,
    spacetime_classes: Vec<DnClass>,
    scalar_charge: Option<RightWeight>,
    left_excitation: Excitation,
    right_excitation: Excitation,
    gamma: Vec<Rational>,
    oscillator: RightWeight,
    mass: Rational,
}
impl Field {
    pub fn source(&self) -> [usize; 3] {
        self.source
    }
    pub fn element(&self) -> &SpaceGroupElement {
        &self.element
    }
    pub fn weight_indices(&self) -> &[usize] {
        &self.weight_indices
    }
    pub fn highest_weight(&self) -> &GaugeVector {
        &self.highest
    }
    pub fn representations(&self) -> &[Representation] {
        &self.representations
    }
    pub fn u1_charges(&self) -> &[Rational] {
        &self.charges
    }
    pub fn multiplet(&self) -> Multiplet {
        self.multiplet
    }
    pub fn right_weights(&self) -> &[RightWeight] {
        &self.right_weights
    }
    pub fn origin_classes(&self) -> &[DnClass] {
        &self.origin_classes
    }
    pub fn spacetime_classes(&self) -> &[DnClass] {
        &self.spacetime_classes
    }
    pub fn scalar_charge(&self) -> Option<&RightWeight> {
        self.scalar_charge.as_ref()
    }
    pub fn left_excitation(&self) -> &Excitation {
        &self.left_excitation
    }
    pub fn right_excitation(&self) -> &Excitation {
        &self.right_excitation
    }
    pub fn gamma(&self) -> &[Rational] {
        &self.gamma
    }
    pub fn oscillator_contribution(&self) -> &RightWeight {
        &self.oscillator
    }
    pub fn mass(&self) -> &Rational {
        &self.mass
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TachyonicSpectrum {
    fields: Vec<Field>,
}
impl TachyonicSpectrum {
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}
#[derive(Clone, Debug)]
pub struct Spectrum {
    theory: TheorySpec,
    basis: GaugeBasis,
    fields: Vec<Field>,
    tachyonic: TachyonicSpectrum,
}
impl Spectrum {
    /// New spectrum view with selected massless fields; the original is retained.
    /// Useful for effective matter content and anomaly diagnostics.
    pub fn select_fields(&self, mut keep: impl FnMut(&Field) -> bool) -> Self {
        Self {
            fields: self.fields.iter().filter(|f| keep(f)).cloned().collect(),
            ..self.clone()
        }
    }
    pub fn theory(&self) -> TheorySpec {
        self.theory
    }
    /// Immutable, exact change of charge coordinates. All field metadata and
    /// tachyon classifications survive; no states need to be solved again.
    pub fn rebased(&self, generators: Vec<GaugeVector>) -> Result<Self, PhysicsError> {
        let basis = self.basis.clone().with_u1_generators(generators)?;
        let rebase = |fields: &[Field]| {
            fields
                .iter()
                .cloned()
                .map(|mut f| {
                    f.charges = basis.u1.iter().map(|u| u.dot(&f.highest)).collect();
                    f
                })
                .collect()
        };
        Ok(Self {
            theory: self.theory,
            fields: rebase(&self.fields),
            tachyonic: TachyonicSpectrum {
                fields: rebase(&self.tachyonic.fields),
            },
            basis,
        })
    }
    pub fn basis(&self) -> &GaugeBasis {
        &self.basis
    }
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
    pub fn tachyonic(&self) -> &TachyonicSpectrum {
        &self.tachyonic
    }
}

fn dominant(
    mut w: GaugeVector,
    basis: &GaugeBasis,
    limits: SolverLimits,
) -> Result<GaugeVector, PhysicsError> {
    // Each reflection in a negative simple pairing increases the regular
    // chamber functional. The finite Weyl orbit guarantees termination.
    for _ in 0..limits.search_nodes {
        let negative = basis
            .system
            .factors()
            .iter()
            .flat_map(|f| f.simple_roots())
            .find(|a| w.dot(a) < Rational::ZERO);
        let Some(root) = negative else { return Ok(w) };
        w = weyl_reflect(&w, root)?;
    }
    Err(PhysicsError::ResourceLimit)
}
fn multiplet(
    weights: &[RightWeight],
    supercharges: &[RightWeight],
    tachyon: Option<bool>,
) -> Result<Multiplet, PhysicsError> {
    let half = Rational::new(1, 2)?;
    if let Some(excited) = tachyon {
        return Ok(Multiplet::Tachyon {
            excited,
            conjugate: weights[0].coordinates()[1..]
                .iter()
                .find(|x| !x.is_zero())
                .is_some_and(|x| x > &Rational::ZERO),
        });
    }
    if supercharges.is_empty() {
        let q = weights[0].coordinates();
        let particle = if q[0] == Rational::from(-1) {
            Particle::Gauge
        } else if q[0] == Rational::ONE {
            Particle::ConjugateGauge
        } else if q[0] == -&half {
            Particle::LeftFermion
        } else if q[0] == half {
            Particle::RightFermion
        } else if q[0].is_zero() {
            if q[1..]
                .iter()
                .find(|x| !x.is_zero())
                .is_some_and(|x| x > &Rational::ZERO)
            {
                Particle::ConjugateScalar
            } else {
                Particle::Scalar
            }
        } else {
            return Err(PhysicsError::UnsupportedSupermultiplet);
        };
        return Ok(Multiplet::Particle(particle));
    }
    let n = supercharges.len();
    if n > 2 || weights.len() != 1 << n {
        return Err(PhysicsError::UnsupportedSupermultiplet);
    }
    for (helicity, kind) in [
        (Rational::from(2), SusyMultiplet::ConjugateGravity),
        (Rational::from(-2), SusyMultiplet::Gravity),
        (Rational::ONE, SusyMultiplet::ConjugateVector),
        (Rational::from(-1), SusyMultiplet::Vector),
        (
            half.clone(),
            if n == 2 {
                SusyMultiplet::HalfHyper
            } else {
                SusyMultiplet::RightChiral
            },
        ),
        (-half, SusyMultiplet::LeftChiral),
    ] {
        if weights.iter().any(|w| w.coordinates()[0] == helicity) {
            return Ok(Multiplet::Susy(kind));
        }
    }
    Err(PhysicsError::UnsupportedSupermultiplet)
}
fn right_groups(
    weights: Vec<RightWeight>,
    supercharges: &[RightWeight],
) -> Result<Vec<Vec<RightWeight>>, PhysicsError> {
    if supercharges.len() > 2 {
        return Err(PhysicsError::UnsupportedSupermultiplet);
    }
    let mut used = vec![false; weights.len()];
    let mut out = Vec::new();
    for i in 0..weights.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let mut group = vec![weights[i].clone()];
        for mut combination in 1..3_usize.pow(supercharges.len() as u32) {
            let mut candidate = weights[i].clone();
            for q in supercharges.iter().rev() {
                match combination % 3 {
                    1 => candidate = candidate.add(q),
                    2 => candidate = candidate.sub(q),
                    _ => {}
                }
                combination /= 3;
            }
            if let Some(j) = (i + 1..weights.len()).find(|&j| !used[j] && weights[j] == candidate) {
                used[j] = true;
                group.push(weights[j].clone());
            }
        }
        out.push(group);
    }
    Ok(out)
}

/// Assemble fields from computed states, using a basis derived from the input
/// embedding. Neither oracle spectra nor anomaly-derived U(1) directions enter.
pub fn calculate_spectrum(
    solved: &SolvedSectors,
    limits: SolverLimits,
) -> Result<Spectrum, PhysicsError> {
    let basis = GaugeBasis::from_solved(solved)?;
    assemble_spectrum(solved, basis, limits)
}

pub fn spectrum_in_basis(
    solved: &SolvedSectors,
    basis: GaugeBasis,
    limits: SolverLimits,
) -> Result<Spectrum, PhysicsError> {
    let expected = GaugeBasis::from_solved(solved)?;
    if basis.system != expected.system {
        return Err(PhysicsError::InvalidRepresentation);
    }
    assemble_spectrum(solved, basis, limits)
}

fn assemble_spectrum(
    solved: &SolvedSectors,
    basis: GaugeBasis,
    limits: SolverLimits,
) -> Result<Spectrum, PhysicsError> {
    let mut fields = Vec::new();
    let mut tachyons = Vec::new();
    let mut dominant_cache = BTreeMap::new();
    let mut rep_cache = BTreeMap::new();
    for (si, sector) in solved.sectors().iter().enumerate() {
        for (bi, brane) in sector.branes().iter().enumerate() {
            for (tachyon, states) in [(false, brane.states()), (true, brane.tachyons())] {
                for (st, state) in states.iter().enumerate() {
                    let left = if tachyon {
                        &brane.tachyon_shells()[state.right().shell()][state.left().shell()]
                    } else {
                        &brane.left_shells()[state.left().shell()]
                    };
                    let right = if tachyon {
                        &sector.tachyon_shells()[state.right().shell()]
                    } else {
                        &sector.right_shells()[state.right().shell()]
                    };
                    let right_weights = state
                        .right()
                        .indices()
                        .iter()
                        .map(|&i| right.weights()[i].clone())
                        .collect::<Vec<_>>();
                    let qgroups = right_groups(right_weights, solved.model().supercharges())?;
                    let mut groups: Vec<(GaugeVector, Vec<usize>)> = Vec::new();
                    let mut positions = BTreeMap::new();
                    for &index in state.left().indices() {
                        let w = &left.weights()[index];
                        if !dominant_cache.contains_key(w) {
                            dominant_cache.insert(w.clone(), dominant(w.clone(), &basis, limits)?);
                        }
                        let highest = &dominant_cache[w];
                        let next = groups.len();
                        let position = *positions.entry(highest.clone()).or_insert_with(|| {
                            groups.push((highest.clone(), Vec::new()));
                            next
                        });
                        groups[position].1.push(index);
                    }
                    for (highest, weight_indices) in groups {
                        if !rep_cache.contains_key(&highest) {
                            let representations = basis
                                .system
                                .factors()
                                .iter()
                                .map(|f| {
                                    Ok(Representation {
                                        algebra: f.algebra().clone(),
                                        dynkin: dynkin_labels(&highest, f.simple_roots())?,
                                        dimension: representation_dimension(f, &highest)?,
                                    })
                                })
                                .collect::<Result<Vec<_>, PhysicsError>>()?;
                            rep_cache.insert(highest.clone(), representations);
                        }
                        let representations = &rep_cache[&highest];
                        let charges = basis.u1.iter().map(|u| u.dot(&highest)).collect::<Vec<_>>();
                        for q in &qgroups {
                            let mut kind = multiplet(
                                q,
                                solved.model().supercharges(),
                                tachyon.then_some(
                                    right.excitation().is_excited()
                                        || left.excitation().is_excited(),
                                ),
                            )?;
                            let singlet = representations
                                .iter()
                                .all(|r| r.dimension == BigInt::from(1))
                                && charges.iter().all(Rational::is_zero);
                            if singlet && sector.factors() == [0; 3] {
                                kind = match kind {
                                    Multiplet::Susy(SusyMultiplet::LeftChiral) => {
                                        Multiplet::Susy(SusyMultiplet::LeftModulus)
                                    }
                                    Multiplet::Susy(SusyMultiplet::RightChiral) => {
                                        Multiplet::Susy(SusyMultiplet::RightModulus)
                                    }
                                    Multiplet::Particle(Particle::Gauge) => {
                                        Multiplet::Particle(Particle::Gravity)
                                    }
                                    Multiplet::Particle(Particle::ConjugateGauge) => {
                                        Multiplet::Particle(Particle::ConjugateGravity)
                                    }
                                    Multiplet::Particle(Particle::Scalar) => {
                                        Multiplet::Particle(Particle::Modulus)
                                    }
                                    Multiplet::Particle(Particle::ConjugateScalar) => {
                                        Multiplet::Particle(Particle::ConjugateModulus)
                                    }
                                    _ => kind,
                                };
                            }
                            let scalar_charge = scalar_charge(q, solved.model().supercharges());
                            let origin_classes = q
                                .iter()
                                .map(|q| {
                                    so8_class(&q.sub(sector.twist()))
                                        .ok_or(PhysicsError::InvalidRepresentation)
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            let spin_shift = if solved.model().theory() == TheorySpec::NonSusy {
                                solved.model().geometry().space_group().generators()[0]
                                    .twist
                                    .scale(&Rational::from(i128::from(sector.factors()[0])))
                            } else {
                                RightWeight::zero()
                            };
                            let spacetime_classes = q
                                .iter()
                                .map(|q| {
                                    so8_class(&q.sub(sector.twist()).add(&spin_shift))
                                        .ok_or(PhysicsError::InvalidRepresentation)
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            let field = Field {
                                source: [si, bi, st],
                                element: brane.element().clone(),
                                weight_indices: weight_indices.clone(),
                                highest: highest.clone(),
                                representations: representations.clone(),
                                charges: charges.clone(),
                                multiplet: kind,
                                right_weights: q.clone(),
                                origin_classes,
                                spacetime_classes,
                                scalar_charge,
                                left_excitation: left.excitation().clone(),
                                right_excitation: right.excitation().clone(),
                                gamma: state.gamma().to_vec(),
                                oscillator: state.oscillator_contribution().clone(),
                                mass: state.mass().clone(),
                            };
                            let out = if tachyon { &mut tachyons } else { &mut fields };
                            if out.len() >= limits.solutions {
                                return Err(PhysicsError::ResourceLimit);
                            }
                            out.push(field);
                        }
                    }
                }
            }
        }
    }
    Ok(Spectrum {
        theory: solved.model().theory(),
        basis,
        fields,
        tachyonic: TachyonicSpectrum { fields: tachyons },
    })
}

fn scalar_charge(weights: &[RightWeight], supercharges: &[RightWeight]) -> Option<RightWeight> {
    let half = Rational::new(1, 2).expect("nonzero");
    if supercharges.len() == 1 {
        let mut result = None;
        for q in weights {
            result = if q == &RightWeight::new(std::array::from_fn(|_| -&half)) {
                Some(q.sub(&supercharges[0]))
            } else if q == &RightWeight::new(std::array::from_fn(|_| half.clone()))
                || q.coordinates()[0] == -&half
            {
                Some(q.add(&supercharges[0]))
            } else if q.coordinates()[0] == half {
                Some(q.sub(&supercharges[0]))
            } else {
                result
            };
        }
        result
    } else if supercharges.len() == 2 {
        weights
            .iter()
            .rev()
            .find(|q| q.coordinates()[0].is_zero())
            .cloned()
    } else {
        None
    }
}
