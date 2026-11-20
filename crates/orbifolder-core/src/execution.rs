//! Deterministic model-level execution. No global Rayon pool or nested workers.
use crate::{
    analysis::Computation,
    anomalies::{
        AnomalyAnalysis, AnomalyError, AnomalyOptions, AnomalyReport, analyze_spectrum_report,
    },
    geometry::{EmbeddingSpec, Geometry, ModelError, TheorySpec, embed},
    math_core::Rational,
    physics_engine::{
        PhysicsError, SolvedSectors, SolverLimits, Spectrum, TachyonSearch, calculate_spectrum,
        finish_prefiltered, solve_sectors, tachyon_prefilter,
    },
    susy::{SelectionRules, SusyError, SusySpectrum},
};
use rayon::prelude::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModelRequest {
    pub geometry: Arc<Geometry>,
    pub embedding: EmbeddingSpec,
    pub theory: TheorySpec,
    pub options: ComputeOptions,
    pub coupling_rules: SelectionRules,
}
#[derive(Clone, Debug)]
pub struct ComputeOptions {
    pub limits: SolverLimits,
    pub tachyons: TachyonSearch,
    pub anomalies: bool,
    pub stop_on_tachyon: bool,
    pub stop_on_anomaly: bool,
}
impl Default for ComputeOptions {
    fn default() -> Self {
        Self {
            limits: SolverLimits::default(),
            tachyons: TachyonSearch::Complete,
            anomalies: true,
            stop_on_tachyon: false,
            stop_on_anomaly: false,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeError {
    Model(ModelError),
    Physics(PhysicsError),
    Anomalies(AnomalyError),
    Susy(SusyError),
    InvalidOptions,
    Filtered(FilterReason),
}
impl From<ModelError> for ComputeError {
    fn from(e: ModelError) -> Self {
        Self::Model(e)
    }
}
impl From<PhysicsError> for ComputeError {
    fn from(e: PhysicsError) -> Self {
        Self::Physics(e)
    }
}
impl From<AnomalyError> for ComputeError {
    fn from(e: AnomalyError) -> Self {
        Self::Anomalies(e)
    }
}
impl From<SusyError> for ComputeError {
    fn from(e: SusyError) -> Self {
        Self::Susy(e)
    }
}
impl std::fmt::Display for ComputeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ComputeError {}
#[derive(Clone, Debug)]
pub struct ModelResult {
    solved: SolvedSectors,
    spectrum: Spectrum,
    anomalies: Option<AnomalyAnalysis>,
    susy: Computation<SusySpectrum>,
    coupling_rules: SelectionRules,
    provenance: Provenance,
}
impl ModelResult {
    pub fn solved(&self) -> &SolvedSectors {
        &self.solved
    }
    pub fn spectrum(&self) -> &Spectrum {
        &self.spectrum
    }
    pub fn anomalies(&self) -> Option<&AnomalyAnalysis> {
        self.anomalies.as_ref()
    }
    pub fn susy(&self) -> &Computation<SusySpectrum> {
        &self.susy
    }
    pub fn coupling_rules(&self) -> &SelectionRules {
        &self.coupling_rules
    }
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterReason {
    Tachyon,
    Anomaly,
}

#[derive(Clone, Debug, Default)]
pub struct Provenance {
    pub tachyon_search: Option<TachyonSearch>,
    pub tachyon_complete: bool,
    pub tachyon_excluded_by_theory: bool,
    pub minimum_mass: Option<Rational>,
    pub supercharges: Option<usize>,
    pub anomalies: Option<bool>,
    pub anomalies_requested: bool,
    pub massless_sectors_solved: bool,
    pub representations_assembled: bool,
    pub canonical_spectrum_assembled: bool,
    pub susy_view_computed: bool,
    pub stage: &'static str,
}
impl Provenance {
    pub fn tachyon_free(&self) -> Option<bool> {
        if self.minimum_mass.is_some() {
            Some(false)
        } else if self.tachyon_complete || self.tachyon_excluded_by_theory {
            Some(true)
        } else {
            None
        }
    }
}

pub struct ScanFailure {
    pub error: ComputeError,
    pub provenance: Provenance,
}

pub struct FilteredResult {
    pub reason: FilterReason,
    pub provenance: Provenance,
    pub anomalies: Option<AnomalyReport>,
}

pub enum Outcome {
    Complete(Box<ModelResult>),
    Filtered(Box<FilteredResult>),
}

pub fn compute(request: &ModelRequest) -> Result<ModelResult, ComputeError> {
    match evaluate(request).map_err(|failure| failure.error)? {
        Outcome::Complete(result) => Ok(*result),
        Outcome::Filtered(result) => Err(ComputeError::Filtered(result.reason)),
    }
}

pub fn evaluate(request: &ModelRequest) -> Result<Outcome, Box<ScanFailure>> {
    evaluate_with_spectrum(request, calculate_spectrum)
}

fn evaluate_with_spectrum(
    request: &ModelRequest,
    build_spectrum: impl FnOnce(&SolvedSectors, SolverLimits) -> Result<Spectrum, PhysicsError>,
) -> Result<Outcome, Box<ScanFailure>> {
    let options = &request.options;
    let mut provenance = Provenance {
        stage: "options",
        anomalies_requested: options.anomalies,
        ..Default::default()
    };
    let failure = |error, provenance: &Provenance| {
        Box::new(ScanFailure {
            error,
            provenance: provenance.clone(),
        })
    };
    if (options.stop_on_tachyon && options.tachyons == TachyonSearch::Off)
        || (options.stop_on_anomaly && !options.anomalies)
    {
        return Err(failure(ComputeError::InvalidOptions, &provenance));
    }
    provenance.stage = "embedding";
    let model = embed(
        Arc::clone(&request.geometry),
        request.embedding.clone(),
        request.theory,
    )
    .map_err(|error| failure(error.into(), &provenance))?;
    let susy = request.theory == TheorySpec::Susy;
    provenance.supercharges = Some(if susy { model.supercharges().len() } else { 0 });
    provenance.tachyon_excluded_by_theory = susy;
    if !susy && options.tachyons != TachyonSearch::Off {
        provenance.tachyon_search = Some(options.tachyons);
    }
    let solved = if options.stop_on_tachyon && !susy {
        provenance.stage = "tachyon_projection";
        match tachyon_prefilter(&model, options.tachyons, options.limits)
            .map_err(|error| failure(error.into(), &provenance))?
        {
            Err(mass) => {
                provenance.minimum_mass = Some(mass);
                return Ok(Outcome::Filtered(Box::new(FilteredResult {
                    reason: FilterReason::Tachyon,
                    provenance,
                    anomalies: None,
                })));
            }
            Ok(data) => {
                provenance.tachyon_complete = options.tachyons == TachyonSearch::Complete;
                provenance.stage = "massless_sectors";
                finish_prefiltered(&model, options.limits, data)
            }
        }
    } else {
        provenance.stage = "sectors";
        solve_sectors(&model, options.tachyons, options.limits)
    }
    .map_err(|error| failure(error.into(), &provenance))?;
    provenance.massless_sectors_solved = true;
    provenance.tachyon_complete = provenance.tachyon_search == Some(TachyonSearch::Complete);
    provenance.minimum_mass = solved
        .sectors()
        .iter()
        .flat_map(|sector| sector.branes())
        .flat_map(|brane| brane.tachyons())
        .map(|state| state.mass())
        .min()
        .cloned();
    provenance.stage = "representations";
    let spectrum = build_spectrum(&solved, options.limits)
        .map_err(|error| failure(error.into(), &provenance))?;
    provenance.representations_assembled = true;
    provenance.stage = "anomalies";
    let anomalies = if options.anomalies {
        let report = analyze_spectrum_report(&spectrum, AnomalyOptions::default())
            .map_err(|error| failure(error.into(), &provenance))?;
        let passed = report.is_anomaly_free();
        provenance.anomalies = Some(passed);
        if options.stop_on_anomaly && report.has_failed_checks() {
            return Ok(Outcome::Filtered(Box::new(FilteredResult {
                reason: FilterReason::Anomaly,
                provenance,
                anomalies: Some(report),
            })));
        }
        provenance.stage = "canonical_spectrum";
        let analysis = AnomalyAnalysis::from_report(&spectrum, report)
            .map_err(|error| failure(error.into(), &provenance))?;
        provenance.canonical_spectrum_assembled = true;
        Some(analysis)
    } else {
        None
    };
    provenance.stage = "complete";
    let susy = SusySpectrum::from_spectrum(&solved, &spectrum);
    provenance.susy_view_computed = matches!(susy, Computation::Computed(_));
    Ok(Outcome::Complete(Box::new(ModelResult {
        solved,
        spectrum,
        anomalies,
        susy,
        coupling_rules: request.coupling_rules.clone(),
        provenance,
    })))
}
#[derive(Debug)]
pub enum ExecutionError {
    InvalidThreads,
    Pool(String),
}
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ExecutionError {}
/// Explicit reusable pool. The serial path is genuinely serial, not a 1-thread pool.
pub struct Executor {
    pool: Option<rayon::ThreadPool>,
}
impl Executor {
    pub fn map<I: Sync, T: Send>(
        &self,
        items: &[I],
        run: impl Fn(&I) -> T + Sync + Send,
    ) -> Vec<T> {
        match &self.pool {
            None => items.iter().map(run).collect(),
            Some(pool) => pool.install(|| items.par_iter().map(run).collect()),
        }
    }
    pub fn serial() -> Self {
        Self { pool: None }
    }
    pub fn parallel(threads: usize) -> Result<Self, ExecutionError> {
        if threads == 0 {
            return Err(ExecutionError::InvalidThreads);
        }
        Ok(Self {
            pool: Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .map_err(|e| ExecutionError::Pool(e.to_string()))?,
            ),
        })
    }
    pub fn threads(&self) -> usize {
        self.pool
            .as_ref()
            .map_or(1, rayon::ThreadPool::current_num_threads)
    }
    pub fn compute_many(
        &self,
        requests: &[ModelRequest],
    ) -> Vec<Result<ModelResult, ComputeError>> {
        self.compute_many_with(requests, std::convert::identity)
    }
    /// Project inside each worker, releasing the complete result before taking
    /// another request. Scan summaries therefore retain only O(threads) spectra.
    pub fn compute_many_with<T: Send>(
        &self,
        requests: &[ModelRequest],
        project: impl Fn(ModelResult) -> T + Sync,
    ) -> Vec<Result<T, ComputeError>> {
        let run = |request: &ModelRequest| compute(request).map(&project);
        match &self.pool {
            None => requests.iter().map(run).collect(),
            Some(pool) => pool.install(|| requests.par_iter().map(run).collect()),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
