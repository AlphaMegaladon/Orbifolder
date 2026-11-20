# orbifolder

An exact rational Rust/Python engine for heterotic orbifold string
compactifications, by **Martin Wimmer**. Calculate SUSY and non-SUSY spectra,
tachyons, anomalies and cubic SUSY selection rules with immutable inputs and
deterministic parallel model scans.

This is a partial library port of the original C++ programs. The supported
core profile does not yet include their random-model search, phenomenology
classifiers, beta functions or full vacuum/mass-matrix workflows. See the
[coverage and package review](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/14_precommit_review.md) and
[future work](https://github.com/AlphaMegaladon/Orbifolder/blob/main/ROADMAP.md).

## Install

Python **3.11+** is required. To install this checkout, first install a current
stable Rust toolchain and a system C linker, then use a compatible Python:

```bash
/usr/bin/python3 -m venv .venv  # Ubuntu 24.04: Python 3.12
source .venv/bin/activate
python -m pip install .
```

See the [Ubuntu development guide](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/11_development_ubuntu.md) for system
packages, Rust installation, Conda coexistence and optional C++ oracle builds.
Virtual environments and build directories copied from another machine must
be recreated. This checkout has release workflows, but their presence does not
establish that the package has been published to PyPI.

Once a release is available on PyPI:

```bash
pip install orbifolder
# or
uv add orbifolder
```

The release workflow is configured to build ABI3 wheels for Linux (x86_64/aarch64), Windows
(x86_64/ARM64) and macOS (Intel/Apple Silicon). All six native builds passed
[release CI](https://github.com/AlphaMegaladon/Orbifolder/actions/runs/43782095269)
on 2026-11-16, including installation tests under CPython 3.11 and 3.14.
Installing a wheel needs no Rust,
C++ compiler or external geometry files. Building from an sdist requires a
current stable Rust toolchain and a platform C linker. C++ and the legacy
libraries are needed only to rebuild the optional reference programs.

Linux wheels target glibc-based distributions through `manylinux`; Alpine Linux
(musl), 32-bit systems and other architectures such as RISC-V, POWER and IBM Z
are outside the current wheel matrix. A source build may be possible where
Python, Rust and the dependencies support the target, but is not a guarantee
of support for untested platforms. See the
[release platform matrix](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/15_pypi_publishing.md#plattformen-und-wheel-abdeckung).

## Quickstart

This standard Z3 embedding runs entirely in memory:

```python
import orbifolder
geometry = orbifolder.Geometry.from_preset("Z3")
shift = "1/3 1/3 -2/3 " + "0 " * 13
zero = "0 " * 16
text = "\n".join(["begin model", "Label:Z3_standard", "Lattice:E8xE8", "Shifts and Wilsonlines:", shift, *([zero] * 7), "end model"])
model, = orbifolder.parse_legacy_models(text, geometry)
evaluation = model.compute()
print(evaluation.summary())
print(orbifolder.Geometry.available_presets())
assert evaluation.tachyon_count == 0
```

Presets: `Z3`, `Z6-II` (alias `Z6_II`) and `Z3xZ3`. These are the **specific
reference torus lattices** used in the seven-model verification, not every
crystallographic realization of the named point group. For non-SUSY embeddings,
use `Geometry.from_preset("Z3", theory_name="non_susy")`; this includes the
order-two supersymmetry-breaking generator and the three-shift convention.
SUSY presets include the discrete selection rules configured in the reference
geometries. Those three geometries configure no R operators; explicit R
profiles are tested separately. A preset does not add unconfigured R rules.

## Batch scans and legacy catalogs

The 0.2 scan API lets multiple candidates share one prepared geometry:

```python
geometry = orbifolder.Geometry.from_preset("Z6-II", theory_name="non_susy")
request = geometry.request(label="candidate", lattice="E8xE8",
                           shifts=shifts, wilson_lines=wilson_lines)
options = orbifolder.ComputeOptions()
row, = orbifolder.Executor(threads=4).compute_compact([request], options=options)
if row["status"] == "ok":
    spectrum = row["compact"]  # Exact grouped field data and checks in one basis.
```

`geometry.build_model(...)` uses the same input to construct a `Model`.
`geometry.validate_embedding(...)` and `geometry.embedding_profile()` run
without a spectrum. Opt-in `stop_on_tachyon` and `stop_on_anomaly` can produce
`filtered` results with unavailable counts. Both options default to false.
See the [0.2 release plan](docs/04_release_0_2_0_plan.md) for the mask and
provenance contracts.

```python
models = orbifolder.load_legacy_file("my_models.txt", geometry="Z3")
summaries = geometry.evaluate_batch_summaries(models, threads=4)
for item in summaries:
    if item["status"] == "ok":
        summary = item["summary"]
        if summary["is_anomaly_free"] and summary["is_tachyon_free"]:
            print(summary["label"], summary["gauge_factors"])
    else:
        print(item["index"], item["error"])
```

`Result.summary()` and `Executor(threads=4).compute_summaries(models)` also expose
the light scan projection. Summaries contain theory, anomaly status, tachyon
status, field counts, ADE gauge factors, U(1) count, charged-singlet count and
exact FI status/coefficient. Batch projection runs **inside Rust workers** and
releases full results per model; it does not copy thousands of fields to Python
or retain all full spectra. Counts follow the explicit CPT/multiplet convention
of the detailed spectrum. Charged singlets mean massless fields trivial under
all non-abelian factors with at least one nonzero U(1) charge; CPT partners count
separately. FI is `not_applicable` in non-SUSY, and omitted anomaly calculations
have `is_anomaly_free=None`, never a fabricated successful check.

For detailed results use `Executor(threads=4).compute_many(models)` or
`geometry.evaluate_legacy_text(text, threads=4)`. Batch items preserve input
order and carry a result or a categorized error. A malformed model block raises
`InputError`; an invalid physical embedding produces `ModelError` (or an error
batch item). `Geometry.evaluate_batch_summaries` requires matching geometries;
`Executor.compute_summaries` also supports batches of mixed geometries.

Both `parse_legacy_models(text, geometry)` and `load_legacy_file(path, geometry)`
accept a `Geometry` or preset name. They load one or more complete `begin model`
blocks. The selected geometry must match every model in a catalog. Embedded
`SpaceGroup:` filenames are descriptive and never opened; CLI/program directives
are never executed. Only `load_legacy_file` opens a file, in Python. The Rust
core has no filesystem I/O. Use UTF-8 by default or pass `encoding=...`.
Legacy generalized-torsion, freely acting Wilson-line and preselected U(1)-basis
sections raise `InputError` rather than silently changing the requested model.

## Exact results and SUSY analyses

Structured inputs use `Geometry.from_data(...)` and `Model.from_data(...)`.
Integers, rational strings, `(num, den)` pairs and `fractions.Fraction` are
accepted; floating-point input is rejected. Integer overflow promotes exact
arithmetic to BigInt. Results are immutable; exported dictionaries are copies.
Installed type stubs and `py.typed` support IDE completion and static checking.

```python
from orbifolder import exact_data

charges = exact_data(evaluation.fields[3].to_data())["u1_charges"]
anomalies = evaluation.anomalies()
pool = [f.id for f in evaluation.fields if f.to_data()["multiplet"] == {"family": "susy", "kind": "LeftChiral"}][:6]
couplings = evaluation.superpotential(field_pool=pool, max_degree=3)
```

Select chiral-superfield IDs from your spectrum for coupling analyses.
`evaluation.couplings(trials)` checks specified monomials; `superpotential(...)` enumerates
the verified degree-1–3 profile. non-SUSY SUSY-only analyses return an explicit
`not_applicable` status. Exact outputs use integer `{"num": n, "den": d}` objects;
`exact_data(..., rational_format="fraction" | "pair" | "dict")` converts them.
Costly native work releases the GIL. Serial execution and Rayon pools of
1/2/4/8 threads are tested for identical canonical results, IDs and errors.

## Scientific background and verification

The engine builds heterotic orbifolds from exact torus Gram matrices, space
groups, gauge shifts and Wilson lines, then solves and projects left/right
states. It computes representations, exact U(1) charges, tachyons, canonical
anomalous U(1) rotations, Green–Schwarz checks and SUSY selection rules.
Its scientific lineage is the original SUSY **orbifolder v1.2.1** and
**nonSUSYorbifolder**; see [attribution](https://github.com/AlphaMegaladon/Orbifolder/blob/main/THIRD_PARTY_NOTICES.md) and
[CITATION.cff](https://github.com/AlphaMegaladon/Orbifolder/blob/main/CITATION.cff). Please cite both predecessor papers and this
software in research.

Seven frozen C++ oracle models certify 4,300 massless and twelve tachyon fields,
including standard Z3/Z6-II, Wilson-line Z3xZ3 and a positive tachyon case.
Additional probes validate exact algebra, selection rules and negative cases.
The migration corrects the following documented consistency/precision issues:

- Float tolerance discrepancies: exact rational phases, charges and lattice
  decisions; exact Gram matrices retain irrational-basis information.
- Uninitialized non-SUSY `CHalfState::Excited`: classify tachyons by actual
  oscillator excitations ([audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/TACHYON_AUDIT.md)).
- Missing non-SUSY Wilson modular checks: reject `3W²=2/3` despite valid lattice
  order ([audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/WILSON_INVARIANCE_AUDIT.md)).
- Fixed tachyon search windows: complete norm bounds remain the default;
  a separate legacy profile supports comparison
  ([audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/TACHYON_WINDOW_AUDIT.md)).
- Verbose-dependent non-SUSY anomaly success and premature SUSY no-U(1)
  return: evaluate every required check
  ([audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/ANOMALY_VALIDATION_AUDIT.md)).
- Incomplete tensor decomposition and unsymmetrized identical superfields:
  exact characters and separate tensor/polynomial invariants
  ([audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/SUSY_SELECTION_AUDIT.md)).

E6 has no symmetric cubic Lie-anomaly tensor; representation cubic invariants
do not change this. Full Spin(32) spectra, nontrivial torsion, higher-order
couplings and numerical amplitudes/VEVs remain outside the seven-case
certification. See the [current repository audit](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/12_repository_audit.md),
[historical final verification](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/09_final_verification_and_release.md),
[packaging verification](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/10_packaging_and_repository_hygiene.md) and the
[benchmark scope](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/benchmark/README.md).

## Development and releases

```bash
/usr/bin/python3 -m venv .venv
source .venv/bin/activate
source "$HOME/.cargo/env"
python -m pip install '.[dev]'
# Linux: format, Clippy, Rust tests, release wheel and isolated Python tests
bash tests/verify_release.sh
python tests/check_typing.py
```

The Linux verification script selects this checkout's `.venv` explicitly, so
Conda's base interpreter cannot accidentally supply Python 3.10. On other
platforms run the individual commands in the [packaging report](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/10_packaging_and_repository_hygiene.md)
with the corresponding environment paths. The [documentation index](https://github.com/AlphaMegaladon/Orbifolder/blob/main/docs/README.md)
distinguishes current setup instructions from historical migration reports.

Oracle archives are optional and excluded from Git and distributions.
[Oracle reproduction](https://github.com/AlphaMegaladon/Orbifolder/blob/main/tests/ground_truth/README.md) explains their sources and
local setup. Regular tests use included input data and frozen JSON only.
The [release workflow](https://github.com/AlphaMegaladon/Orbifolder/blob/main/.github/workflows/release.yml) builds and tests wheels
and sdist, then supports PyPI Trusted Publishing. Configure the repository's
`pypi` environment and a matching PyPI trusted publisher before tagging `v0.1.0`;
see the packaging report for the exact release procedure. A manual workflow run
builds/tests only unless its publish option is explicitly enabled.

Licensed under **GPL-3.0-or-later**, without warranty. See [LICENSE](https://github.com/AlphaMegaladon/Orbifolder/blob/main/LICENSE)
and [THIRD_PARTY_NOTICES.md](https://github.com/AlphaMegaladon/Orbifolder/blob/main/THIRD_PARTY_NOTICES.md).
