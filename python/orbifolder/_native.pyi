# The extension's nominal classes are re-exported by the Python facade.
from . import BatchItem as BatchItem, Executor as Executor, Field as Field, Geometry as Geometry, InputError as InputError, Model as Model, ModelError as ModelError, Result as Result, ExecutionError as ExecutionError, __version__ as __version__
from . import _Theory, _Rule
from . import CompactSpectrum as CompactSpectrum, ComputeOptions as ComputeOptions, FilteredResult as FilteredResult, ModelRequest as ModelRequest, PrefixMasker as PrefixMasker

__all__ = ["BatchItem", "CompactSpectrum", "ComputeOptions", "Executor", "Field", "FilteredResult", "Geometry", "InputError", "Model", "ModelError", "ModelRequest", "PrefixMasker", "Result", "ExecutionError", "parse_legacy_models", "__version__"]

def parse_legacy_models(text: str, geometry: Geometry, *, theory_name: _Theory | None = None, rules: list[_Rule] | None = None) -> list[Model]: ...
