use super::{ElementSpec, Geometry, ModelError, SpaceGroup, SpaceGroupElement};
use crate::math_core::{
    GaugeLattice, GaugeVector, Matrix, Phase, Rational, RightWeight, TorusVector, Vector,
};
use num_bigint::BigInt;
use num_integer::Integer;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TheorySpec {
    Susy,
    NonSusy,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TorsionPolicy {
    Disabled,
    SusyPhaseContract,
    StoredNotApplied,
}

/// Alternating bicharacter on the space group's abelianization. Slots are
/// (m,n,k,l1,...,l6); quantization is checked against all generator relations.
#[derive(Clone, Debug)]
pub struct DiscreteTorsion {
    enabled: bool,
    coefficients: Matrix,
}
impl Default for DiscreteTorsion {
    fn default() -> Self {
        Self {
            enabled: false,
            coefficients: Matrix::zeros(9, 9).expect("fixed dimensions"),
        }
    }
}
impl DiscreteTorsion {
    pub fn new(enabled: bool, coefficients: Matrix) -> Result<Self, ModelError> {
        if coefficients.rows() != 9 || coefficients.cols() != 9 {
            return Err(ModelError::InvalidTorsion);
        }
        for i in 0..9 {
            if !coefficients.get(i, i).unwrap().is_integer() {
                return Err(ModelError::InvalidTorsion);
            }
            for j in i + 1..9 {
                if !(coefficients.get(i, j).unwrap() + coefficients.get(j, i).unwrap()).is_integer()
                {
                    return Err(ModelError::InvalidTorsion);
                }
            }
        }
        if !enabled && coefficients.entries().iter().any(|x| !x.is_zero()) {
            return Err(ModelError::InvalidTorsion);
        }
        Ok(Self {
            enabled,
            coefficients,
        })
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn coefficients(&self) -> &Matrix {
        &self.coefficients
    }
    fn validate(&self, group: &SpaceGroup) -> Result<(), ModelError> {
        if !self.enabled {
            return Ok(());
        }
        let mut relations = Vec::new();
        for i in 0..3 {
            let g = &group.generators()[i];
            let mut rotation = Matrix::identity(6)?;
            let mut translation = TorusVector::zero();
            for _ in 0..g.order {
                translation = translation.add(&rotation.apply_vector(&g.translation)?);
                rotation = rotation.multiply(&g.rotation)?;
            }
            let mut relation = vec![Rational::ZERO; 9];
            relation[i] = Rational::from(i128::from(g.order));
            for j in 0..6 {
                relation[3 + j] = -&translation.coordinates()[j];
            }
            relations.push(relation);
            for j in 0..6 {
                let mut relation = vec![Rational::ZERO; 9];
                for k in 0..6 {
                    relation[3 + k] =
                        g.rotation.get(k, j).unwrap() - Rational::from(i128::from(k == j));
                }
                relations.push(relation);
            }
            for j in i + 1..3 {
                let h = &group.generators()[j];
                let commutator = g
                    .translation
                    .add(&g.rotation.apply_vector(&h.translation)?)
                    .sub(&h.translation.add(&h.rotation.apply_vector(&g.translation)?));
                let mut relation = vec![Rational::ZERO; 3];
                relation.extend(commutator.coordinates().iter().cloned());
                relations.push(relation);
            }
        }
        for relation in relations {
            for j in 0..9 {
                let value = (0..9).fold(Rational::ZERO, |s, i| {
                    s + &relation[i] * self.coefficients.get(i, j).unwrap()
                });
                if !value.is_integer() {
                    return Err(ModelError::InvalidTorsion);
                }
            }
        }
        Ok(())
    }
    pub fn abstract_phase(
        &self,
        group: &SpaceGroup,
        g: &SpaceGroupElement,
        h: &SpaceGroupElement,
    ) -> Result<Phase, ModelError> {
        self.validate(group)?;
        group.element(g.spec())?;
        group.element(h.spec())?;
        if !self.enabled {
            return Ok(Phase::default());
        }
        let coordinates = |e: &SpaceGroupElement| -> Result<Vec<Rational>, ModelError> {
            let mut values = e.factors().map(|x| Rational::from(i128::from(x))).to_vec();
            values.extend(
                e.translation()
                    .sub(group.section_translation(e.factors())?)
                    .coordinates()
                    .iter()
                    .cloned(),
            );
            Ok(values)
        };
        Ok(Phase::new(
            self.coefficients
                .bilinear(&coordinates(g)?, &coordinates(h)?)?,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct EmbeddingSpec {
    pub label: String,
    pub lattice: GaugeLattice,
    pub shifts: [GaugeVector; 3],
    pub wilson_lines: [GaugeVector; 6],
    pub torsion: DiscreteTorsion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalElement {
    element: SpaceGroupElement,
    shift: GaugeVector,
    twist: RightWeight,
}
impl LocalElement {
    pub fn element(&self) -> &SpaceGroupElement {
        &self.element
    }
    pub fn shift(&self) -> &GaugeVector {
        &self.shift
    }
    pub fn twist(&self) -> &RightWeight {
        &self.twist
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModularCheck {
    left: usize,
    right: usize,
    prefactor: BigInt,
    value: Rational,
}
impl ModularCheck {
    pub fn indices(&self) -> (usize, usize) {
        (self.left, self.right)
    }
    pub fn prefactor(&self) -> &BigInt {
        &self.prefactor
    }
    pub fn value(&self) -> &Rational {
        &self.value
    }
}

/// Four independent CPT representatives of the SO(8) supercharge spinors.
pub fn invariant_supercharges(twists: &[RightWeight]) -> Vec<RightWeight> {
    (0..4)
        .rev()
        .map(|positive| {
            Vector::new(std::array::from_fn(|i| {
                Rational::new(if i == positive { 1 } else { -1 }, 2).expect("nonzero denominator")
            }))
        })
        .filter(|q| twists.iter().all(|v| q.dot(v).is_integer()))
        .collect()
}

#[derive(Clone, Debug)]
pub struct OrbifoldGroup {
    geometry: Arc<Geometry>,
    spec: EmbeddingSpec,
    theory: TheorySpec,
    elements: Vec<LocalElement>,
    centralizers: Vec<Vec<LocalElement>>,
    supercharges: Vec<RightWeight>,
    checks: Vec<ModularCheck>,
}
impl OrbifoldGroup {
    pub fn theory(&self) -> TheorySpec {
        self.theory
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    pub fn geometry_handle(&self) -> Arc<Geometry> {
        Arc::clone(&self.geometry)
    }
    pub fn label(&self) -> &str {
        &self.spec.label
    }
    pub fn lattice(&self) -> GaugeLattice {
        self.spec.lattice
    }
    pub fn shifts(&self) -> &[GaugeVector; 3] {
        &self.spec.shifts
    }
    pub fn wilson_lines(&self) -> &[GaugeVector; 6] {
        &self.spec.wilson_lines
    }
    pub fn elements(&self) -> &[LocalElement] {
        &self.elements
    }
    pub fn centralizers(&self) -> &[Vec<LocalElement>] {
        &self.centralizers
    }
    pub fn supercharges(&self) -> &[RightWeight] {
        &self.supercharges
    }
    pub fn modular_checks(&self) -> &[ModularCheck] {
        &self.checks
    }
    pub fn torsion(&self) -> &DiscreteTorsion {
        &self.spec.torsion
    }
    pub fn torsion_policy(&self) -> TorsionPolicy {
        if !self.spec.torsion.enabled {
            TorsionPolicy::Disabled
        } else if self.theory == TheorySpec::Susy {
            TorsionPolicy::SusyPhaseContract
        } else {
            TorsionPolicy::StoredNotApplied
        }
    }
    pub fn projection_torsion_phase(
        &self,
        g: &SpaceGroupElement,
        h: &SpaceGroupElement,
    ) -> Result<Phase, ModelError> {
        if self.torsion_policy() == TorsionPolicy::StoredNotApplied {
            return Err(ModelError::UnsupportedTorsionProjection);
        }
        self.spec
            .torsion
            .abstract_phase(self.geometry.space_group(), g, h)
    }
    pub fn local_element(&self, g: &SpaceGroupElement) -> Result<LocalElement, ModelError> {
        local(self.geometry.space_group(), &self.spec, g)
    }
}

fn wilson(vector: &TorusVector, lines: &[GaugeVector; 6]) -> GaugeVector {
    (0..6).fold(GaugeVector::zero(), |sum, i| {
        sum.add(&lines[i].scale(&vector.coordinates()[i]))
    })
}
fn local(
    group: &SpaceGroup,
    spec: &EmbeddingSpec,
    g: &SpaceGroupElement,
) -> Result<LocalElement, ModelError> {
    group.element(g.spec())?;
    let mut shift = wilson(g.translation(), &spec.wilson_lines);
    let mut twist = RightWeight::zero();
    for (i, &power) in g.factors().iter().enumerate() {
        let power = Rational::from(i128::from(power));
        shift = shift.add(&spec.shifts[i].scale(&power));
        twist = twist.add(&group.generators()[i].twist.scale(&power));
    }
    Ok(LocalElement {
        element: g.clone(),
        shift,
        twist,
    })
}

/// Validate a parent-lattice embedding. An Arc allows many independent models to
/// share a certified immutable geometry without global caches or mutations.
pub fn embedding_orders(geometry: &Geometry) -> [BigInt; 9] {
    let orders = geometry.space_group().orders();
    std::array::from_fn(|index| {
        if index < 3 {
            BigInt::from(orders[index])
        } else {
            geometry.wilson_relations().orders()[index - 3]
                .clone()
                .unwrap_or_else(|| BigInt::from(1))
        }
    })
}

pub(super) fn embedding_support(
    geometry: &Geometry,
    spec: &EmbeddingSpec,
) -> Result<(), ModelError> {
    let group = geometry.space_group();
    if spec.lattice == GaugeLattice::D8xD8 {
        return Err(ModelError::UnsupportedGaugeLattice);
    }
    // Roto-translations require separate Narain/embedding references; geometry
    // itself supports them, but this embedding contract covers pure twist lifts.
    if group
        .generators()
        .iter()
        .any(|g| g.translation != TorusVector::zero())
    {
        return Err(ModelError::UnsupportedEmbedding);
    }
    spec.torsion.validate(group)
}

pub fn validate_embedding(
    geometry: &Geometry,
    spec: &EmbeddingSpec,
) -> Result<Vec<ModularCheck>, ModelError> {
    validate_embedding_detailed(geometry, spec).map_err(|failure| failure.error)
}

#[derive(Debug)]
pub struct EmbeddingFailure {
    pub error: ModelError,
    pub slots: Vec<usize>,
    pub scaled_vector: Option<Box<GaugeVector>>,
}

pub fn validate_embedding_detailed(
    geometry: &Geometry,
    spec: &EmbeddingSpec,
) -> Result<Vec<ModularCheck>, Box<EmbeddingFailure>> {
    let failed = |error, slots, vector: Option<GaugeVector>| {
        Box::new(EmbeddingFailure {
            error,
            slots,
            scaled_vector: vector.map(Box::new),
        })
    };
    embedding_support(geometry, spec).map_err(|error| failed(error, Vec::new(), None))?;
    let group = geometry.space_group();
    let orders = embedding_orders(geometry);
    for (i, order) in orders[..3].iter().enumerate() {
        let scaled = spec.shifts[i].scale(&Rational::from(order.clone()));
        if !spec.lattice.contains(&scaled) {
            return Err(failed(
                ModelError::InvalidEmbeddingOrder,
                vec![i],
                Some(scaled),
            ));
        }
    }
    let relations = geometry.wilson_relations();
    for (i, order) in orders[3..].iter().enumerate() {
        if relations.orders()[i].is_none() && spec.wilson_lines[i] != GaugeVector::zero() {
            return Err(failed(
                ModelError::UnsupportedEmbedding,
                vec![i + 3],
                Some(spec.wilson_lines[i].clone()),
            ));
        }
        let scaled = spec.wilson_lines[i].scale(&Rational::from(order.clone()));
        if !spec.lattice.contains(&scaled) {
            return Err(failed(
                ModelError::InvalidEmbeddingOrder,
                vec![i + 3],
                Some(scaled),
            ));
        }
    }
    for relation in relations.relations() {
        let combined = wilson(relation, &spec.wilson_lines);
        if !spec.lattice.contains(&combined) {
            let slots = relation
                .coordinates()
                .iter()
                .enumerate()
                .filter_map(|(index, coefficient)| (!coefficient.is_zero()).then_some(index + 3))
                .collect();
            return Err(failed(ModelError::WilsonRelation, slots, Some(combined)));
        }
    }
    let vectors = spec
        .shifts
        .iter()
        .chain(&spec.wilson_lines)
        .collect::<Vec<_>>();
    let mut twists = group
        .generators()
        .iter()
        .map(|g| g.twist.clone())
        .collect::<Vec<_>>();
    twists.resize(9, RightWeight::zero());
    let mut checks = Vec::new();
    for i in 0..9 {
        for j in i..9 {
            let prefactor = orders[i].gcd(&orders[j]);
            let value = Rational::from(prefactor.clone())
                * (vectors[i].dot(vectors[j]) - twists[i].dot(&twists[j]));
            if !value.is_even_integer() {
                return Err(failed(
                    ModelError::ModularInvariance {
                        left: i,
                        right: j,
                        value,
                    },
                    vec![i, j],
                    None,
                ));
            }
            checks.push(ModularCheck {
                left: i,
                right: j,
                prefactor,
                value,
            });
        }
    }
    Ok(checks)
}

pub fn embed(
    geometry: impl Into<Arc<Geometry>>,
    spec: EmbeddingSpec,
    theory: TheorySpec,
) -> Result<OrbifoldGroup, ModelError> {
    let geometry = geometry.into();
    let checks = validate_embedding(&geometry, &spec)?;
    let group = geometry.space_group();
    let mut elements = Vec::new();
    let mut centralizers = Vec::new();
    for sector in geometry.sectors() {
        for brane in sector.fixed_branes() {
            elements.push(local(group, &spec, brane.element())?);
            centralizers.push(
                brane
                    .centralizer_presentation()
                    .iter()
                    .map(|h| local(group, &spec, h))
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
    }
    let twists = group
        .generators()
        .each_ref()
        .map(|generator| generator.twist.clone());
    let supercharges = invariant_supercharges(&twists);
    // A validated object is the status proof; no mutable CheckStatus flags.
    Ok(OrbifoldGroup {
        geometry,
        spec,
        theory,
        elements,
        centralizers,
        supercharges,
        checks,
    })
}

/// Explicit compatibility convention, separate from the three-factor core.
impl ElementSpec {
    pub fn from_susy(k: u32, l: u32, translation: TorusVector) -> Self {
        Self {
            factors: [k, l, 0],
            translation,
        }
    }
}
