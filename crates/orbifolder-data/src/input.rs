use num_bigint::BigInt;
use orbifolder_core::{
    execution::{ComputeOptions, ModelRequest},
    geometry::{
        DiscreteTorsion, ElementSpec, EmbeddingSpec, GeneratorSpec, Geometry, GeometrySpec, Limits,
        TheorySpec,
    },
    math_core::{GaugeLattice, Matrix, Rational, Vector},
    susy::{DiscreteRule, RuleCharge, SelectionRules},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataError(pub String);
impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for DataError {}
fn invalid(e: impl std::fmt::Display) -> DataError {
    DataError(e.to_string())
}
/// Numbers never pass through f64. JSON integer tokens remain arbitrary precision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exact(pub Rational);
impl Serialize for Exact {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        crate::rational(&self.0).serialize(s)
    }
}
fn integer(v: &Value) -> Result<BigInt, DataError> {
    let text = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return Err(invalid("Expected an integer, not a float or bool")),
    };
    text.parse().map_err(invalid)
}
impl<'de> Deserialize<'de> for Exact {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let r = (|| match &v {
            Value::String(s) => s.parse().map_err(invalid),
            Value::Number(_) => Ok(Rational::from(integer(&v)?)),
            Value::Array(a) if a.len() == 2 => {
                Rational::from_bigints(integer(&a[0])?, integer(&a[1])?).map_err(invalid)
            }
            Value::Object(o) if o.len() == 2 && o.contains_key("num") && o.contains_key("den") => {
                Rational::from_bigints(integer(&o["num"])?, integer(&o["den"])?).map_err(invalid)
            }
            _ => Err(invalid(
                "Expected an exact integer, rational string or numerator/denominator pair",
            )),
        })()
        .map_err(serde::de::Error::custom)?;
        Ok(Self(r))
    }
}
pub fn exact_vector<const N: usize>(v: &[Exact]) -> Result<Vector<N>, DataError> {
    Vector::from_slice(&v.iter().map(|x| x.0.clone()).collect::<Vec<_>>()).map_err(invalid)
}
use exact_vector as vector;
fn matrix(v: &[Vec<Exact>]) -> Result<Matrix, DataError> {
    Matrix::from_rows(
        v.iter()
            .map(|r| r.iter().map(|x| x.0.clone()).collect())
            .collect(),
    )
    .map_err(invalid)
}
fn vec_data<const N: usize>(v: &Vector<N>) -> Vec<Exact> {
    v.coordinates().iter().cloned().map(Exact).collect()
}
fn mat_data(v: &Matrix) -> Vec<Vec<Exact>> {
    (0..v.rows())
        .map(|i| {
            (0..v.cols())
                .map(|j| Exact(v.get(i, j).unwrap().clone()))
                .collect()
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementData {
    pub factors: [u32; 3],
    pub translation: Vec<Exact>,
}
impl ElementData {
    fn spec(&self) -> Result<ElementSpec, DataError> {
        Ok(ElementSpec {
            factors: self.factors,
            translation: vector(&self.translation)?,
        })
    }
    fn from_spec(e: &ElementSpec) -> Self {
        Self {
            factors: e.factors,
            translation: vec_data(&e.translation),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratorData {
    pub order: u32,
    pub rotation: Vec<Vec<Exact>>,
    pub translation: Vec<Exact>,
    pub twist: Vec<Exact>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryData {
    pub gram: Vec<Vec<Exact>>,
    pub generators: [GeneratorData; 3],
    #[serde(default)]
    pub representatives: Option<Vec<Vec<ElementData>>>,
    #[serde(default)]
    pub centralizer_presentations: Option<Vec<Vec<ElementData>>>,
}
impl GeometryData {
    pub fn spec(&self) -> Result<GeometrySpec, DataError> {
        let g = self
            .generators
            .iter()
            .map(|g| {
                Ok(GeneratorSpec {
                    order: g.order,
                    rotation: matrix(&g.rotation)?,
                    translation: vector(&g.translation)?,
                    twist: vector(&g.twist)?,
                })
            })
            .collect::<Result<Vec<_>, DataError>>()?;
        let elements = |v: &Option<Vec<Vec<ElementData>>>| {
            v.as_ref()
                .map(|v| {
                    v.iter()
                        .map(|s| s.iter().map(ElementData::spec).collect())
                        .collect()
                })
                .transpose()
        };
        Ok(GeometrySpec {
            gram: matrix(&self.gram)?,
            generators: g
                .try_into()
                .map_err(|_| invalid("Three generators required"))?,
            representatives: elements(&self.representatives)?,
            centralizer_presentations: elements(&self.centralizer_presentations)?,
            limits: Limits::default(),
        })
    }
    pub fn build(&self) -> Result<Arc<Geometry>, DataError> {
        Ok(Arc::new(Geometry::build(self.spec()?).map_err(invalid)?))
    }
    pub fn from_spec(s: &GeometrySpec) -> Self {
        let elements = |v: &Option<Vec<Vec<ElementSpec>>>| {
            v.as_ref().map(|v| {
                v.iter()
                    .map(|s| s.iter().map(ElementData::from_spec).collect())
                    .collect()
            })
        };
        Self {
            gram: mat_data(&s.gram),
            generators: std::array::from_fn(|i| {
                let g = &s.generators[i];
                GeneratorData {
                    order: g.order,
                    rotation: mat_data(&g.rotation),
                    translation: vec_data(&g.translation),
                    twist: vec_data(&g.twist),
                }
            }),
            representatives: elements(&s.representatives),
            centralizer_presentations: elements(&s.centralizer_presentations),
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TheoryData {
    Susy,
    NonSusy,
}
impl From<TheoryData> for TheorySpec {
    fn from(v: TheoryData) -> Self {
        match v {
            TheoryData::Susy => Self::Susy,
            TheoryData::NonSusy => Self::NonSusy,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelData {
    pub schema_version: u32,
    pub theory: TheoryData,
    pub label: String,
    pub lattice: String,
    pub geometry: GeometryData,
    pub shifts: [Vec<Exact>; 3],
    pub wilson_lines: [Vec<Exact>; 6],
    #[serde(default)]
    pub torsion: Option<Vec<Vec<Exact>>>,
    #[serde(default)]
    pub rules: Vec<RuleData>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedEmbeddingData {
    schema_version: u32,
    theory: TheoryData,
    label: String,
    lattice: String,
    shifts: [Vec<Exact>; 3],
    wilson_lines: [Vec<Exact>; 6],
    #[serde(default)]
    torsion: Option<Vec<Vec<Exact>>>,
    rules: Vec<RuleData>,
}

impl ModelData {
    pub fn from_prepared_data(value: Value, geometry: &GeometryData) -> Result<Self, DataError> {
        let data: PreparedEmbeddingData = serde_json::from_value(value).map_err(invalid)?;
        Ok(Self {
            schema_version: data.schema_version,
            theory: data.theory,
            label: data.label,
            lattice: data.lattice,
            geometry: geometry.clone(),
            shifts: data.shifts,
            wilson_lines: data.wilson_lines,
            torsion: data.torsion,
            rules: data.rules,
        })
    }
    /// Parse embeddings from all complete model blocks; CLI commands outside
    /// the blocks are data only and are never interpreted or executed.
    pub fn parse_legacy_models(
        text: &str,
        geometry: GeometryData,
        theory: TheoryData,
    ) -> Result<Vec<Self>, DataError> {
        let mut blocks = Vec::new();
        let mut block: Option<String> = None;
        for line in text.lines() {
            let content = line.trim();
            match content {
                "begin model" => {
                    if block.is_some() {
                        return Err(invalid("Nested model blocks"));
                    }
                    block = Some(String::from("begin model\n"));
                }
                "end model" => {
                    let mut b = block
                        .take()
                        .ok_or_else(|| invalid("Unexpected end model"))?;
                    b.push_str("end model\n");
                    blocks.push(Self::from_text(&b, geometry.clone(), theory)?);
                }
                _ => {
                    if let Some(b) = block.as_mut() {
                        b.push_str(line);
                        b.push('\n');
                    }
                }
            }
        }
        if block.is_some() {
            return Err(invalid("Unterminated model block"));
        }
        if blocks.is_empty() {
            return Err(invalid("No model blocks found"));
        }
        Ok(blocks)
    }
    pub fn from_json(text: &str) -> Result<Self, DataError> {
        serde_json::from_str(text).map_err(invalid)
    }
    pub fn embedding(&self) -> Result<EmbeddingSpec, DataError> {
        if self.schema_version != 1 {
            return Err(invalid("Unsupported schema_version"));
        }
        let lattice = match self.lattice.as_str() {
            "E8xE8" => GaugeLattice::E8xE8,
            "Spin32" | "D16Plus" => GaugeLattice::D16Plus,
            "D8xD8" => GaugeLattice::D8xD8,
            _ => return Err(invalid("Unknown gauge lattice")),
        };
        Ok(EmbeddingSpec {
            label: self.label.clone(),
            lattice,
            shifts: std::array::from_fn::<_, 3, _>(|i| vector(&self.shifts[i]))
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|_| invalid("Three shifts required"))?,
            wilson_lines: std::array::from_fn::<_, 6, _>(|i| vector(&self.wilson_lines[i]))
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|_| invalid("Six Wilson lines required"))?,
            torsion: match &self.torsion {
                Some(t) => DiscreteTorsion::new(true, matrix(t)?).map_err(invalid)?,
                None => DiscreteTorsion::default(),
            },
        })
    }
    pub fn request(&self, shared: Option<Arc<Geometry>>) -> Result<ModelRequest, DataError> {
        let embedding = self.embedding()?;
        if let Some(g) = &shared {
            let spec = self.geometry.spec()?;
            let space = g.space_group();
            let generators_match = spec
                .generators
                .iter()
                .zip(space.generators())
                .all(|(a, b)| {
                    a.order == b.order
                        && a.rotation == b.rotation
                        && a.translation == b.translation
                        && a.twist == b.twist
                });
            let representatives = g
                .sectors()
                .iter()
                .map(|s| {
                    s.fixed_branes()
                        .iter()
                        .map(|b| b.element().spec())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let centralizers = g
                .sectors()
                .iter()
                .flat_map(|s| s.fixed_branes())
                .map(|b| {
                    b.centralizer_presentation()
                        .iter()
                        .map(|e| e.spec())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if &spec.gram != space.gram()
                || !generators_match
                || spec
                    .representatives
                    .as_ref()
                    .is_some_and(|r| r != &representatives)
                || spec
                    .centralizer_presentations
                    .as_ref()
                    .is_some_and(|r| r != &centralizers)
            {
                return Err(invalid(
                    "Shared geometry does not match the supplied input and ID conventions",
                ));
            }
        }
        self.request_prepared(
            match shared {
                Some(g) => g,
                None => self.geometry.build()?,
            },
            embedding,
        )
    }
    pub fn request_prepared(
        &self,
        geometry: Arc<Geometry>,
        embedding: EmbeddingSpec,
    ) -> Result<ModelRequest, DataError> {
        Ok(ModelRequest {
            geometry,
            embedding,
            theory: self.theory.into(),
            options: ComputeOptions::default(),
            coupling_rules: selection_rules(&self.rules)?,
        })
    }
    /// Legacy model text supplies embeddings only; geometry is explicitly supplied
    /// in memory. SpaceGroup filenames and CLI/program blocks are never executed.
    pub fn from_text(
        text: &str,
        geometry: GeometryData,
        theory: TheoryData,
    ) -> Result<Self, DataError> {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with("//"))
            .collect::<Vec<_>>();
        if lines.iter().filter(|&&s| s == "begin model").count() != 1 {
            return Err(invalid("Exactly one model block required"));
        }
        let begin = lines.iter().position(|&s| s == "begin model").unwrap();
        let end = lines
            .iter()
            .position(|&s| s == "end model")
            .filter(|&end| end > begin)
            .ok_or_else(|| invalid("Missing end model"))?;
        let lines = &lines[begin..=end];
        for marker in [
            "discrete torsion",
            "use freely acting WL:",
            "begin U1 generators",
        ] {
            if lines.iter().any(|line| line.starts_with(marker)) {
                return Err(invalid(format!("Unsupported legacy section: {marker}")));
            }
        }
        let property = |prefix: &str| {
            let matches = lines
                .iter()
                .filter_map(|s| s.strip_prefix(prefix))
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(invalid(format!("Expected exactly one {prefix}")));
            }
            Ok(matches[0].trim().to_string())
        };
        let start = lines
            .iter()
            .position(|&s| s == "Shifts and Wilsonlines:")
            .ok_or_else(|| invalid("Missing Shifts and Wilsonlines"))?
            + 1;
        let end = lines[start..]
            .iter()
            .position(|&s| s == "end model")
            .ok_or_else(|| invalid("Missing end model"))?
            + start;
        let rows = lines[start..end]
            .iter()
            .filter(|s| !s.starts_with("Program filename:"))
            .map(|s| {
                s.split_whitespace()
                    .map(|x| x.parse::<Rational>().map(Exact).map_err(invalid))
                    .collect()
            })
            .collect::<Result<Vec<Vec<Exact>>, DataError>>()?;
        let n = match theory {
            TheoryData::Susy => 2,
            TheoryData::NonSusy => 3,
        };
        if rows.len() != n + 6 {
            return Err(invalid("Incorrect shift/Wilson row count"));
        }
        Ok(Self {
            schema_version: 1,
            theory,
            label: property("Label:")?,
            lattice: property("Lattice:")?,
            geometry,
            shifts: std::array::from_fn::<_, 3, _>(|i| {
                if i < n {
                    rows[i].clone()
                } else {
                    vec![Exact(Rational::ZERO); 16]
                }
            }),
            wilson_lines: std::array::from_fn(|i| rows[n + i].clone()),
            torsion: None,
            rules: Vec::new(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleData {
    pub kind: String,
    pub order: u32,
    pub target: Exact,
    pub operator: Vec<Exact>,
}
pub fn selection_rules(rules: &[RuleData]) -> Result<SelectionRules, DataError> {
    Ok(SelectionRules {
        discrete: rules
            .iter()
            .map(|r| {
                let op = r.operator.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
                let charge = match r.kind.as_str() {
                    "r" => RuleCharge::R(Box::new(
                        op.try_into()
                            .map_err(|_| invalid("R operator needs four coordinates"))?,
                    )),
                    "non_r" => RuleCharge::NonR(Box::new(
                        op.try_into()
                            .map_err(|_| invalid("Non-R operator needs nine coordinates"))?,
                    )),
                    _ => return Err(invalid("Unknown rule kind")),
                };
                DiscreteRule::new(r.order, r.target.0.clone(), charge).map_err(invalid)
            })
            .collect::<Result<Vec<_>, _>>()?,
        ..SelectionRules::default()
    })
}
