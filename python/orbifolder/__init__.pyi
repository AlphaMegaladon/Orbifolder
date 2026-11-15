from collections.abc import Sequence as _Sequence
from fractions import Fraction as Fraction
from os import PathLike as _PathLike
from typing import Literal as _Literal, NotRequired as _NotRequired, Self as _Self, TypedDict as _TypedDict, final as _final

__all__ = ["BatchItem", "Executor", "Field", "Geometry", "InputError", "Model", "ModelError", "Result", "ExecutionError", "exact_data", "parse_legacy_models", "load_legacy_file", "__version__"]
__version__: str

_Theory = _Literal["susy", "non_susy"]
_JSON = None | bool | int | str | list["_JSON"] | dict[str, "_JSON"]
_Converted = None | bool | int | str | Fraction | tuple[int, int] | list["_Converted"] | dict[str, "_Converted"]

class _Rational(_TypedDict):
    num: int
    den: int

_Exact = int | str | Fraction | tuple[int, int] | list[int] | _Rational

class _Element(_TypedDict):
    factors: list[int]
    translation: list[_Exact]

class _Generator(_TypedDict):
    order: int
    rotation: list[list[_Exact]]
    translation: list[_Exact]
    twist: list[_Exact]

class _GeometryData(_TypedDict):
    gram: list[list[_Exact]]
    generators: list[_Generator]
    representatives: _NotRequired[list[list[_Element]] | None]
    centralizer_presentations: _NotRequired[list[list[_Element]] | None]

class _Rule(_TypedDict):
    kind: _Literal["r", "non_r"]
    order: int
    target: _Exact
    operator: list[_Exact]

class _ModelData(_TypedDict):
    schema_version: int
    theory: _Theory
    label: str
    lattice: _Literal["E8xE8", "Spin32", "D16Plus", "D8xD8"]
    geometry: _GeometryData
    shifts: list[list[_Exact]]
    wilson_lines: list[list[_Exact]]
    torsion: _NotRequired[list[list[_Exact]] | None]
    rules: _NotRequired[list[_Rule]]

class _Error(_TypedDict):
    status: _Literal["error"]
    category: _Literal["model", "physics", "anomalies", "susy"]
    message: str

class _Fi(_TypedDict):
    status: _Literal["computed", "not_applicable", "unsupported", "not_requested"]
    value: _NotRequired[_Rational]
    reason: _NotRequired[str]
    has_anomalous_u1: bool | None

class _Summary(_TypedDict):
    label: str
    theory: _Theory
    anomaly_status: _Literal["computed", "not_requested"]
    is_anomaly_free: bool | None
    is_tachyon_free: bool
    massless_count: int
    tachyon_count: int
    gauge_factors: list[str]
    u1_count: int
    charged_singlet_count: int
    fi: _Fi

class _SummaryOK(_TypedDict):
    index: int
    status: _Literal["ok"]
    summary: _Summary
    error: None

class _SummaryError(_TypedDict):
    index: int
    status: _Literal["error"]
    summary: None
    error: _Error

class _Representation(_TypedDict):
    algebra: str
    dynkin: list[int]
    dimension: int

class _Excitation(_TypedDict):
    oscillators: list[int]
    energy: _Rational
    target: _Rational
    excited: bool

class _SusyMultiplet(_TypedDict):
    family: _Literal["susy"]
    kind: _Literal["LeftChiral", "RightChiral", "Vector", "ConjugateVector", "HalfHyper", "Gravity", "ConjugateGravity", "LeftModulus", "RightModulus"]

class _Particle(_TypedDict):
    family: _Literal["particle"]
    kind: _Literal["Gauge", "ConjugateGauge", "Scalar", "ConjugateScalar", "LeftFermion", "RightFermion", "Gravity", "ConjugateGravity", "Modulus", "ConjugateModulus"]

class _Tachyon(_TypedDict):
    family: _Literal["tachyon"]
    excited: bool
    conjugate: bool

class _FieldData(_TypedDict):
    id: int
    source: list[int]
    element: _Element
    weight_indices: list[int]
    highest_weight: list[_Rational]
    representations: list[_Representation]
    u1_charges: list[_Rational]
    multiplet: _SusyMultiplet | _Particle | _Tachyon
    right_weights: list[list[_Rational]]
    origin_classes: list[str]
    spacetime_classes: list[str]
    scalar_charge: list[_Rational] | None
    left_excitation: _Excitation
    right_excitation: _Excitation
    gamma: list[_Rational]
    oscillator_contribution: list[_Rational]
    mass: _Rational

class _Unavailable(_TypedDict):
    status: _Literal["not_applicable", "unsupported"]
    reason: str

class _NotRequested(_TypedDict):
    status: _Literal["not_requested"]

class _ComputedBool(_TypedDict):
    status: _Literal["computed"]
    value: bool

class _ComputedRational(_TypedDict):
    status: _Literal["computed"]
    value: _Rational

class _Traces(_TypedDict):
    gravitational: list[_Rational]
    mixed: list[list[_Rational]]
    gauge_cubic: list[_Rational]
    abelian_cubic: list[list[list[_Rational]]]
    su2_doublets: list[int | None]
    su2_parity: list[int | None]

class _Rotation(_TypedDict):
    generators: list[list[_Rational]]
    transformation: list[list[_Rational]]
    gram: list[list[_Rational]]
    trace_vector: list[_Rational]
    anomalous_index: int | None

class _FiDetails(_TypedDict):
    legacy_d0: _Rational
    norm_squared: _Rational
    trace_over24: _Rational
    susy_coefficient: _ComputedRational | _Unavailable

class _CheckIdentifier(_TypedDict):
    kind: _Literal["gauge_cubic", "witten", "mixed_universal", "abelian_universal", "traceless_gravity", "traceless_mixed", "independent_abelian_directions", "fi_normalization"]
    factor: _NotRequired[int]
    u1: _NotRequired[int]
    indices: _NotRequired[list[int]]

class _AnomalyCheck(_TypedDict):
    check: _CheckIdentifier
    result: _ComputedBool | _Unavailable
    actual: _Rational | None
    expected: _Rational | None

class _SpectrumData(_TypedDict):
    theory: _Literal["Susy", "NonSusy"]
    gauge_group: dict[str, _JSON]
    fields: list[_FieldData]
    tachyons: list[_FieldData]

class _Anomalies(_TypedDict):
    status: _Literal["computed"]
    is_anomaly_free: bool
    raw: _Traces
    canonical: _Traces
    rotation: _Rotation
    levels: list[_Rational]
    fi: _FiDetails
    checks: list[_AnomalyCheck]
    spectrum: _SpectrumData

class _DiscreteCheck(_TypedDict):
    kind: _Literal["r", "non_r"]
    order: int
    target: _Rational
    operator: list[_Rational]
    charge_sum: _Rational
    passed: bool

class _Coupling(_TypedDict):
    fields: list[int]
    degree: int
    charge_sum: list[_Rational]
    tensor_singlets: int
    polynomial_singlets: int
    gauge_invariant: bool
    native_gauge_invariant: bool
    native_pre: bool
    native_allowed: bool
    polynomial_allowed: bool
    point_group: bool
    space_group: bool
    native_fixed_torus: bool
    gamma_sums: list[_Rational]
    gamma: _ComputedBool | _Unavailable
    discrete: list[_DiscreteCheck]

class _CouplingTrials(_TypedDict):
    status: _Literal["computed"]
    trials: list[_Coupling]

class _Superpotential(_TypedDict):
    status: _Literal["computed"]
    max_degree: int
    field_pool: list[int]
    candidates: list[_Coupling]
    allowed: list[int]

class InputError(ValueError): ...
class ModelError(ValueError):
    category: _Literal["model", "physics", "anomalies", "susy"]
class ExecutionError(RuntimeError): ...

@_final
class Geometry:
    @staticmethod
    def available_presets() -> list[str]: ...
    @staticmethod
    def from_preset(name: str, *, theory_name: _Theory = "susy") -> Geometry: ...
    @staticmethod
    def from_data(data: _GeometryData) -> Geometry: ...
    def to_data(self) -> _GeometryData: ...
    def evaluate_legacy_text(self, text: str, *, theory_name: _Theory | None = None, rules: list[_Rule] | None = None, threads: int | None = None) -> list[BatchItem]: ...
    def evaluate_batch_summaries(self, models: _Sequence[Model], *, threads: int | None = None, anomalies: bool = True) -> list[_SummaryOK | _SummaryError]: ...

@_final
class Model:
    @staticmethod
    def from_data(data: _ModelData) -> Model: ...
    @staticmethod
    def from_text(text: str, geometry: Geometry, theory_name: _Theory = "susy", rules: list[_Rule] | None = None) -> Model: ...
    def to_data(self) -> _ModelData: ...
    @property
    def label(self) -> str: ...
    def compute(self, *, anomalies: bool = True, search_nodes: int = 10_000_000, solutions: int = 1_000_000, tachyon_search: _Literal["complete", "legacy"] = "complete") -> Result: ...

@_final
class Result:
    def summary(self) -> _Summary: ...
    def to_json(self) -> str: ...
    def to_data(self) -> dict[str, _JSON]: ...
    @property
    def massless_count(self) -> int: ...
    @property
    def tachyon_count(self) -> int: ...
    @property
    def fields(self) -> list[Field]: ...
    @property
    def tachyons(self) -> list[Field]: ...
    def anomalies(self) -> _Anomalies | _NotRequested: ...
    def couplings(self, trials: _Sequence[_Sequence[int]], rules: list[_Rule] | None = None) -> _CouplingTrials | _Unavailable: ...
    def superpotential(self, field_pool: _Sequence[int], max_degree: int = 3) -> _Superpotential | _Unavailable: ...

@_final
class Field:
    @property
    def id(self) -> int: ...
    @property
    def is_tachyon(self) -> bool: ...
    def to_data(self) -> _FieldData: ...

@_final
class BatchItem:
    @property
    def index(self) -> int: ...
    @property
    def ok(self) -> bool: ...
    @property
    def value(self) -> Result | None: ...
    @property
    def error(self) -> _Error | None: ...
    def to_json(self) -> str: ...
    def to_data(self) -> dict[str, _JSON]: ...

@_final
class Executor:
    def __new__(cls, threads: int | None = None) -> _Self: ...
    @property
    def threads(self) -> int: ...
    def compute_many(self, models: _Sequence[Model]) -> list[BatchItem]: ...
    def compute_summaries(self, models: _Sequence[Model]) -> list[_SummaryOK | _SummaryError]: ...

def parse_legacy_models(text: str, geometry: Geometry | str = "Z3", *, theory_name: _Theory | None = None, rules: list[_Rule] | None = None) -> list[Model]: ...
def load_legacy_file(path: str | _PathLike[str], geometry: Geometry | str = "Z3", *, theory_name: _Theory | None = None, rules: list[_Rule] | None = None, encoding: str = "utf-8") -> list[Model]: ...
def exact_data(data: _JSON | _FieldData | _ModelData | _GeometryData, *, rational_format: _Literal["fraction", "pair", "dict"] = "fraction") -> _Converted: ...
