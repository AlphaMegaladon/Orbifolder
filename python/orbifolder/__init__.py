"""Exact, immutable model calculations supplied by the Rust extension."""
from fractions import Fraction
from pathlib import Path as _Path
from . import _native
from ._native import BatchItem, CompactSpectrum, ComputeOptions, Executor, Field, FilteredResult, Geometry, InputError, Model, ModelError, ModelRequest, PrefixMasker, Result, ExecutionError, __version__

__all__ = ["BatchItem", "CompactSpectrum", "ComputeOptions", "Executor", "Field", "FilteredResult", "Geometry", "InputError", "Model", "ModelError", "ModelRequest", "PrefixMasker", "Result", "ExecutionError", "exact_data", "parse_legacy_models", "load_legacy_file", "__version__"]


def parse_legacy_models(text, geometry="Z3", *, theory_name=None, rules=None):
    """Parse all model blocks using explicit in-memory geometry or a preset.

    Geometry filenames in the text are descriptive and are never opened.
    Use a matching geometry for every block; mixed-geometry catalogs must be
    split by the caller. CLI/program commands are never executed.
    """
    if isinstance(geometry, str):
        geometry = Geometry.from_preset(geometry, theory_name=theory_name or "susy")
    if not isinstance(geometry, Geometry):
        raise TypeError("geometry must be a Geometry or preset name")
    return _native.parse_legacy_models(text, geometry, theory_name=theory_name, rules=rules)


def load_legacy_file(path, geometry="Z3", *, theory_name=None, rules=None, encoding="utf-8"):
    """Read a legacy catalog at the Python boundary, then parse its embeddings.

    File errors remain OSError/UnicodeError; model syntax errors use InputError.
    The Rust core performs no filesystem I/O.
    """
    return parse_legacy_models(_Path(path).read_text(encoding=encoding), geometry,
                               theory_name=theory_name, rules=rules)


def exact_data(data, *, rational_format="fraction"):
    """Convert copied exact result data to Fraction, (num, den), or dict form."""
    if rational_format not in ("fraction", "pair", "dict"):
        raise ValueError("rational_format must be fraction, pair or dict")
    if isinstance(data, dict):
        if set(data) == {"num", "den"}:
            n, d = data["num"], data["den"]
            return Fraction(n, d) if rational_format == "fraction" else ((n, d) if rational_format == "pair" else data.copy())
        return {key: exact_data(value, rational_format=rational_format) for key, value in data.items()}
    if isinstance(data, list):
        return [exact_data(value, rational_format=rational_format) for value in data]
    return data
