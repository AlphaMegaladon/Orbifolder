#![deny(unsafe_code)]
use num_bigint::BigInt;
use orbifolder_core::{
    analysis::Computation,
    execution::{self, ComputeOptions, Executor, ModelRequest, ModelResult, Outcome},
    physics_engine::TachyonSearch,
    susy::{self, AnalysisLimits, Monomial},
};
use orbifolder_data::{GeometryData, ModelData, RuleData, TheoryData};
use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
    types::{PyBool, PyDict, PyInt, PyList, PySequence, PyString, PyTuple},
};
use serde_json::{Value, json};
use std::sync::Arc;
mod scan;
pyo3::create_exception!(_native, InputError, PyValueError);
pyo3::create_exception!(_native, ModelError, PyValueError);
pyo3::create_exception!(_native, ExecutionError, PyRuntimeError);
fn input_error(e: impl std::fmt::Display) -> PyErr {
    InputError::new_err(e.to_string())
}
fn python_value(py: Python<'_>, object: &Bound<'_, PyAny>, depth: usize) -> PyResult<Value> {
    if depth > 64 {
        return Err(input_error("Input nesting exceeds 64 levels"));
    }
    if object.is_none() {
        return Ok(Value::Null);
    }
    if object.is_instance_of::<PyBool>() {
        return Ok(json!(object.extract::<bool>()?));
    }
    if object.is_instance_of::<PyInt>() {
        return Ok(orbifolder_data::integer(&object.extract::<BigInt>()?));
    }
    if object.is_instance_of::<PyString>() {
        return Ok(json!(object.extract::<String>()?));
    }
    if let Ok(dict) = object.cast::<PyDict>() {
        let mut out = serde_json::Map::new();
        for (k, v) in dict.iter() {
            out.insert(k.extract::<String>()?, python_value(py, &v, depth + 1)?);
        }
        return Ok(Value::Object(out));
    }
    if object.is_instance_of::<PyList>() || object.is_instance_of::<PyTuple>() {
        return Ok(Value::Array(
            object
                .try_iter()?
                .map(|o| python_value(py, &o?, depth + 1))
                .collect::<PyResult<Vec<_>>>()?,
        ));
    }
    if object.is_instance(&py.import("fractions")?.getattr("Fraction")?)? {
        return Ok(
            json!({"num":orbifolder_data::integer(&object.getattr("numerator")?.extract::<BigInt>()?),"den":orbifolder_data::integer(&object.getattr("denominator")?.extract::<BigInt>()?)}),
        );
    }
    Err(PyTypeError::new_err(
        "Exact input requires dict/list/tuple, integers, rational strings or Fraction; floats are rejected",
    ))
}
fn json_input(py: Python<'_>, object: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(python_value(py, object, 0)?.to_string())
}
fn json_output(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    value_output(
        py,
        &serde_json::from_str::<Value>(text).map_err(input_error)?,
    )
}
fn value_output(py: Python<'_>, v: &Value) -> PyResult<Py<PyAny>> {
    match v {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => b.into_py_any(py),
        Value::String(s) => s.into_py_any(py),
        Value::Number(n) => {
            if let Some(value) = n.as_i64() {
                value.into_py_any(py)
            } else if let Some(value) = n.as_u64() {
                value.into_py_any(py)
            } else {
                n.to_string()
                    .parse::<BigInt>()
                    .map_err(input_error)?
                    .into_py_any(py)
            }
        }
        Value::Array(a) => {
            let list = PyList::empty(py);
            for v in a {
                list.append(value_output(py, v)?)?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Object(o) => {
            let dict = PyDict::new(py);
            for (k, v) in o {
                dict.set_item(k, value_output(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}
fn core_error(py: Python<'_>, e: execution::ComputeError) -> PyErr {
    let data = orbifolder_data::error(&e);
    let err = ModelError::new_err(e.to_string());
    let _ = err
        .value(py)
        .setattr("category", data["category"].as_str().unwrap_or("model"));
    err
}
#[pyclass(name = "Geometry", frozen)]
struct PyGeometry {
    data: GeometryData,
    geometry: Arc<orbifolder_core::geometry::Geometry>,
    theory: TheoryData,
    rules: Vec<RuleData>,
}
#[pymethods]
impl PyGeometry {
    #[staticmethod]
    #[pyo3(signature=(data,*,theory_name="susy",rules=None))]
    fn from_data(
        py: Python<'_>,
        data: &Bound<'_, PyAny>,
        theory_name: &str,
        rules: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let data: GeometryData =
            serde_json::from_str(&json_input(py, data)?).map_err(input_error)?;
        let theory = theory(theory_name)?;
        let rules = rules
            .map(|value| {
                serde_json::from_str::<Vec<RuleData>>(&json_input(py, value)?).map_err(input_error)
            })
            .transpose()?
            .unwrap_or_default();
        let geometry = py.detach(|| data.build()).map_err(input_error)?;
        Ok(Self {
            data,
            geometry,
            theory,
            rules,
        })
    }
    #[staticmethod]
    fn available_presets() -> Vec<&'static str> {
        orbifolder_data::PRESET_NAMES.to_vec()
    }
    #[staticmethod]
    #[pyo3(signature=(name,*,theory_name="susy"))]
    fn from_preset(py: Python<'_>, name: &str, theory_name: &str) -> PyResult<Self> {
        let theory = theory(theory_name)?;
        let preset = orbifolder_data::preset(name, theory).map_err(input_error)?;
        let geometry = py.detach(|| preset.geometry.build()).map_err(input_error)?;
        Ok(Self {
            data: preset.geometry,
            geometry,
            theory,
            rules: preset.rules,
        })
    }
    #[pyo3(signature=(text,*,theory_name=None,rules=None,threads=None))]
    fn evaluate_legacy_text(
        &self,
        py: Python<'_>,
        text: &str,
        theory_name: Option<&str>,
        rules: Option<&Bound<'_, PyAny>>,
        threads: Option<usize>,
    ) -> PyResult<Vec<PyBatchItem>> {
        let models = parsed_models(py, text, self, theory_name, rules)?;
        let requests = models
            .iter()
            .map(|m| (*m.request).clone())
            .collect::<Vec<_>>();
        let executor = PyExecutor::new(py, threads)?;
        Ok(batch_results(
            py.detach(|| executor.inner.compute_many(&requests)),
        ))
    }
    #[pyo3(signature=(models,*,threads=None,anomalies=true))]
    fn evaluate_batch_summaries(
        &self,
        py: Python<'_>,
        models: Vec<Py<PyModel>>,
        threads: Option<usize>,
        anomalies: bool,
    ) -> PyResult<Py<PyAny>> {
        let data = models
            .iter()
            .map(|m| m.borrow(py).data.clone())
            .collect::<Vec<_>>();
        let requests = py
            .detach(|| {
                data.iter()
                    .map(|d| {
                        let mut r = d.request(Some(Arc::clone(&self.geometry)))?;
                        r.options.anomalies = anomalies;
                        Ok(r)
                    })
                    .collect::<Result<Vec<_>, orbifolder_data::DataError>>()
            })
            .map_err(input_error)?;
        let executor = PyExecutor::new(py, threads)?;
        let values = py.detach(|| summary_results(&executor.inner, &requests));
        value_output(py, &Value::Array(values))
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_output(py, &serde_json::to_string(&self.data).map_err(input_error)?)
    }
}
fn parsed_models(
    py: Python<'_>,
    text: &str,
    geometry: &PyGeometry,
    theory_name: Option<&str>,
    rules: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<PyModel>> {
    let theory = theory_name
        .map(theory)
        .transpose()?
        .unwrap_or(geometry.theory);
    let rules = match rules {
        Some(r) => {
            serde_json::from_str::<Vec<RuleData>>(&json_input(py, r)?).map_err(input_error)?
        }
        None => geometry.rules.clone(),
    };
    py.detach(|| {
        ModelData::parse_legacy_models(text, geometry.data.clone(), theory)?
            .into_iter()
            .map(|mut data| {
                data.rules = rules.clone();
                let request = Arc::new(data.request(Some(Arc::clone(&geometry.geometry)))?);
                Ok(PyModel { data, request })
            })
            .collect::<Result<Vec<_>, orbifolder_data::DataError>>()
    })
    .map_err(input_error)
}
#[pyfunction]
#[pyo3(signature=(text,geometry,*,theory_name=None,rules=None))]
fn parse_legacy_models(
    py: Python<'_>,
    text: &str,
    geometry: &PyGeometry,
    theory_name: Option<&str>,
    rules: Option<&Bound<'_, PyAny>>,
) -> PyResult<Vec<PyModel>> {
    parsed_models(py, text, geometry, theory_name, rules)
}
#[pyclass(name = "Model", frozen)]
struct PyModel {
    data: ModelData,
    request: Arc<ModelRequest>,
}
fn theory(s: &str) -> PyResult<TheoryData> {
    match s {
        "susy" => Ok(TheoryData::Susy),
        "non_susy" => Ok(TheoryData::NonSusy),
        _ => Err(input_error("Theory must be susy or non_susy")),
    }
}
#[pymethods]
impl PyModel {
    #[staticmethod]
    fn from_data(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let data = ModelData::from_json(&json_input(py, data)?).map_err(input_error)?;
        let request = py.detach(|| data.request(None)).map_err(input_error)?;
        Ok(Self {
            data,
            request: Arc::new(request),
        })
    }
    #[staticmethod]
    #[pyo3(signature=(text,geometry,theory_name=None,rules=None))]
    fn from_text(
        py: Python<'_>,
        text: &str,
        geometry: &PyGeometry,
        theory_name: Option<&str>,
        rules: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let mut data = ModelData::from_text(
            text,
            geometry.data.clone(),
            theory_name
                .map(theory)
                .transpose()?
                .unwrap_or(geometry.theory),
        )
        .map_err(input_error)?;
        if let Some(rules) = rules {
            data.rules = serde_json::from_str::<Vec<RuleData>>(&json_input(py, rules)?)
                .map_err(input_error)?;
        } else {
            data.rules = geometry.rules.clone();
        }
        let request = py
            .detach(|| data.request(Some(Arc::clone(&geometry.geometry))))
            .map_err(input_error)?;
        Ok(Self {
            data,
            request: Arc::new(request),
        })
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_output(py, &serde_json::to_string(&self.data).map_err(input_error)?)
    }
    #[getter]
    fn label(&self) -> &str {
        &self.data.label
    }
    #[pyo3(signature=(*,options=None,anomalies=None,search_nodes=None,solutions=None,tachyon_search=None))]
    #[allow(clippy::too_many_arguments)]
    fn compute(
        &self,
        py: Python<'_>,
        options: Option<&scan::PyComputeOptions>,
        anomalies: Option<bool>,
        search_nodes: Option<usize>,
        solutions: Option<usize>,
        tachyon_search: Option<&str>,
    ) -> PyResult<Py<PyAny>> {
        let mut request = (*self.request).clone();
        if let Some(options) = options {
            if anomalies.is_some()
                || search_nodes.is_some()
                || solutions.is_some()
                || tachyon_search.is_some()
            {
                return Err(input_error("pass options or legacy keywords, not both"));
            }
            request.options = options.inner.clone();
        } else {
            if let Some(value) = anomalies {
                request.options.anomalies = value;
            }
            if let Some(value) = search_nodes {
                request.options.limits.search_nodes = value;
            }
            if let Some(value) = solutions {
                request.options.limits.solutions = value;
            }
            if let Some(value) = tachyon_search {
                request.options.tachyons = scan::tachyon_mode(value)?;
            }
        }
        let result = py
            .detach(|| execution::evaluate(&request))
            .map_err(|failure| scan::scan_exception(py, &failure, options.is_none()))?;
        match result {
            Outcome::Complete(result) => Ok(Py::new(
                py,
                PyModelResult {
                    inner: Arc::from(result),
                },
            )?
            .into_any()),
            Outcome::Filtered(result) => Ok(Py::new(
                py,
                scan::PyFilteredResult {
                    data: scan::filtered_data(&result),
                },
            )?
            .into_any()),
        }
    }
}
#[pyclass(name = "Result", frozen)]
struct PyModelResult {
    inner: Arc<ModelResult>,
}
#[pymethods]
impl PyModelResult {
    fn summary(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let summary = py.detach(|| orbifolder_data::summary(&self.inner));
        value_output(py, &summary)
    }
    fn to_json(&self, py: Python<'_>) -> String {
        py.detach(|| orbifolder_data::result(&self.inner).to_string())
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_output(py, &self.to_json(py))
    }
    #[getter]
    fn massless_count(&self) -> usize {
        self.inner.spectrum().fields().len()
    }
    #[getter]
    fn tachyon_count(&self) -> usize {
        self.inner.spectrum().tachyonic().fields().len()
    }
    #[getter]
    fn fields(&self) -> Vec<PyField> {
        (0..self.massless_count())
            .map(|index| PyField {
                result: Arc::clone(&self.inner),
                index,
                tachyon: false,
            })
            .collect()
    }
    #[getter]
    fn tachyons(&self) -> Vec<PyField> {
        (0..self.tachyon_count())
            .map(|index| PyField {
                result: Arc::clone(&self.inner),
                index,
                tachyon: true,
            })
            .collect()
    }
    fn anomalies(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let v = py.detach(|| {
            self.inner
                .anomalies()
                .map(orbifolder_data::anomalies)
                .unwrap_or_else(|| json!({"status":"not_requested"}))
        });
        value_output(py, &v)
    }
    #[pyo3(signature=(trials,rules=None))]
    fn couplings(
        &self,
        py: Python<'_>,
        trials: Vec<Vec<usize>>,
        rules: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let rules = match rules {
            Some(r) => orbifolder_data::selection_rules(
                &serde_json::from_str::<Vec<RuleData>>(&json_input(py, r)?).map_err(input_error)?,
            )
            .map_err(input_error)?,
            None => self.inner.coupling_rules().clone(),
        };
        let result=py.detach(|| -> Result<Value,susy::SusyError> {match self.inner.susy(){Computation::Computed(view)=>{
            let monomials=trials.iter().map(|ids|Monomial::new(view,ids)).collect::<Result<Vec<_>,_>>()?;
            let reports=susy::analyze_couplings(view,&monomials,&rules,AnalysisLimits::default())?;
            Ok(json!({"status":"computed","trials":reports.iter().map(orbifolder_data::coupling).collect::<Vec<_>>()}))
        },other=>Ok(orbifolder_data::status(other,|_|Value::Null))}}).map_err(input_error)?;
        value_output(py, &result)
    }
    #[pyo3(signature=(field_pool,max_degree=3))]
    fn superpotential(
        &self,
        py: Python<'_>,
        field_pool: Vec<usize>,
        max_degree: u32,
    ) -> PyResult<Py<PyAny>> {
        let result = py
            .detach(|| -> Result<Value, susy::SusyError> {
                match self.inner.susy() {
                    Computation::Computed(view) => Ok(orbifolder_data::superpotential(
                        &susy::enumerate_superpotential(
                            view,
                            &field_pool,
                            max_degree,
                            self.inner.coupling_rules(),
                            AnalysisLimits::default(),
                        )?,
                    )),
                    other => Ok(orbifolder_data::status(other, |_| Value::Null)),
                }
            })
            .map_err(input_error)?;
        value_output(py, &result)
    }
}
#[pyclass(name = "Field", frozen)]
struct PyField {
    result: Arc<ModelResult>,
    index: usize,
    tachyon: bool,
}
#[pymethods]
impl PyField {
    #[getter]
    fn id(&self) -> usize {
        self.index
    }
    #[getter]
    fn is_tachyon(&self) -> bool {
        self.tachyon
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let field = if self.tachyon {
            &self.result.spectrum().tachyonic().fields()[self.index]
        } else {
            &self.result.spectrum().fields()[self.index]
        };
        value_output(py, &orbifolder_data::field(self.index, field))
    }
}
#[pyclass(name = "BatchItem", frozen)]
struct PyBatchItem {
    index: usize,
    value: Option<Arc<ModelResult>>,
    error: Option<Value>,
    filtered: Option<Value>,
}
#[pymethods]
impl PyBatchItem {
    #[getter]
    fn index(&self) -> usize {
        self.index
    }
    #[getter]
    fn ok(&self) -> bool {
        self.value.is_some()
    }
    #[getter]
    fn status(&self) -> &'static str {
        if self.ok() {
            "ok"
        } else if self.filtered.is_some() {
            "filtered"
        } else {
            "error"
        }
    }
    #[getter]
    fn filtered(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        value_output(py, self.filtered.as_ref().unwrap_or(&Value::Null))
    }
    #[getter]
    fn value(&self) -> Option<PyModelResult> {
        self.value.as_ref().map(|r| PyModelResult {
            inner: Arc::clone(r),
        })
    }
    #[getter]
    fn error(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        value_output(py, self.error.as_ref().unwrap_or(&Value::Null))
    }
    fn to_json(&self, py: Python<'_>) -> String {
        py.detach(|| {
            let mut value = json!({"index":self.index,"status":self.status(),"result":self.value.as_ref().map(|r|orbifolder_data::result(r)),"error":self.error});
            if let Some(filtered) = &self.filtered { value["filtered"] = filtered.clone(); }
            value.to_string()
        })
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_output(py, &self.to_json(py))
    }
}
#[pyclass(name = "Executor", frozen)]
struct PyExecutor {
    inner: Executor,
}
fn batch_results(results: Vec<Result<ModelResult, execution::ComputeError>>) -> Vec<PyBatchItem> {
    results
        .into_iter()
        .enumerate()
        .map(|(index, r)| match r {
            Ok(r) => PyBatchItem {
                index,
                value: Some(Arc::new(r)),
                error: None,
                filtered: None,
            },
            Err(e) => PyBatchItem {
                index,
                value: None,
                error: Some(orbifolder_data::error(&e)),
                filtered: None,
            },
        })
        .collect()
}
fn summary_results(executor: &Executor, requests: &[ModelRequest]) -> Vec<Value> {
    executor.compute_many_with(requests,|r|orbifolder_data::summary(&r)).into_iter().enumerate().map(|(index,r)|match r {
        Ok(summary)=>json!({"index":index,"status":"ok","summary":summary,"error":null}),
        Err(e)=>json!({"index":index,"status":"error","summary":null,"error":orbifolder_data::error(&e)}),
    }).collect()
}
#[pymethods]
impl PyExecutor {
    #[new]
    #[pyo3(signature=(threads=None))]
    fn new(py: Python<'_>, threads: Option<usize>) -> PyResult<Self> {
        let inner = py
            .detach(|| match threads {
                None => Ok(Executor::serial()),
                Some(n) => Executor::parallel(n),
            })
            .map_err(|e| ExecutionError::new_err(e.to_string()))?;
        Ok(Self { inner })
    }
    #[getter]
    fn threads(&self) -> usize {
        self.inner.threads()
    }
    #[pyo3(signature=(models,*,options=None))]
    fn compute_many(
        &self,
        py: Python<'_>,
        models: Vec<Py<PyAny>>,
        options: Option<&scan::PyComputeOptions>,
    ) -> Vec<PyBatchItem> {
        let requests = scan::requests(py, models, options);
        py.detach(|| {
            self.inner
                .map(&requests, |request| scan::evaluate_item(request, Arc::from))
        })
        .into_iter()
        .enumerate()
        .map(|(index, item)| PyBatchItem {
            index,
            value: item.value,
            error: item.error,
            filtered: item.filtered,
        })
        .collect()
    }
    #[pyo3(signature=(models,*,options=None))]
    fn compute_summaries(
        &self,
        py: Python<'_>,
        models: Vec<Py<PyAny>>,
        options: Option<&scan::PyComputeOptions>,
    ) -> PyResult<Py<PyAny>> {
        scan::project_batch(
            py,
            &self.inner,
            models,
            options,
            "summary",
            orbifolder_data::summary,
        )
    }
    #[pyo3(signature=(items,*,options=None))]
    fn compute_compact(
        &self,
        py: Python<'_>,
        items: Vec<Py<PyAny>>,
        options: Option<&scan::PyComputeOptions>,
    ) -> PyResult<Py<PyAny>> {
        scan::project_batch(
            py,
            &self.inner,
            items,
            options,
            "compact",
            orbifolder_data::compact_spectrum,
        )
    }
}
#[pymodule]
pub fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    scan::register(m)?;
    m.add_function(wrap_pyfunction!(parse_legacy_models, m)?)?;
    m.add_class::<PyGeometry>()?;
    m.add_class::<PyModel>()?;
    m.add_class::<PyModelResult>()?;
    m.add_class::<PyField>()?;
    m.add_class::<PyExecutor>()?;
    m.add_class::<PyBatchItem>()?;
    m.add("InputError", m.py().get_type::<InputError>())?;
    m.add("ModelError", m.py().get_type::<ModelError>())?;
    m.add("ExecutionError", m.py().get_type::<ExecutionError>())?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
