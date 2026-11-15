//! Deterministic model-level execution. No global Rayon pool or nested workers.
use crate::{
    analysis::Computation,
    anomalies::{AnomalyAnalysis, AnomalyError, AnomalyOptions, analyze_anomalies},
    geometry::{EmbeddingSpec, Geometry, ModelError, TheorySpec, embed},
    physics_engine::{
        PhysicsError, SolvedSectors, SolverLimits, Spectrum, TachyonSearch, calculate_spectrum,
        solve_sectors,
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
}
impl Default for ComputeOptions {
    fn default() -> Self {
        Self {
            limits: SolverLimits::default(),
            tachyons: TachyonSearch::Complete,
            anomalies: true,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComputeError {
    Model(ModelError),
    Physics(PhysicsError),
    Anomalies(AnomalyError),
    Susy(SusyError),
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
}
pub fn compute(request: &ModelRequest) -> Result<ModelResult, ComputeError> {
    let model = embed(
        Arc::clone(&request.geometry),
        request.embedding.clone(),
        request.theory,
    )?;
    let solved = solve_sectors(&model, request.options.tachyons, request.options.limits)?;
    let susy = SusySpectrum::from_solved(&solved, request.options.limits)?;
    let spectrum = match &susy {
        Computation::Computed(s) => s.spectrum().clone(),
        _ => calculate_spectrum(&solved, request.options.limits)?,
    };
    let anomalies = if request.options.anomalies {
        Some(analyze_anomalies(&spectrum, AnomalyOptions::default())?)
    } else {
        None
    };
    Ok(ModelResult {
        solved,
        spectrum,
        anomalies,
        susy,
        coupling_rules: request.coupling_rules.clone(),
    })
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
