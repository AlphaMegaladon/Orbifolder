use super::{
    Excitation, MassShell, Mover, PhysicsError, SectorModes, ShellLattice, SolverLimits,
    enumerate_shell, shell::enumerate_ball,
};
use crate::{
    geometry::{
        ElementSpec, LocalElement, OrbifoldGroup, SpaceGroupElement, TheorySpec, TorsionPolicy,
    },
    math_core::{GaugeVector, Rational, RightWeight, TorusVector},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TachyonSearch {
    LegacyWindow,
    #[default]
    Complete,
    Off,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HalfState {
    shell: usize,
    indices: Vec<usize>,
    phases: Vec<Rational>,
}
impl HalfState {
    pub fn shell(&self) -> usize {
        self.shell
    }
    pub fn indices(&self) -> &[usize] {
        &self.indices
    }
    pub fn phases(&self) -> &[Rational] {
        &self.phases
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedState {
    left: HalfState,
    right: HalfState,
    gamma: Vec<Rational>,
    oscillator_contribution: RightWeight,
    mass: Rational,
}
impl ProjectedState {
    pub fn left(&self) -> &HalfState {
        &self.left
    }
    pub fn right(&self) -> &HalfState {
        &self.right
    }
    pub fn gamma(&self) -> &[Rational] {
        &self.gamma
    }
    pub fn oscillator_contribution(&self) -> &RightWeight {
        &self.oscillator_contribution
    }
    pub fn mass(&self) -> &Rational {
        &self.mass
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BraneStates {
    element: SpaceGroupElement,
    left_shells: Vec<MassShell<16>>,
    left_movers: Vec<HalfState>,
    states: Vec<ProjectedState>,
    tachyon_shells: Vec<Vec<MassShell<16>>>,
    tachyons: Vec<ProjectedState>,
}
impl BraneStates {
    pub fn element(&self) -> &SpaceGroupElement {
        &self.element
    }
    pub fn left_shells(&self) -> &[MassShell<16>] {
        &self.left_shells
    }
    pub fn left_movers(&self) -> &[HalfState] {
        &self.left_movers
    }
    pub fn states(&self) -> &[ProjectedState] {
        &self.states
    }
    /// Outer index is the right tachyon shell, not a mutable excitation flag.
    pub fn tachyon_shells(&self) -> &[Vec<MassShell<16>>] {
        &self.tachyon_shells
    }
    pub fn tachyons(&self) -> &[ProjectedState] {
        &self.tachyons
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectorKinematics {
    factors: [u32; 3],
    twist: RightWeight,
    modes: SectorModes,
    right_shells: Vec<MassShell<4>>,
    right_movers: Vec<HalfState>,
    tachyon_shells: Vec<MassShell<4>>,
    tachyon_movers: Vec<HalfState>,
    tachyon_masses: Vec<Rational>,
}
impl SectorKinematics {
    pub fn factors(&self) -> [u32; 3] {
        self.factors
    }
    pub fn twist(&self) -> &RightWeight {
        &self.twist
    }
    pub fn modes(&self) -> &SectorModes {
        &self.modes
    }
    pub fn right_shells(&self) -> &[MassShell<4>] {
        &self.right_shells
    }
    pub fn right_movers(&self) -> &[HalfState] {
        &self.right_movers
    }
    pub fn tachyon_shells(&self) -> &[MassShell<4>] {
        &self.tachyon_shells
    }
    pub fn tachyon_movers(&self) -> &[HalfState] {
        &self.tachyon_movers
    }
    pub fn tachyon_masses(&self) -> &[Rational] {
        &self.tachyon_masses
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectorStates {
    kinematics: SectorKinematics,
    branes: Vec<BraneStates>,
}
impl SectorStates {
    pub fn kinematics(&self) -> &SectorKinematics {
        &self.kinematics
    }
    pub fn factors(&self) -> [u32; 3] {
        self.kinematics.factors()
    }
    pub fn twist(&self) -> &RightWeight {
        self.kinematics.twist()
    }
    pub fn modes(&self) -> &SectorModes {
        self.kinematics.modes()
    }
    pub fn right_shells(&self) -> &[MassShell<4>] {
        self.kinematics.right_shells()
    }
    pub fn right_movers(&self) -> &[HalfState] {
        self.kinematics.right_movers()
    }
    pub fn tachyon_shells(&self) -> &[MassShell<4>] {
        self.kinematics.tachyon_shells()
    }
    pub fn tachyon_movers(&self) -> &[HalfState] {
        self.kinematics.tachyon_movers()
    }
    pub fn tachyon_masses(&self) -> &[Rational] {
        self.kinematics.tachyon_masses()
    }
    pub fn branes(&self) -> &[BraneStates] {
        &self.branes
    }
}

#[derive(Clone, Debug)]
pub struct SolvedSectors {
    model: OrbifoldGroup,
    sectors: Vec<SectorStates>,
    gamma_generators: Vec<LocalElement>,
}
impl SolvedSectors {
    pub fn model(&self) -> &OrbifoldGroup {
        &self.model
    }
    pub fn sectors(&self) -> &[SectorStates] {
        &self.sectors
    }
    pub fn gamma_generators(&self) -> &[LocalElement] {
        &self.gamma_generators
    }
}

fn group_phases<const N: usize>(
    shells: &[MassShell<N>],
    mut phase: impl FnMut(
        &crate::math_core::Vector<N>,
        &Excitation,
    ) -> Result<Option<Vec<Rational>>, PhysicsError>,
) -> Result<Vec<HalfState>, PhysicsError> {
    let mut out: Vec<HalfState> = Vec::new();
    for (si, shell) in shells.iter().enumerate() {
        let mut positions = BTreeMap::new();
        for (wi, w) in shell.weights().iter().enumerate() {
            let Some(phases) = phase(w, shell.excitation())? else {
                continue;
            };
            let next = out.len();
            let index = *positions.entry(phases.clone()).or_insert_with(|| {
                out.push(HalfState {
                    shell: si,
                    indices: Vec::new(),
                    phases,
                });
                next
            });
            out[index].indices.push(wi);
        }
    }
    Ok(out)
}

fn build_right_movers(
    shells: &[MassShell<4>],
    modes: &SectorModes,
    twists: &[RightWeight],
    theory: TheorySpec,
) -> Result<Vec<HalfState>, PhysicsError> {
    group_phases(shells, |q, e| {
        Ok(Some(
            twists
                .iter()
                .map(|v| {
                    let oscillator = e.transformation(modes.right(), v);
                    let sign = if theory == TheorySpec::Susy { 1 } else { -1 };
                    (q.dot(v) + Rational::from(sign) * oscillator).modulo_one()
                })
                .collect(),
        ))
    })
}
fn build_left_movers(
    shells: &[MassShell<16>],
    modes: &SectorModes,
    g: &LocalElement,
    centralizer: &[LocalElement],
    gamma: &[LocalElement],
    model: &OrbifoldGroup,
) -> Result<Vec<HalfState>, PhysicsError> {
    let half = Rational::new(1, 2)?;
    group_phases(shells, |p, e| {
        let phases = centralizer
            .iter()
            .chain(gamma)
            .map(|h| {
                let torsion = if model.torsion_policy() == TorsionPolicy::StoredNotApplied {
                    Rational::ZERO
                } else {
                    model
                        .projection_torsion_phase(g.element(), h.element())?
                        .value()
                        .clone()
                };
                Ok((p.dot(h.shift())
                    - &half * (g.shift().dot(h.shift()) - g.twist().dot(h.twist()))
                    + e.transformation(modes.left(), h.twist())
                    + torsion)
                    .modulo_one())
            })
            .collect::<Result<Vec<_>, PhysicsError>>()?;
        for i in 0..centralizer.len() {
            for j in i + 1..centralizer.len() {
                if centralizer[i].element().factors() == centralizer[j].element().factors()
                    && phases[i] != phases[j]
                {
                    return Ok(None);
                }
            }
        }
        Ok(Some(phases))
    })
}
fn right_phase(r: &HalfState, h: &LocalElement) -> Rational {
    h.element()
        .factors()
        .iter()
        .zip(&r.phases)
        .fold(Rational::ZERO, |s, (&n, v)| {
            s + Rational::from(i128::from(n)) * v
        })
        .modulo_one()
}
fn project(
    left: &[HalfState],
    right: &[HalfState],
    shells: &[MassShell<16>],
    modes: &SectorModes,
    centralizer: &[LocalElement],
    gamma: &[LocalElement],
    mass: &Rational,
) -> Vec<ProjectedState> {
    let mut out = Vec::new();
    for l in left {
        for r in right {
            if centralizer
                .iter()
                .enumerate()
                .all(|(i, h)| l.phases[i] == right_phase(r, h))
            {
                out.push(ProjectedState {
                    left: l.clone(),
                    right: r.clone(),
                    gamma: gamma
                        .iter()
                        .enumerate()
                        .map(|(i, h)| {
                            (right_phase(r, h) - &l.phases[centralizer.len() + i]).modulo_one()
                        })
                        .collect(),
                    oscillator_contribution: shells[l.shell].excitation.contribution(modes.left()),
                    mass: mass.clone(),
                });
            }
        }
    }
    out
}

type ShellCache = BTreeMap<(GaugeVector, Rational), Vec<GaugeVector>>;
fn build_left_shells(
    shift: &GaugeVector,
    excitations: Vec<Excitation>,
    model: &OrbifoldGroup,
    cache: &mut ShellCache,
    limits: SolverLimits,
) -> Result<Vec<MassShell<16>>, PhysicsError> {
    let mut out = Vec::new();
    for excitation in excitations {
        let key = (shift.clone(), excitation.target().clone());
        if !cache.contains_key(&key) {
            cache.insert(
                key.clone(),
                enumerate_shell(
                    shift,
                    excitation.target(),
                    ShellLattice::Gauge(model.lattice()),
                    limits,
                )?,
            );
        }
        let weights = cache[&key].clone();
        if !weights.is_empty() {
            out.push(MassShell {
                excitation,
                weights,
            });
        }
    }
    Ok(out)
}

pub fn solve_sectors(
    model: &OrbifoldGroup,
    search: TachyonSearch,
    limits: SolverLimits,
) -> Result<SolvedSectors, PhysicsError> {
    Ok(solve_pass(model, search, limits, None, false)?.0)
}

pub(crate) fn tachyon_prefilter(
    model: &OrbifoldGroup,
    search: TachyonSearch,
    limits: SolverLimits,
) -> Result<Result<SolvedSectors, Rational>, PhysicsError> {
    let (solved, mass) = solve_pass(model, search, limits, None, true)?;
    Ok(mass.map_or(Ok(solved), Err))
}

pub(crate) fn finish_prefiltered(
    model: &OrbifoldGroup,
    limits: SolverLimits,
    tachyons: SolvedSectors,
) -> Result<SolvedSectors, PhysicsError> {
    Ok(solve_pass(model, TachyonSearch::Off, limits, Some(tachyons), false)?.0)
}

fn solve_pass(
    model: &OrbifoldGroup,
    search: TachyonSearch,
    limits: SolverLimits,
    cached: Option<SolvedSectors>,
    tachyon_only: bool,
) -> Result<(SolvedSectors, Option<Rational>), PhysicsError> {
    let space = model.geometry().space_group();
    let twists = space.generators().each_ref().map(|g| g.twist.clone());
    let mut gamma = Vec::new();
    for i in 0..3 {
        if space.orders()[i] > 1 {
            let mut factors = [0; 3];
            factors[i] = 1;
            gamma.push(model.local_element(&space.element(ElementSpec {
                factors,
                translation: space.generators()[i].translation.clone(),
            })?)?);
        }
    }
    for i in 0..6 {
        gamma.push(model.local_element(&space.element(ElementSpec {
            factors: [0; 3],
            translation: TorusVector::from_integers(std::array::from_fn(|j| i128::from(i == j))),
        })?)?);
    }
    let mut sectors = Vec::new();
    let mut cache = ShellCache::new();
    let mut flat = 0;
    let mut cached = cached.map(|data| data.sectors.into_iter());
    for sector in model.geometry().sectors() {
        let twist = (0..3).fold(RightWeight::zero(), |s, i| {
            s.add(&twists[i].scale(&Rational::from(i128::from(sector.factors()[i]))))
        });
        let mut kinematics = solve_kinematics_pass(
            sector.factors(),
            twist,
            &twists,
            model.theory(),
            search,
            limits,
            !tachyon_only,
        )?;
        let mut cached_branes = cached.as_mut().map(|data| {
            let previous = data.next().expect("same geometry");
            kinematics.tachyon_shells = previous.kinematics.tachyon_shells;
            kinematics.tachyon_masses = previous.kinematics.tachyon_masses;
            kinematics.tachyon_movers = previous.kinematics.tachyon_movers;
            previous.branes.into_iter()
        });
        let modes = kinematics.modes();
        let right_movers = kinematics.right_movers();
        let tachyon_movers = kinematics.tachyon_movers();
        let tachyon_masses = kinematics.tachyon_masses();
        let mut branes = Vec::new();
        for _brane in sector.fixed_branes() {
            let g = &model.elements()[flat];
            let centralizer = &model.centralizers()[flat];
            let left_shells = if tachyon_only {
                Vec::new()
            } else {
                build_left_shells(
                    g.shift(),
                    modes.excitations(Mover::Left, &Rational::ZERO, limits)?,
                    model,
                    &mut cache,
                    limits,
                )?
            };
            let left_movers =
                build_left_movers(&left_shells, modes, g, centralizer, &gamma, model)?;
            let states = project(
                &left_movers,
                right_movers,
                &left_shells,
                modes,
                centralizer,
                &gamma,
                &Rational::ZERO,
            );
            let mut tachyons = Vec::new();
            let mut tachyon_left = Vec::new();
            if let Some(cached_branes) = &mut cached_branes {
                let previous = cached_branes.next().expect("same geometry");
                tachyons = previous.tachyons;
                tachyon_left = previous.tachyon_shells;
            } else {
                for (ti, mass) in tachyon_masses.iter().enumerate() {
                    let shells = build_left_shells(
                        g.shift(),
                        modes.excitations(Mover::Left, mass, limits)?,
                        model,
                        &mut cache,
                        limits,
                    )?;
                    let movers = build_left_movers(&shells, modes, g, centralizer, &gamma, model)?;
                    let right = tachyon_movers
                        .iter()
                        .filter(|r| r.shell == ti)
                        .cloned()
                        .collect::<Vec<_>>();
                    tachyons.extend(project(
                        &movers,
                        &right,
                        &shells,
                        modes,
                        centralizer,
                        &gamma,
                        mass,
                    ));
                    if tachyon_only && !tachyons.is_empty() {
                        return Ok((
                            SolvedSectors {
                                model: model.clone(),
                                sectors,
                                gamma_generators: gamma,
                            },
                            Some(mass.clone()),
                        ));
                    }
                    tachyon_left.push(shells);
                }
            }
            branes.push(BraneStates {
                element: g.element().clone(),
                left_shells,
                left_movers,
                states,
                tachyon_shells: tachyon_left,
                tachyons,
            });
            flat += 1;
        }
        sectors.push(SectorStates { kinematics, branes });
    }
    Ok((
        SolvedSectors {
            model: model.clone(),
            sectors,
            gamma_generators: gamma,
        },
        None,
    ))
}

/// Solve local right-moving kinematics without a gauge embedding or projection.
pub fn solve_kinematics(
    factors: [u32; 3],
    twist: RightWeight,
    twists: &[RightWeight; 3],
    theory: TheorySpec,
    search: TachyonSearch,
    limits: SolverLimits,
) -> Result<SectorKinematics, PhysicsError> {
    solve_kinematics_pass(factors, twist, twists, theory, search, limits, true)
}

fn solve_kinematics_pass(
    factors: [u32; 3],
    twist: RightWeight,
    twists: &[RightWeight; 3],
    theory: TheorySpec,
    search: TachyonSearch,
    limits: SolverLimits,
    massless: bool,
) -> Result<SectorKinematics, PhysicsError> {
    let modes = SectorModes::new(&twist, twists, factors, theory);
    let mut right_shells = Vec::new();
    let mut tachyon_shells = Vec::new();
    let mut tachyon_masses = Vec::new();
    for excitation in modes.excitations(Mover::Right, &Rational::ZERO, limits)? {
        if massless {
            let weights =
                enumerate_shell(&twist, excitation.target(), ShellLattice::RightSo8, limits)?;
            if !weights.is_empty() {
                right_shells.push(MassShell {
                    excitation: excitation.clone(),
                    weights,
                });
            }
        }
        if theory == TheorySpec::NonSusy && search != TachyonSearch::Off {
            let weights = enumerate_ball(
                &twist,
                excitation.target(),
                ShellLattice::RightSo8,
                false,
                limits,
            )?;
            // Split different negative mass levels before level matching.
            let mut by_mass: BTreeMap<Rational, Vec<RightWeight>> = BTreeMap::new();
            for q in weights {
                let mass = Rational::new(1, 2)? * q.norm_squared()
                    + excitation.energy()
                    + modes.vacuum().right();
                if mass >= Rational::ZERO {
                    continue;
                }
                if search == TachyonSearch::LegacyWindow {
                    let unshifted = q.sub(&twist);
                    if !unshifted.coordinates().iter().enumerate().all(|(i, x)| {
                        x.is_integer()
                            && x >= &Rational::from(-7)
                            && x <= &Rational::from(if i == 0 { 0 } else { 5 })
                    }) {
                        continue;
                    }
                }
                by_mass.entry(mass).or_default().push(q);
            }
            for (mass, weights) in by_mass {
                tachyon_masses.push(mass);
                tachyon_shells.push(MassShell {
                    excitation: excitation.clone(),
                    weights,
                });
            }
        }
    }
    let right_movers = build_right_movers(&right_shells, &modes, twists, theory)?;
    let tachyon_movers = build_right_movers(&tachyon_shells, &modes, twists, theory)?;
    Ok(SectorKinematics {
        factors,
        twist,
        modes,
        right_shells,
        right_movers,
        tachyon_shells,
        tachyon_movers,
        tachyon_masses,
    })
}
