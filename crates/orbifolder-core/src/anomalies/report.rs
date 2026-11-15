use super::{
    AnomalyError, AnomalyTraces, BasisRotation, Computation, Handedness, MatterField,
    canonical_u1_basis, trace_matter,
};
use crate::{
    geometry::TheorySpec,
    math_core::{Algebra, GaugeVector, Rational, RootSystem},
    physics_engine::{Multiplet, Particle, Spectrum, SusyMultiplet},
};
use num_bigint::BigInt;

#[derive(Clone, Debug, Default)]
pub struct AnomalyOptions {
    pub non_abelian_levels: Option<Vec<Rational>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnomalyCheck {
    GaugeCubic { factor: usize },
    Witten { factor: usize },
    MixedUniversal { u1: usize, factor: usize },
    AbelianUniversal { indices: [usize; 3] },
    TracelessGravity { u1: usize },
    TracelessMixed { u1: usize, factor: usize },
    IndependentAbelianDirections,
    FiNormalization,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    kind: AnomalyCheck,
    status: Computation<bool>,
    actual: Option<Rational>,
    expected: Option<Rational>,
}
impl CheckResult {
    pub fn kind(&self) -> &AnomalyCheck {
        &self.kind
    }
    pub fn status(&self) -> &Computation<bool> {
        &self.status
    }
    pub fn actual(&self) -> Option<&Rational> {
        self.actual.as_ref()
    }
    pub fn expected(&self) -> Option<&Rational> {
        self.expected.as_ref()
    }
    pub fn passed(&self) -> bool {
        matches!(
            self.status,
            Computation::Computed(true) | Computation::NotApplicable(_)
        )
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiData {
    legacy_d0: Rational,
    norm: Rational,
    gs_trace_over24: Rational,
    supersymmetric_coefficient: Computation<Rational>,
}
impl FiData {
    pub fn legacy_d0(&self) -> &Rational {
        &self.legacy_d0
    }
    pub fn anomalous_norm_squared(&self) -> &Rational {
        &self.norm
    }
    pub fn gs_trace_over24(&self) -> &Rational {
        &self.gs_trace_over24
    }
    /// Exact coefficient of g_s^2 M_Pl^2/pi^2; no numerical pi is introduced.
    pub fn supersymmetric_coefficient(&self) -> &Computation<Rational> {
        &self.supersymmetric_coefficient
    }
}
#[derive(Clone, Debug)]
pub struct AnomalyReport {
    raw: AnomalyTraces,
    canonical: AnomalyTraces,
    rotation: BasisRotation,
    levels: Vec<Rational>,
    fi: FiData,
    checks: Vec<CheckResult>,
    anomaly_free: bool,
}
impl AnomalyReport {
    pub fn raw(&self) -> &AnomalyTraces {
        &self.raw
    }
    pub fn canonical(&self) -> &AnomalyTraces {
        &self.canonical
    }
    pub fn rotation(&self) -> &BasisRotation {
        &self.rotation
    }
    pub fn non_abelian_levels(&self) -> &[Rational] {
        &self.levels
    }
    pub fn fi(&self) -> &FiData {
        &self.fi
    }
    pub fn checks(&self) -> &[CheckResult] {
        &self.checks
    }
    /// Includes cancellation by the universal 4D GS term, rather than demanding
    /// that the anomalous-direction raw traces vanish.
    pub fn is_anomaly_free(&self) -> bool {
        self.anomaly_free
    }
}
#[derive(Clone, Debug)]
pub struct AnomalyAnalysis {
    report: AnomalyReport,
    spectrum: Spectrum,
}
impl AnomalyAnalysis {
    pub fn report(&self) -> &AnomalyReport {
        &self.report
    }
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }
}
pub fn analyze_anomalies(
    spectrum: &Spectrum,
    options: AnomalyOptions,
) -> Result<AnomalyAnalysis, AnomalyError> {
    // The spectrum contains CPT partners explicitly. Choose one independent
    // left-handed representative per pair, matching native CheckAnomaly.
    let mut matter = Vec::new();
    for f in spectrum.fields() {
        let active = match f.multiplet() {
            Multiplet::Susy(SusyMultiplet::LeftChiral | SusyMultiplet::LeftModulus)
            | Multiplet::Particle(Particle::LeftFermion) => true,
            Multiplet::Susy(SusyMultiplet::HalfHyper) => {
                return Err(AnomalyError::UnsupportedMultiplet);
            }
            _ => false,
        };
        if active {
            matter.push(MatterField::new(
                f.representations()
                    .iter()
                    .map(|r| r.dynkin_labels().to_vec())
                    .collect(),
                f.u1_charges().to_vec(),
                Handedness::Left,
                BigInt::from(1),
            )?);
        }
    }
    let report = analyze_matter(
        spectrum.basis().root_system(),
        spectrum.basis().u1_generators(),
        &matter,
        spectrum.theory(),
        options,
    )?;
    let rebased = spectrum.rebased(report.rotation.generators().to_vec())?;
    Ok(AnomalyAnalysis {
        report,
        spectrum: rebased,
    })
}
pub fn analyze_matter(
    system: &RootSystem<16>,
    generators: &[GaugeVector],
    matter: &[MatterField],
    theory: TheorySpec,
    options: AnomalyOptions,
) -> Result<AnomalyReport, AnomalyError> {
    let f = system.factors().len();
    let n = generators.len();
    let levels = options
        .non_abelian_levels
        .unwrap_or_else(|| vec![Rational::ONE; f]);
    if levels.len() != f
        || levels
            .iter()
            .any(|k| !k.is_integer() || k <= &Rational::ZERO)
    {
        return Err(AnomalyError::InvalidLevels);
    }
    let raw = trace_matter(system, generators, matter)?;
    let rotation = canonical_u1_basis(system, generators, raw.gravitational())?;
    let transformed = matter
        .iter()
        .map(|m| m.transformed(rotation.transformation()))
        .collect::<Result<Vec<_>, _>>()?;
    let canonical = trace_matter(system, rotation.generators(), &transformed)?;
    let a = rotation.anomalous_index();
    let trace = a.map_or(Rational::ZERO, |a| canonical.gravitational()[a].clone());
    let norm = a.map_or(Rational::ZERO, |a| {
        rotation.norm_squared(a).unwrap().clone()
    });
    let fi = FiData {
        legacy_d0: trace.clone(),
        norm: norm.clone(),
        gs_trace_over24: trace.checked_div(&Rational::from(24))?,
        supersymmetric_coefficient: if theory == TheorySpec::Susy {
            Computation::Computed(trace.checked_div(&Rational::from(192))?)
        } else {
            Computation::NotApplicable("No supersymmetric auxiliary D field in the non-SUSY theory")
        },
    };
    let mut checks = Vec::new();
    fn equality(
        checks: &mut Vec<CheckResult>,
        kind: AnomalyCheck,
        actual: Rational,
        expected: Rational,
    ) {
        checks.push(CheckResult {
            kind,
            status: Computation::Computed(actual == expected),
            actual: Some(actual),
            expected: Some(expected),
        });
    }
    for (g, factor) in system.factors().iter().enumerate() {
        let kind = AnomalyCheck::GaugeCubic { factor: g };
        if matches!(factor.algebra(),Algebra::A(r) if *r>=2) {
            equality(
                &mut checks,
                kind,
                canonical.gauge_cubic()[g].clone(),
                Rational::ZERO,
            );
        } else {
            checks.push(CheckResult {
                kind,
                status: Computation::NotApplicable("No local cubic gauge invariant"),
                actual: Some(Rational::ZERO),
                expected: Some(Rational::ZERO),
            });
        }
        let kind = AnomalyCheck::Witten { factor: g };
        if let Some(parity) = canonical.su2_parity()[g] {
            equality(
                &mut checks,
                kind,
                Rational::from(i128::from(parity)),
                Rational::ZERO,
            );
        } else {
            checks.push(CheckResult {
                kind,
                status: Computation::NotApplicable("Gauge factor is not SU(2)"),
                actual: None,
                expected: None,
            });
        }
    }
    let rank = canonical.independent_abelian_directions()?;
    equality(
        &mut checks,
        AnomalyCheck::IndependentAbelianDirections,
        Rational::from(BigInt::from(rank)),
        Rational::from(BigInt::from(rank.min(usize::from(a.is_some())))),
    );
    for i in 0..n {
        let delta = canonical.gravitational()[i].checked_div(&Rational::from(24))?;
        for (g, k) in levels.iter().enumerate() {
            equality(
                &mut checks,
                AnomalyCheck::MixedUniversal { u1: i, factor: g },
                canonical.mixed()[i][g].clone(),
                k * &delta,
            );
            if Some(i) != a {
                equality(
                    &mut checks,
                    AnomalyCheck::TracelessMixed { u1: i, factor: g },
                    canonical.mixed()[i][g].clone(),
                    Rational::ZERO,
                );
            }
        }
        if Some(i) != a {
            equality(
                &mut checks,
                AnomalyCheck::TracelessGravity { u1: i },
                canonical.gravitational()[i].clone(),
                Rational::ZERO,
            );
        }
        for j in i..n {
            for k in j..n {
                // Covariant GS tensor C_ijk = delta_i K_jk + cyclic; K=2G.
                let expected = (canonical.gravitational()[i].clone()
                    * rotation.gram().get(j, k).unwrap()
                    + canonical.gravitational()[j].clone() * rotation.gram().get(i, k).unwrap()
                    + canonical.gravitational()[k].clone() * rotation.gram().get(i, j).unwrap())
                .checked_div(&Rational::from(12))?;
                equality(
                    &mut checks,
                    AnomalyCheck::AbelianUniversal { indices: [i, j, k] },
                    canonical.abelian_cubic(i, j, k).unwrap().clone(),
                    expected,
                );
            }
        }
    }
    equality(
        &mut checks,
        AnomalyCheck::FiNormalization,
        trace,
        Rational::from(12) * norm,
    );
    let anomaly_free = checks.iter().all(CheckResult::passed);
    Ok(AnomalyReport {
        raw,
        canonical,
        rotation,
        levels,
        fi,
        checks,
        anomaly_free,
    })
}
