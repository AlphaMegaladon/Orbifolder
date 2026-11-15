use super::{ModelError, integer::Smith};
use crate::math_core::{DnClass, Matrix, Rational, RightWeight, TorusVector, Vector, dn_class};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub point_elements: usize,
    pub fixed_classes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            point_elements: 4096,
            fixed_classes: 100_000,
        }
    }
}

/// Mutable input description; validated results expose no mutable fields.
#[derive(Clone, Debug)]
pub struct GeneratorSpec {
    pub order: u32,
    pub rotation: Matrix,
    pub translation: TorusVector,
    pub twist: RightWeight,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementSpec {
    pub factors: [u32; 3],
    pub translation: TorusVector,
}
#[derive(Clone, Debug)]
pub struct GeometrySpec {
    pub gram: Matrix,
    pub generators: [GeneratorSpec; 3],
    /// Optional coordinate/catalog conventions, certified independently.
    pub representatives: Option<Vec<Vec<ElementSpec>>>,
    pub centralizer_presentations: Option<Vec<Vec<ElementSpec>>>,
    pub limits: Limits,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SpaceGroupElement {
    factors: [u32; 3],
    translation: TorusVector,
}
impl SpaceGroupElement {
    pub fn factors(&self) -> [u32; 3] {
        self.factors
    }
    pub fn translation(&self) -> &TorusVector {
        &self.translation
    }
    pub fn spec(&self) -> ElementSpec {
        ElementSpec {
            factors: self.factors,
            translation: self.translation.clone(),
        }
    }
}

pub(super) fn subtract(a: &Matrix, b: &Matrix) -> Result<Matrix, ModelError> {
    if a.rows() != b.rows() || a.cols() != b.cols() {
        return Err(ModelError::InvalidRotation);
    }
    Ok(Matrix::new(
        a.rows(),
        a.cols(),
        a.entries()
            .iter()
            .zip(b.entries())
            .map(|(x, y)| x - y)
            .collect(),
    )?)
}
pub(super) fn columns(vectors: &[TorusVector]) -> Result<Matrix, ModelError> {
    Ok(Matrix::new(
        6,
        vectors.len(),
        (0..6)
            .flat_map(|i| vectors.iter().map(move |v| v.coordinates()[i].clone()))
            .collect(),
    )?)
}
fn integral(v: &TorusVector) -> bool {
    v.coordinates().iter().all(Rational::is_integer)
}

#[derive(Clone, Debug)]
pub struct SpaceGroup {
    gram: Matrix,
    generators: [GeneratorSpec; 3],
    orders: [u32; 3],
    rotations: Vec<Matrix>,
    sections: Vec<TorusVector>,
    points: Vec<[u32; 3]>,
    limits: Limits,
}
impl SpaceGroup {
    pub fn new(
        gram: Matrix,
        generators: [GeneratorSpec; 3],
        limits: Limits,
    ) -> Result<Self, ModelError> {
        if gram.rows() != 6 || gram.cols() != 6 || !gram.is_positive_definite() {
            return Err(ModelError::InvalidMetric);
        }
        let orders = std::array::from_fn(|i| generators[i].order);
        let count = orders
            .iter()
            .try_fold(1_usize, |n, &k| {
                if k == 0 {
                    None
                } else {
                    n.checked_mul(k as usize)
                }
            })
            .ok_or(ModelError::InvalidOrder)?;
        if count > limits.point_elements {
            return Err(ModelError::ResourceLimit);
        }
        let identity = Matrix::identity(6)?;
        for g in &generators {
            if g.rotation.rows() != 6
                || g.rotation.cols() != 6
                || !g.rotation.entries().iter().all(Rational::is_integer)
                || !gram.is_isometry(&g.rotation)?
            {
                return Err(ModelError::InvalidRotation);
            }
            let twist = g.twist.scale(&Rational::from(i128::from(g.order)));
            if dn_class(twist.coordinates())? != Some(DnClass::Zero) {
                return Err(ModelError::InvalidOrder);
            }
            let mut r = identity.clone();
            let mut t = TorusVector::zero();
            for _ in 0..g.order {
                t = t.add(&r.apply_vector(&g.translation)?);
                r = r.multiply(&g.rotation)?;
            }
            if r != identity || !integral(&t) {
                return Err(ModelError::InvalidRelations);
            }
        }
        for i in 0..3 {
            for j in i + 1..3 {
                let a = &generators[i];
                let b = &generators[j];
                if a.rotation.multiply(&b.rotation)? != b.rotation.multiply(&a.rotation)? {
                    return Err(ModelError::InvalidRelations);
                }
                let commutator = a
                    .translation
                    .add(&a.rotation.apply_vector(&b.translation)?)
                    .sub(&b.translation.add(&b.rotation.apply_vector(&a.translation)?));
                if !integral(&commutator) {
                    return Err(ModelError::InvalidRelations);
                }
            }
        }
        let mut rotations = Vec::with_capacity(count);
        let mut sections = Vec::with_capacity(count);
        let mut points = Vec::with_capacity(count);
        for m in 0..orders[0] {
            for n in 0..orders[1] {
                for k in 0..orders[2] {
                    let factors = [m, n, k];
                    let mut r = identity.clone();
                    let mut t = TorusVector::zero();
                    for i in 0..3 {
                        for _ in 0..factors[i] {
                            t = t.add(&r.apply_vector(&generators[i].translation)?);
                            r = r.multiply(&generators[i].rotation)?;
                        }
                    }
                    let twist = (0..3).fold(RightWeight::zero(), |v, i| {
                        v.add(
                            &generators[i]
                                .twist
                                .scale(&Rational::from(i128::from(factors[i]))),
                        )
                    });
                    if !super::character::consistent(&r, &twist)? {
                        return Err(ModelError::InvalidRotation);
                    }
                    rotations.push(r);
                    sections.push(t);
                    points.push(factors);
                }
            }
        }
        Ok(Self {
            gram,
            generators,
            orders,
            rotations,
            sections,
            points,
            limits,
        })
    }
    fn index(&self, f: [u32; 3]) -> Result<usize, ModelError> {
        if (0..3).any(|i| f[i] >= self.orders[i]) {
            return Err(ModelError::InvalidElement);
        }
        Ok(
            ((f[0] as usize * self.orders[1] as usize) + f[1] as usize) * self.orders[2] as usize
                + f[2] as usize,
        )
    }
    pub fn orders(&self) -> [u32; 3] {
        self.orders
    }
    pub fn gram(&self) -> &Matrix {
        &self.gram
    }
    pub fn generators(&self) -> &[GeneratorSpec; 3] {
        &self.generators
    }
    pub fn points(&self) -> &[[u32; 3]] {
        &self.points
    }
    pub fn rotation(&self, factors: [u32; 3]) -> Result<&Matrix, ModelError> {
        Ok(&self.rotations[self.index(factors)?])
    }
    pub fn section_translation(&self, factors: [u32; 3]) -> Result<&TorusVector, ModelError> {
        Ok(&self.sections[self.index(factors)?])
    }
    pub fn element(&self, spec: ElementSpec) -> Result<SpaceGroupElement, ModelError> {
        if !integral(
            &spec
                .translation
                .sub(self.section_translation(spec.factors)?),
        ) {
            return Err(ModelError::InvalidElement);
        }
        Ok(SpaceGroupElement {
            factors: spec.factors,
            translation: spec.translation,
        })
    }
    pub fn identity(&self) -> SpaceGroupElement {
        SpaceGroupElement {
            factors: [0; 3],
            translation: TorusVector::zero(),
        }
    }
    pub fn multiply(
        &self,
        a: &SpaceGroupElement,
        b: &SpaceGroupElement,
    ) -> Result<SpaceGroupElement, ModelError> {
        self.element(a.spec())?;
        self.element(b.spec())?;
        let factors = std::array::from_fn(|i| {
            ((u64::from(a.factors[i]) + u64::from(b.factors[i])) % u64::from(self.orders[i])) as u32
        });
        let translation = a
            .translation
            .add(&self.rotation(a.factors)?.apply_vector(&b.translation)?);
        self.element(ElementSpec {
            factors,
            translation,
        })
    }
    pub fn inverse(&self, g: &SpaceGroupElement) -> Result<SpaceGroupElement, ModelError> {
        self.element(g.spec())?;
        let factors = std::array::from_fn(|i| {
            if g.factors[i] == 0 {
                0
            } else {
                self.orders[i] - g.factors[i]
            }
        });
        let translation = self
            .rotation(factors)?
            .apply_vector(&g.translation)?
            .scale(&Rational::from(-1));
        self.element(ElementSpec {
            factors,
            translation,
        })
    }
    pub fn commutes(
        &self,
        g: &SpaceGroupElement,
        h: &SpaceGroupElement,
    ) -> Result<bool, ModelError> {
        Ok(self.multiply(g, h)? == self.multiply(h, g)?)
    }
    pub fn conjugate(
        &self,
        h: &SpaceGroupElement,
        g: &SpaceGroupElement,
    ) -> Result<SpaceGroupElement, ModelError> {
        self.multiply(&self.multiply(h, g)?, &self.inverse(h)?)
    }
    /// Whether the product of conjugacy classes contains the identity.
    /// Translation conjugations generate sum_i (1-R_i) Lambda. Finite point
    /// conjugations are enumerated modulo that exact integer image (Smith form).
    /// The first conjugation is fixed by simultaneous conjugation of the product.
    pub fn conjugacy_product_contains_identity(
        &self,
        elements: &[SpaceGroupElement],
        cap: usize,
    ) -> Result<bool, ModelError> {
        if elements.is_empty() {
            return Ok(true);
        }
        for g in elements {
            self.element(g.spec())?;
        }
        let factors = elements.iter().fold([0_u64; 3], |mut total, g| {
            for (i, x) in total.iter_mut().enumerate() {
                *x = (*x + u64::from(g.factors[i])) % u64::from(self.orders[i]);
            }
            total
        });
        if factors != [0; 3] {
            return Ok(false);
        }
        let mut vectors = Vec::new();
        for g in elements {
            let a = self.fixed_operator(g.factors)?;
            for j in 0..6 {
                vectors.push(TorusVector::new(std::array::from_fn(|i| {
                    a.get(i, j).unwrap().clone()
                })));
            }
        }
        let smith = Smith::new(&columns(&vectors)?)?;
        let mut current = vec![elements[0].clone()];
        let mut work = 0_usize;
        for g in &elements[1..] {
            let mut next = BTreeMap::new();
            for prefix in &current {
                for &p in &self.points {
                    work = work.checked_add(1).ok_or(ModelError::ResourceLimit)?;
                    if work > cap {
                        return Err(ModelError::ResourceLimit);
                    }
                    let h = self.element(ElementSpec {
                        factors: p,
                        translation: self.section_translation(p)?.clone(),
                    })?;
                    let product = self.multiply(prefix, &self.conjugate(&h, g)?)?;
                    let integral = product
                        .translation
                        .sub(self.section_translation(product.factors)?);
                    let key = smith
                        .torsion_key(integral.coordinates())?
                        .ok_or(ModelError::InvalidElement)?;
                    next.entry(key).or_insert(product);
                }
            }
            current = next.into_values().collect();
        }
        for product in current {
            if smith.solve(product.translation.coordinates())?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn fixed_operator(&self, f: [u32; 3]) -> Result<Matrix, ModelError> {
        subtract(&Matrix::identity(6)?, self.rotation(f)?)
    }
    pub fn fixed_locus(&self, g: &SpaceGroupElement) -> Result<FixedLocus, ModelError> {
        self.element(g.spec())?;
        let a = self.fixed_operator(g.factors)?;
        let solution = a.solve(g.translation.coordinates()).map_err(|e| {
            if e == crate::math_core::MathError::InconsistentSystem {
                ModelError::NoFixedLocus
            } else {
                e.into()
            }
        })?;
        let invariant = Smith::new(&a)?
            .kernel()
            .iter()
            .map(|v| Vector::from_slice(v))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FixedLocus {
            point: Vector::from_slice(solution.particular())?,
            invariant_lattice: invariant,
        })
    }
    pub fn centralizer(&self, g: &SpaceGroupElement) -> Result<Centralizer, ModelError> {
        self.element(g.spec())?;
        let a = self.fixed_operator(g.factors)?;
        let smith = Smith::new(&a)?;
        let mut representatives = Vec::new();
        for &factors in &self.points {
            let section = self.section_translation(factors)?;
            let rhs = self
                .fixed_operator(factors)?
                .apply_vector(&g.translation)?
                .sub(&a.apply_vector(section)?);
            if let Some(z) = smith.solve(rhs.coordinates())? {
                representatives.push(self.element(ElementSpec {
                    factors,
                    translation: section.add(&Vector::from_slice(&z)?),
                })?);
            }
        }
        let invariant_lattice = smith
            .kernel()
            .iter()
            .map(|v| Vector::from_slice(v))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Centralizer {
            representatives,
            invariant_lattice,
        })
    }
    fn orbit_key(
        &self,
        f: [u32; 3],
        translation: &TorusVector,
        smith: &Smith,
    ) -> Result<Vec<BigInt>, ModelError> {
        let a = self.fixed_operator(f)?;
        let section = self.section_translation(f)?;
        let mut best = None;
        for &h in &self.points {
            let conjugated = self
                .rotation(h)?
                .apply_vector(translation)?
                .add(&a.apply_vector(self.section_translation(h)?)?);
            let key = smith
                .torsion_key(conjugated.sub(section).coordinates())?
                .ok_or(ModelError::NoFixedLocus)?;
            if best.as_ref().is_none_or(|old| &key < old) {
                best = Some(key);
            }
        }
        best.ok_or(ModelError::InvalidOrder)
    }
    fn canonical_representatives(
        &self,
        f: [u32; 3],
    ) -> Result<(Smith, BTreeMap<Vec<BigInt>, SpaceGroupElement>), ModelError> {
        let smith = Smith::new(&self.fixed_operator(f)?)?;
        let section = self.section_translation(f)?;
        let offset = smith.left.apply(section.coordinates())?;
        if offset.iter().skip(smith.rank).any(|x| !x.is_integer()) {
            return Ok((smith, BTreeMap::new()));
        }
        let count = smith
            .diagonal
            .iter()
            .take(smith.rank)
            .try_fold(1_usize, |n, d| d.to_usize().and_then(|d| n.checked_mul(d)))
            .ok_or(ModelError::ResourceLimit)?;
        if count > self.limits.fixed_classes {
            return Err(ModelError::ResourceLimit);
        }
        let inverse = smith.left.inverse()?;
        let mut representatives = BTreeMap::new();
        for index in 0..count {
            let mut digits = index;
            let mut coordinates = vec![Rational::ZERO; 6];
            for i in 0..smith.rank {
                let d = smith.diagonal[i]
                    .to_usize()
                    .ok_or(ModelError::ResourceLimit)?;
                coordinates[i] = &offset[i] + Rational::from((digits % d) as i128);
                digits /= d;
            }
            let translation = Vector::from_slice(&inverse.apply(&coordinates)?)?;
            let key = self.orbit_key(f, &translation, &smith)?;
            representatives
                .entry(key)
                .or_insert(self.element(ElementSpec {
                    factors: f,
                    translation,
                })?);
        }
        Ok((smith, representatives))
    }
    /// Certify that a finite imported list generates the complete infinite group.
    fn certify_presentation(
        &self,
        g: &SpaceGroupElement,
        full: &Centralizer,
        list: &[SpaceGroupElement],
    ) -> Result<(), ModelError> {
        let mut generators = Vec::new();
        for h in list {
            if !self.commutes(g, h)? {
                return Err(ModelError::InvalidPresentation);
            }
            generators.push(h.clone());
            generators.push(self.inverse(h)?);
        }
        let mut words = BTreeMap::from([([0; 3], self.identity())]);
        let mut queue = VecDeque::from([[0; 3]]);
        let mut translations = BTreeSet::new();
        while let Some(f) = queue.pop_front() {
            let word = words[&f].clone();
            for h in &generators {
                let next = self.multiply(&word, h)?;
                if let Some(known) = words.get(&next.factors) {
                    let delta = next.translation.sub(&known.translation);
                    if delta != TorusVector::zero() {
                        translations.insert(delta);
                    }
                } else {
                    queue.push_back(next.factors);
                    words.insert(next.factors, next);
                }
            }
        }
        if words.len() != full.representatives.len() {
            return Err(ModelError::InvalidPresentation);
        }
        let image = Smith::new(&columns(&translations.into_iter().collect::<Vec<_>>())?)?;
        for v in &full.invariant_lattice {
            if image.solve(v.coordinates())?.is_none() {
                return Err(ModelError::InvalidPresentation);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixedLocus {
    point: TorusVector,
    invariant_lattice: Vec<TorusVector>,
}
impl FixedLocus {
    pub fn point(&self) -> &TorusVector {
        &self.point
    }
    pub fn invariant_lattice(&self) -> &[TorusVector] {
        &self.invariant_lattice
    }
    pub fn dimension(&self) -> usize {
        self.invariant_lattice.len()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Centralizer {
    representatives: Vec<SpaceGroupElement>,
    invariant_lattice: Vec<TorusVector>,
}
impl Centralizer {
    pub fn representatives(&self) -> &[SpaceGroupElement] {
        &self.representatives
    }
    pub fn invariant_lattice(&self) -> &[TorusVector] {
        &self.invariant_lattice
    }
    pub fn generators(&self, group: &SpaceGroup) -> Result<Vec<SpaceGroupElement>, ModelError> {
        let mut result = self
            .representatives
            .iter()
            .filter(|g| *g != &group.identity())
            .cloned()
            .collect::<Vec<_>>();
        for v in &self.invariant_lattice {
            result.push(group.element(ElementSpec {
                factors: [0; 3],
                translation: v.clone(),
            })?);
        }
        Ok(result)
    }
}
#[derive(Clone, Debug)]
pub struct FixedBrane {
    element: SpaceGroupElement,
    locus: FixedLocus,
    centralizer: Centralizer,
    presentation: Vec<SpaceGroupElement>,
}
impl FixedBrane {
    pub fn element(&self) -> &SpaceGroupElement {
        &self.element
    }
    pub fn locus(&self) -> &FixedLocus {
        &self.locus
    }
    pub fn centralizer(&self) -> &Centralizer {
        &self.centralizer
    }
    pub fn centralizer_presentation(&self) -> &[SpaceGroupElement] {
        &self.presentation
    }
}
#[derive(Clone, Debug)]
pub struct Sector {
    factors: [u32; 3],
    branes: Vec<FixedBrane>,
}
impl Sector {
    pub fn factors(&self) -> [u32; 3] {
        self.factors
    }
    pub fn fixed_branes(&self) -> &[FixedBrane] {
        &self.branes
    }
}

#[derive(Clone, Debug)]
pub struct WilsonRelations {
    orders: [Option<BigInt>; 6],
    aliases: Vec<Vec<usize>>,
    relations: Vec<TorusVector>,
}
impl WilsonRelations {
    fn new(group: &SpaceGroup) -> Result<Self, ModelError> {
        let mut relations = Vec::new();
        for generator in group.generators() {
            let a = subtract(&generator.rotation, &Matrix::identity(6)?)?;
            for j in 0..6 {
                relations.push(Vector::new(std::array::from_fn(|i| {
                    a.get(i, j).unwrap().clone()
                })));
            }
        }
        let smith = Smith::new(&columns(&relations)?)?;
        let units = (0..6)
            .map(|i| TorusVector::from_integers(std::array::from_fn(|j| i128::from(i == j))))
            .collect::<Vec<_>>();
        let mut orders = std::array::from_fn(|_| None);
        for i in 0..6 {
            orders[i] = smith.quotient_order(units[i].coordinates())?;
        }
        let mut aliases: Vec<Vec<usize>> = Vec::new();
        for i in 0..6 {
            if orders[i].as_ref().is_none_or(|o| o == &BigInt::from(1)) {
                continue;
            }
            let mut found = false;
            for alias in &mut aliases {
                if smith
                    .solve(units[i].sub(&units[alias[0]]).coordinates())?
                    .is_some()
                {
                    alias.push(i);
                    found = true;
                    break;
                }
            }
            if !found {
                aliases.push(vec![i]);
            }
        }
        aliases.retain(|a| a.len() > 1);
        Ok(Self {
            orders,
            aliases,
            relations,
        })
    }
    pub fn orders(&self) -> &[Option<BigInt>; 6] {
        &self.orders
    }
    pub fn aliases(&self) -> &[Vec<usize>] {
        &self.aliases
    }
    pub fn relations(&self) -> &[TorusVector] {
        &self.relations
    }
}

#[derive(Clone, Debug)]
pub struct Geometry {
    space: SpaceGroup,
    sectors: Vec<Sector>,
    wilson: WilsonRelations,
}
impl Geometry {
    pub fn build(spec: GeometrySpec) -> Result<Self, ModelError> {
        let space = SpaceGroup::new(spec.gram, spec.generators, spec.limits)?;
        if spec
            .representatives
            .as_ref()
            .is_some_and(|s| s.len() != space.points.len())
        {
            return Err(ModelError::IncompleteConstructions);
        }
        let mut sectors = Vec::new();
        let mut flat = 0;
        for (index, &factors) in space.points.iter().enumerate() {
            let (smith, expected) = space.canonical_representatives(factors)?;
            let chosen = if let Some(catalog) = &spec.representatives {
                catalog[index].clone()
            } else {
                expected.values().map(SpaceGroupElement::spec).collect()
            };
            let mut seen = BTreeSet::new();
            let mut branes = Vec::new();
            for element in chosen {
                if element.factors != factors {
                    return Err(ModelError::InvalidElement);
                }
                let element = space.element(element)?;
                let locus = space.fixed_locus(&element)?;
                let key = space.orbit_key(factors, &element.translation, &smith)?;
                if !expected.contains_key(&key) {
                    return Err(ModelError::InvalidElement);
                }
                if !seen.insert(key) {
                    return Err(ModelError::DuplicateConjugacyClass);
                }
                let centralizer = space.centralizer(&element)?;
                let presentation = if let Some(catalog) = &spec.centralizer_presentations {
                    let list = catalog
                        .get(flat)
                        .ok_or(ModelError::InvalidPresentation)?
                        .iter()
                        .cloned()
                        .map(|e| space.element(e))
                        .collect::<Result<Vec<_>, _>>()?;
                    space.certify_presentation(&element, &centralizer, &list)?;
                    list
                } else {
                    centralizer.generators(&space)?
                };
                branes.push(FixedBrane {
                    element,
                    locus,
                    centralizer,
                    presentation,
                });
                flat += 1;
            }
            if seen.len() != expected.len() {
                return Err(ModelError::IncompleteConstructions);
            }
            sectors.push(Sector { factors, branes });
        }
        if spec
            .centralizer_presentations
            .as_ref()
            .is_some_and(|s| s.len() != flat)
        {
            return Err(ModelError::InvalidPresentation);
        }
        let wilson = WilsonRelations::new(&space)?;
        Ok(Self {
            space,
            sectors,
            wilson,
        })
    }
    pub fn space_group(&self) -> &SpaceGroup {
        &self.space
    }
    pub fn sectors(&self) -> &[Sector] {
        &self.sectors
    }
    pub fn wilson_relations(&self) -> &WilsonRelations {
        &self.wilson
    }
}
