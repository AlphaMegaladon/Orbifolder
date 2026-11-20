use super::*;
use orbifolder_core::geometry::{
    PrefixError, PrefixMasker, PrefixSpec, embedding_orders, validate_embedding_detailed,
};
use orbifolder_core::math_core::{GaugeVector, Rational};
use orbifolder_data::Exact;
use std::collections::HashMap;

pub(super) fn tachyon_mode(mode: &str) -> PyResult<TachyonSearch> {
    match mode {
        "complete" => Ok(TachyonSearch::Complete),
        "legacy" => Ok(TachyonSearch::LegacyWindow),
        "off" => Ok(TachyonSearch::Off),
        _ => Err(input_error("Unknown tachyon search profile")),
    }
}

#[pyclass(name = "ComputeOptions", frozen)]
pub(super) struct PyComputeOptions {
    pub(super) inner: ComputeOptions,
}

#[pymethods]
impl PyComputeOptions {
    #[new]
    #[pyo3(signature=(*,search_nodes=10_000_000,solutions=1_000_000,tachyon_search="complete",anomalies=true,stop_on_tachyon=false,stop_on_anomaly=false))]
    fn new(
        search_nodes: usize,
        solutions: usize,
        tachyon_search: &str,
        anomalies: bool,
        stop_on_tachyon: bool,
        stop_on_anomaly: bool,
    ) -> PyResult<Self> {
        let tachyons = tachyon_mode(tachyon_search)?;
        if (stop_on_tachyon && tachyons == TachyonSearch::Off) || (stop_on_anomaly && !anomalies) {
            return Err(input_error("a prefilter requires its corresponding check"));
        }
        Ok(Self {
            inner: ComputeOptions {
                limits: orbifolder_core::physics_engine::SolverLimits {
                    search_nodes,
                    solutions,
                },
                tachyons,
                anomalies,
                stop_on_tachyon,
                stop_on_anomaly,
            },
        })
    }
}

#[pyclass(name = "CompactSpectrum", frozen)]
struct PyCompactSpectrum {
    data: Value,
}

#[pymethods]
impl PyCompactSpectrum {
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        compact_output(py, &self.data)
    }
}

fn compact_output(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    fn write<'py, 'data>(
        py: Python<'py>,
        value: &'data Value,
        strings: &mut HashMap<&'data str, Bound<'py, PyString>>,
    ) -> PyResult<Py<PyAny>> {
        match value {
            Value::Object(items) => {
                let dict = PyDict::new(py);
                for (key, value) in items {
                    let child = write(py, value, strings)?;
                    let key = strings.entry(key).or_insert_with(|| PyString::new(py, key));
                    dict.set_item(&*key, child)?;
                }
                Ok(dict.into_any().unbind())
            }
            Value::Array(items) => {
                let list = PyList::empty(py);
                for value in items {
                    list.append(write(py, value, strings)?)?;
                }
                Ok(list.into_any().unbind())
            }
            Value::String(value) => Ok(strings
                .entry(value)
                .or_insert_with(|| PyString::new(py, value))
                .clone()
                .into_any()
                .unbind()),
            _ => value_output(py, value),
        }
    }
    write(py, value, &mut HashMap::new())
}

#[pyclass(name = "FilteredResult", frozen)]
pub(super) struct PyFilteredResult {
    pub(super) data: Value,
}

#[pymethods]
impl PyFilteredResult {
    #[getter]
    fn status(&self) -> &'static str {
        "filtered"
    }
    #[getter]
    fn reason(&self) -> &str {
        self.data["reason"].as_str().unwrap()
    }
    fn to_data(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        value_output(py, &self.data)
    }
    fn provenance(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        value_output(py, &self.data["provenance"])
    }
}

#[pymethods]
impl PyModelResult {
    fn compact_spectrum(&self, py: Python<'_>) -> PyCompactSpectrum {
        PyCompactSpectrum {
            data: py.detach(|| orbifolder_data::compact_spectrum(&self.inner)),
        }
    }
    fn provenance(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let value = py.detach(|| {
            orbifolder_data::provenance(
                self.inner.provenance(),
                self.inner.anomalies().map(|analysis| analysis.report()),
            )
        });
        value_output(py, &value)
    }
}

pub(super) fn filtered_data(result: &execution::FilteredResult) -> Value {
    let reason = match result.reason {
        execution::FilterReason::Tachyon => "tachyon",
        execution::FilterReason::Anomaly => "anomaly",
    };
    json!({"status":"filtered","reason":reason,"massless_count":null,"tachyon_count":null,
           "provenance":orbifolder_data::provenance(&result.provenance, result.anomalies.as_ref()),
           "anomalies":result.anomalies.as_ref().map(orbifolder_data::anomaly_report)})
}

fn failure_data(failure: &execution::ScanFailure, legacy: bool) -> Value {
    let mut record = orbifolder_data::scan_error(&failure.error, failure.provenance.stage);
    if legacy {
        record["scan_category"] = record["category"].clone();
        record["category"] = orbifolder_data::error(&failure.error)["category"].clone();
    }
    record["provenance"] = orbifolder_data::provenance(&failure.provenance, None);
    record
}

pub(super) fn scan_exception(
    py: Python<'_>,
    failure: &execution::ScanFailure,
    legacy: bool,
) -> PyErr {
    let data = failure_data(failure, legacy);
    let error = ModelError::new_err(failure.error.to_string());
    for key in ["category", "scan_category", "code", "stage", "provenance"] {
        if let Some(value) = data.get(key) {
            if let Ok(native) = value_output(py, value) {
                let _ = error.value(py).setattr(key, native);
            }
        }
    }
    error
}

pub(super) struct BatchRequest {
    request: Result<ModelRequest, String>,
    legacy: bool,
}

pub(super) struct Tile<T> {
    pub(super) value: Option<T>,
    pub(super) error: Option<Value>,
    pub(super) filtered: Option<Value>,
}

pub(super) fn requests(
    py: Python<'_>,
    items: Vec<Py<PyAny>>,
    options: Option<&PyComputeOptions>,
) -> Vec<BatchRequest> {
    items
        .into_iter()
        .map(|item| {
            let bound = item.bind(py);
            let (mut request, legacy) = if let Ok(model) = bound.extract::<PyRef<'_, PyModel>>() {
                (Ok((*model.request).clone()), options.is_none())
            } else if let Ok(prepared) = bound.extract::<PyRef<'_, PyPreparedRequest>>() {
                (prepared.inner.clone(), false)
            } else {
                (Err("expected Model or ModelRequest".into()), false)
            };
            if let (Ok(request), Some(options)) = (&mut request, options) {
                request.options = options.inner.clone();
            }
            BatchRequest { request, legacy }
        })
        .collect()
}

pub(super) fn evaluate_item<T>(
    item: &BatchRequest,
    project: impl FnOnce(Box<ModelResult>) -> T,
) -> Tile<T> {
    match &item.request {
        Err(message) => Tile {
            value: None,
            error: Some(json!({
                "status":"error","category":"input_error","stage":"request","code":"invalid_request","message":message
            })),
            filtered: None,
        },
        Ok(request) => match execution::evaluate(request) {
            Ok(Outcome::Complete(result)) => Tile {
                value: Some(project(result)),
                error: None,
                filtered: None,
            },
            Ok(Outcome::Filtered(result)) => Tile {
                value: None,
                error: None,
                filtered: Some(filtered_data(&result)),
            },
            Err(failure) => Tile {
                value: None,
                error: Some(failure_data(&failure, item.legacy)),
                filtered: None,
            },
        },
    }
}

pub(super) fn project_batch(
    py: Python<'_>,
    executor: &Executor,
    models: Vec<Py<PyAny>>,
    options: Option<&PyComputeOptions>,
    field: &str,
    project: fn(&ModelResult) -> Value,
) -> PyResult<Py<PyAny>> {
    let mut requests = requests(py, models, options);
    if field == "compact" {
        requests
            .iter_mut()
            .for_each(|request| request.legacy = false);
    }
    let rows = py.detach(|| {
        executor.map(&requests, |request| {
            evaluate_item(request, |result| project(&result))
        })
    });
    let values = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let status = if row.value.is_some() {
                "ok"
            } else if row.filtered.is_some() {
                "filtered"
            } else {
                "error"
            };
            let mut data = json!({"index":index,"status":status,"error":row.error});
            data[field] = row.value.unwrap_or(Value::Null);
            if let Some(filtered) = row.filtered {
                data["filtered"] = filtered;
            }
            data
        })
        .collect();
    if field == "compact" {
        compact_output(py, &Value::Array(values))
    } else {
        value_output(py, &Value::Array(values))
    }
}

fn exact_list(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Vec<Exact>> {
    serde_json::from_str(&json_input(py, value)?).map_err(input_error)
}

fn prefix_error(py: Python<'_>, error: PrefixError) -> PyErr {
    match error {
        PrefixError::Input(detail) => input_error(detail),
        PrefixError::ResourceLimit => {
            let result = ExecutionError::new_err("prefix reachability state limit");
            let _ = result.value(py).setattr("category", "resource_limit");
            let _ = result.value(py).setattr("code", "prefix_state_limit");
            result
        }
        PrefixError::Model(error) => core_error(py, execution::ComputeError::Model(error)),
    }
}

#[pyclass(name = "PrefixMasker", frozen)]
struct PyPrefixMasker {
    inner: PrefixMasker,
}

impl PyPrefixMasker {
    fn actions(&self, py: Python<'_>, prefix: &Bound<'_, PySequence>) -> PyResult<Vec<usize>> {
        let length = prefix.len()?;
        if length > 16 {
            return Err(input_error("prefix exceeds sixteen coordinates"));
        }
        let mut actions = Vec::with_capacity(length);
        for index in 0..length {
            let value = prefix.get_item(index)?;
            let action = if value.is_instance_of::<PyInt>() && !value.is_instance_of::<PyBool>() {
                if let Ok(integer) = value.extract::<i64>() {
                    self.inner.integer_action_index(integer)
                } else {
                    self.inner
                        .action_index(&Rational::from(value.extract::<BigInt>()?))
                }
            } else {
                let exact: Exact =
                    serde_json::from_str(&json_input(py, &value)?).map_err(input_error)?;
                self.inner.action_index(&exact.0)
            };
            actions.push(action.ok_or_else(|| input_error("action outside alphabet"))?);
        }
        Ok(actions)
    }
}

#[pymethods]
impl PyPrefixMasker {
    #[getter]
    fn states(&self) -> usize {
        self.inner.states()
    }

    fn mask(&self, py: Python<'_>, prefix: &Bound<'_, PySequence>) -> PyResult<Vec<bool>> {
        self.inner
            .mask(&self.actions(py, prefix)?)
            .map_err(|error| prefix_error(py, error))
    }

    fn is_feasible(&self, py: Python<'_>, prefix: &Bound<'_, PySequence>) -> PyResult<bool> {
        self.inner
            .is_feasible(&self.actions(py, prefix)?)
            .map_err(|error| prefix_error(py, error))
    }
}

#[pyclass(name = "ModelRequest", frozen)]
pub(super) struct PyPreparedRequest {
    pub(super) inner: std::result::Result<ModelRequest, String>,
}

#[pymethods]
impl PyGeometry {
    fn embedding_profile(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let relations = self.geometry.wilson_relations();
        let orders = embedding_orders(&self.geometry);
        let twists = self
            .geometry
            .space_group()
            .generators()
            .each_ref()
            .map(|generator| &generator.twist);
        let dot = |left: usize, right: usize| {
            if right < 3 {
                twists[left].dot(twists[right])
            } else {
                Rational::ZERO
            }
        };
        let congruences = (0..9).flat_map(|left| (left..9).map(move |right| (left, right)))
            .map(|(left, right)| {
                let coefficient = num_integer::Integer::gcd(&orders[left], &orders[right]);
                json!({"indices":[left,right],"coefficient":orbifolder_data::integer(&coefficient),
                       "offset":orbifolder_data::rational(&(dot(left,right) * Rational::from(coefficient))),
                       "modulus":2})
            }).collect::<Vec<_>>();
        value_output(
            py,
            &json!({
                "schema_version":1,"shift_orders":self.geometry.space_group().orders(),
                "wilson_orders":relations.orders().iter().map(|order| order.as_ref().map(orbifolder_data::integer)).collect::<Vec<_>>(),
                "wilson_aliases":relations.aliases(),
                "wilson_relations":relations.relations().iter().map(orbifolder_data::vector).collect::<Vec<_>>(),
                "wilson_quotient":{"basis_columns":orbifolder_data::matrix(relations.quotient_basis()),
                    "coordinate_transform":orbifolder_data::matrix(relations.quotient_coordinates()),
                    "orders":relations.quotient_orders().iter().map(|order| order.as_ref().map(orbifolder_data::integer)).collect::<Vec<_>>(),
                    "free_direction_policy":"unbounded_slot_zero_required"},
                "twist_dot_products":(0..3).map(|left| (0..3).map(|right| orbifolder_data::rational(&twists[left].dot(twists[right]))).collect::<Vec<_>>()).collect::<Vec<_>>(),
                "modular_congruences":congruences,
            }),
        )
    }
}

impl PyGeometry {
    fn read_embedding(&self, py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<ModelData> {
        self.parse_embedding(python_value(py, value, 0)?)
    }

    fn parse_embedding(&self, mut fields: Value) -> PyResult<ModelData> {
        let dict = fields
            .as_object_mut()
            .ok_or_else(|| input_error("expected structured embedding"))?;
        if dict
            .get("theory_name")
            .is_some_and(|value| !value.is_null())
        {
            let value = dict.remove("theory_name").unwrap();
            let name = value
                .as_str()
                .ok_or_else(|| input_error("invalid theory name"))?;
            dict.insert("theory".into(), json!(theory(name)?));
        }
        dict.remove("theory_name");
        if dict.get("rules").is_some_and(Value::is_null) {
            dict.remove("rules");
        }
        if let Some(geometry) = dict.remove("geometry") {
            if geometry != serde_json::to_value(&self.data).map_err(input_error)? {
                return Err(input_error(
                    "embedding geometry disagrees with the prepared geometry",
                ));
            }
        }
        dict.entry("schema_version").or_insert(json!(1));
        dict.entry("theory").or_insert(json!(self.theory));
        dict.entry("rules").or_insert_with(|| json!(self.rules));
        dict.entry("label").or_insert(json!(""));
        ModelData::from_prepared_data(fields, &self.data).map_err(input_error)
    }

    fn structured_model(&self, py: Python<'_>, fields: Value) -> PyResult<PyModel> {
        let data = self.parse_embedding(fields)?;
        let request = py
            .detach(|| data.request_prepared(Arc::clone(&self.geometry), data.embedding()?))
            .map_err(input_error)?;
        Ok(PyModel {
            data,
            request: Arc::new(request),
        })
    }
}

#[pymethods]
impl PyGeometry {
    #[pyo3(signature=(*,label,lattice,shifts,wilson_lines,theory_name=None,torsion=None,rules=None))]
    #[allow(clippy::too_many_arguments)]
    fn build_model(
        &self,
        py: Python<'_>,
        label: &str,
        lattice: &str,
        shifts: &Bound<'_, PyAny>,
        wilson_lines: &Bound<'_, PyAny>,
        theory_name: Option<&str>,
        torsion: Option<&Bound<'_, PyAny>>,
        rules: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<PyModel> {
        self.structured_model(
            py,
            json!({
                "label":label,"lattice":lattice,"shifts":python_value(py, shifts, 0)?,
                "wilson_lines":python_value(py, wilson_lines, 0)?,"theory_name":theory_name,
                "torsion":torsion.map(|value| python_value(py, value, 0)).transpose()?,
                "rules":rules.map(|value| python_value(py, value, 0)).transpose()?
            }),
        )
    }

    #[pyo3(signature=(**embedding))]
    fn request(&self, py: Python<'_>, embedding: Option<&Bound<'_, PyDict>>) -> PyPreparedRequest {
        let result = embedding
            .ok_or_else(|| input_error("expected embedding"))
            .and_then(|value| python_value(py, value.as_any(), 0))
            .and_then(|fields| self.structured_model(py, fields))
            .map(|model| (*model.request).clone())
            .map_err(|error| error.to_string());
        PyPreparedRequest { inner: result }
    }

    #[pyo3(signature=(**embedding))]
    fn validate_embedding(
        &self,
        py: Python<'_>,
        embedding: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        let model = self.structured_model(
            py,
            python_value(
                py,
                embedding
                    .ok_or_else(|| input_error("expected embedding"))?
                    .as_any(),
                0,
            )?,
        )?;
        let validated =
            py.detach(|| validate_embedding_detailed(&self.geometry, &model.request.embedding));
        value_output(
            py,
            &match validated {
                Ok(checks) => json!({"status":"verified","checks":checks.iter().map(|check| json!({
                "indices":check.indices(),"prefactor":orbifolder_data::integer(check.prefactor()),
                "value":orbifolder_data::rational(check.value())})).collect::<Vec<_>>()}),
                Err(failure) => {
                    let mut error = orbifolder_data::scan_error(
                        &execution::ComputeError::Model(failure.error),
                        "embedding",
                    );
                    error["slots"] = json!(failure.slots);
                    if let Some(vector) = failure.scaled_vector {
                        error["scaled_vector"] = orbifolder_data::vector(&vector);
                    }
                    json!({"status":if error["category"] == "physics_invalid" { "failed" } else { "unknown" },"error":error})
                }
            },
        )
    }

    #[pyo3(signature=(*,embedding,slots,alphabet,coordinate_domains=None,basis=None,max_states=100000))]
    #[allow(clippy::too_many_arguments)]
    fn prepare_prefix_mask(
        &self,
        py: Python<'_>,
        embedding: &Bound<'_, PyAny>,
        slots: Vec<usize>,
        alphabet: &Bound<'_, PyAny>,
        coordinate_domains: Option<&Bound<'_, PyAny>>,
        basis: Option<&Bound<'_, PyAny>>,
        max_states: usize,
    ) -> PyResult<PyPrefixMasker> {
        let embedding = self
            .read_embedding(py, embedding)?
            .embedding()
            .map_err(input_error)?;
        let alphabet = exact_list(py, alphabet)?
            .into_iter()
            .map(|entry| entry.0)
            .collect::<Vec<_>>();
        let domains = coordinate_domains
            .map(|value| {
                let rows: Vec<Vec<Exact>> =
                    serde_json::from_str(&json_input(py, value)?).map_err(input_error)?;
                rows.iter()
                    .map(|row| {
                        row.iter()
                            .map(|choice| {
                                alphabet
                                    .iter()
                                    .position(|entry| entry == &choice.0)
                                    .ok_or_else(|| {
                                        input_error("coordinate action outside alphabet")
                                    })
                            })
                            .collect()
                    })
                    .collect()
            })
            .transpose()?;
        let basis: Option<[GaugeVector; 16]> = basis
            .map(|value| {
                let rows: Vec<Vec<Exact>> =
                    serde_json::from_str(&json_input(py, value)?).map_err(input_error)?;
                rows.iter()
                    .map(|row| orbifolder_data::exact_vector(row).map_err(input_error))
                    .collect::<PyResult<Vec<_>>>()?
                    .try_into()
                    .map_err(|_| input_error("expected sixteen basis rows"))
            })
            .transpose()?;
        let spec = PrefixSpec {
            slots,
            alphabet,
            domains,
            basis,
            max_states,
        };
        let inner = py
            .detach(|| PrefixMasker::prepare(&self.geometry, &embedding, spec))
            .map_err(|error| prefix_error(py, error))?;
        Ok(PyPrefixMasker { inner })
    }
}

pub(super) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPrefixMasker>()?;
    module.add_class::<PyPreparedRequest>()?;
    module.add_class::<PyComputeOptions>()?;
    module.add_class::<PyCompactSpectrum>()?;
    module.add_class::<PyFilteredResult>()?;
    Ok(())
}
