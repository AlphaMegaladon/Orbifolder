"""Run the existing C++ oracles twice per case and freeze validated JSON."""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import subprocess
from pathlib import Path
from typing import Any

from compare import SCHEMA, difference, load, require, tachyon_audit, validate

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent.parent
CASES = (
    ("susy_z3", "susy"),
    ("susy_z6_ii", "susy"),
    ("non_susy_z3", "non_susy"),
    ("non_susy_z6_ii", "non_susy"),
    ("susy_z3xz3_wl", "susy"),
    ("non_susy_z3xz3_wl", "non_susy"),
    ("non_susy_tachyonic", "non_susy"),
)


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def linux_path(path: Path) -> str:
    value = path.resolve().as_posix()
    if os.name == "nt":
        parts = value.split("/")
        require(parts[:3] == ["", "", "wsl.localhost"], f"Expected a WSL workspace: {value}")
        return "/" + "/".join(parts[4:])
    return value


def run_linux(args: list[str], cwd: Path, timeout: int, stdin: bytes | None = None) -> subprocess.CompletedProcess[bytes]:
    command = ["env", "LC_ALL=C", "TZ=UTC", *args]
    if os.name == "nt":
        distro = cwd.resolve().as_posix().split("/")[3]
        command = ["wsl.exe", "-d", distro, "--cd", linux_path(cwd), *command]
    return subprocess.run(command, cwd=ROOT if os.name == "nt" else cwd,
                          input=stdin, capture_output=True, timeout=timeout + 10, check=False)


def prepare_cases() -> None:
    """Materialize small, reviewable exact input files; never replace edits."""
    directory = ROOT / "cases"
    directory.mkdir(exist_ok=True)
    zero = " ".join(["0"] * 16)
    for name, twist in (("z3", "1/3 1/3 -2/3"), ("z6_ii", "1/6 1/3 -1/2")):
        geometry = "Z3" if name == "z3" else "Z6-II"
        shift = twist + " " + " ".join(["0"] * 13)
        text = (f"begin model\nLabel:SUSY_{geometry}_standard\n"
                f"SpaceGroup:Geometry/Geometry_{geometry}_1_1.txt\nLattice:E8xE8\n"
                "Shifts and Wilsonlines:\n" + "\n".join([shift] + [zero] * 7) + "\nend model\n")
        target = directory / f"susy_{name}.txt"
        if not target.exists():
            target.write_text(text, encoding="utf-8", newline="\n")
        source = REPO / f"legacy_cpp/non_susy_orbifolder/Models/ZN_models/model{geometry}_1_1.txt"
        target = directory / f"non_susy_{name}.txt"
        if not target.exists():
            target.write_text(source.read_text(encoding="utf-8"), encoding="utf-8", newline="\n")
    source = REPO / "legacy_cpp/susy_orbifolder/Z3xZ3_1_1-1WL-77PatricksModels.txt"
    target = directory / "susy_z3xz3_wl.txt"
    if not target.exists():
        target.write_text(source.read_text(encoding="utf-8").split("end model", 1)[0] + "end model\n",
                          encoding="utf-8", newline="\n")
    target = directory / "non_susy_z3xz3_wl.txt"
    if not target.exists():
        source = REPO / "legacy_cpp/non_susy_orbifolder/Models/ZNxZM_models/modelZ3xZ3_1_1.txt"
        lines = source.read_text(encoding="utf-8").splitlines()
        lines[1] = "Label:Z3xZ3_1_1_WL"
        first_wilson = lines.index("Shifts and Wilsonlines:") + 4
        # A pair of identical order-three Wilson lines in the second E8.
        wilson = " ".join(["0"] * 8 + ["2/3", "-1/3", "-1/3"] + ["0"] * 5)
        lines[first_wilson:first_wilson + 2] = [wilson, wilson]
        target.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def check_sources(work: Path, variant: str) -> None:
    original = REPO / f"legacy_cpp/{variant}_orbifolder"
    for line in (work / "original.sha256").read_text(encoding="utf-8").splitlines():
        expected, relative = line.split("  ", 1)
        require(sha(original / relative) == expected, f"Archived file changed: {relative}")
        # Runtime geometry and gauge-invariance tables must also match the archive.
        if relative.startswith(("./Geometry/", "./GaugeInvariance/")):
            require(sha(work / "source" / relative) == expected, f"Runtime data changed: {relative}")
    lines = (work / "cpp-built.sha256").read_text(encoding="utf-8").splitlines()
    for line in lines:
        expected, relative = line.split("  ", 1)
        require(sha(original / relative) == expected, f"Archived source changed: {relative}")
        require(sha(work / "source" / relative) == expected, f"Build-copy source changed: {relative}")
    for saved, actual in (("oracle-source.sha256", ROOT / "probes/oracle.cpp"),
                          ("oracle-build-script.sha256", ROOT / "build_probes.sh"),
                          ("oracle.sha256", work / "oracle")):
        require((work / saved).read_text().split()[0] == sha(actual), f"Stale probe: {saved}; rebuild probes")


def linked_inputs(work: Path) -> dict[str, str]:
    prefix = linux_path(work) + "/"
    result: dict[str, str] = {}
    for line in (work / "oracle-inputs.sha256").read_text().splitlines():
        expected, filename = line.split("  ", 1)
        require(filename.startswith(prefix), "Linked input outside build directory")
        relative = filename[len(prefix):]
        require(sha(work / relative) == expected, f"Linked object/library changed: {relative}")
        result[relative] = expected
    objects = {p.relative_to(work).as_posix() for p in (work / "build/src/orbifolder").glob("*.o") if p.name != "main.o"}
    objects |= {f"build/src/{name}/lib{name}.a" for name in ("io", "linalg", "groupTheory")}
    require(objects == set(result), "Linked object set is incomplete or includes main.o")
    return result


def normalize_metric(data: dict[str, Any], geometry_file: str) -> None:
    """Curated symbolic interpretation, separate from native C++ observations."""
    require(geometry_file in {f"Geometry/Geometry_{name}_1_1.txt" for name in ("Z3", "Z6-II", "Z3xZ3")},
            "Geometry has no reviewed exact metric")
    z6 = "Z6-II" in geometry_file
    gram = [[0] * 6 for _ in range(6)]
    for i in range(6):
        gram[i][i] = 3 if z6 and i >= 4 else 4
    for i in range(0, 4 if z6 else 6, 2):
        gram[i][i + 1] = gram[i + 1][i] = -2
    data["geometry"]["exact_metric"] = {
        "origin": "curated symbolic interpretation; not a native rational C++ metric",
        "basis_interpretation": "two_A2_plus_sqrt3_axes" if z6 else "three_A2",
        "cartesian_component_abs_error_bound": 4e-11,
        "gram_matrix": [[{"num": n, "den": 1} for n in row] for row in gram],
    }


def generate_case(case_id: str, variant: str, build_root: Path, output: Path, timeout: int, overwrite: bool) -> None:
    work = build_root / variant
    require(work.is_dir(), f"Missing legacy build: {work}")
    check_sources(work, variant)
    objects = linked_inputs(work)
    model = ROOT / "cases" / f"{case_id}.txt"
    text = model.read_text(encoding="utf-8")
    geometry = next(line.split(":", 1)[1] for line in text.splitlines() if line.startswith("SpaceGroup:"))
    label = next(line.split(":", 1)[1] for line in text.splitlines() if line.startswith("Label:"))
    actual_geometry = work / "source" / geometry
    require(sha(actual_geometry) == sha(REPO / f"legacy_cpp/{variant}_orbifolder" / geometry), "Geometry copy differs from archive")
    runs = work / "runs" / case_id
    runs.mkdir(parents=True, exist_ok=True)
    binary_name = "orbifolder" if variant == "susy" else "nonSUSYorbifolder"
    binary = work / "build/src/orbifolder" / binary_name
    require((work / "binary.sha256").read_text().split()[0] == sha(binary), "Original CLI binary changed")
    cli = run_linux(["timeout", "--kill-after=5s", str(timeout), linux_path(binary), linux_path(model)],
                    work / "source", timeout, b"exit orbifolder\n")
    (runs / "cli.stdout").write_bytes(cli.stdout)
    (runs / "cli.stderr").write_bytes(cli.stderr)
    require(cli.returncode == 0 and f'Orbifold "{label}" loaded.'.encode() in cli.stdout,
            f"Original CLI failed for {case_id}: {cli.stderr.decode(errors='replace')}")
    payloads: list[dict[str, Any]] = []
    for run in (1, 2):
        raw = runs / f"probe-{run}.json"
        execution = run_linux(["timeout", "--kill-after=5s", str(timeout), linux_path(work / "oracle"),
                               linux_path(model), linux_path(raw)], work / "source", timeout)
        (runs / f"probe-{run}.stderr").write_bytes(execution.stderr)
        require(execution.returncode == 0, f"Probe failed for {case_id}: {execution.stderr.decode(errors='replace')}")
        payloads.append(load(raw))
    found = difference(*payloads)
    require(found is None, f"Non-repeatable C++ result for {case_id}: {found}")
    data = payloads[0]
    require(data["variant"] == variant and data["orbifold_group"]["label"] == label, "Probe case identity mismatch")
    if case_id.endswith("_wl"):
        require(any(any(component != 0 for component in line) for line in data["orbifold_group"]["wilson_lines"]), "Wilson-line case has only zero Wilson lines")
    data["case_id"] = case_id
    data["input"] = {"model_text": text, "model_sha256": sha(model), "geometry_file": geometry,
                     "geometry_sha256": sha(actual_geometry)}
    normalize_metric(data, geometry)
    data["provenance"] = {
        "legacy_binary_sha256": sha(binary), "probe_binary_sha256": sha(work / "oracle"),
        "cpp_sources_sha256": sha(work / "cpp-built.sha256"), "source_tree_sha256": sha(work / "original.sha256"),
        "probe_source_sha256": sha(ROOT / "probes/oracle.cpp"), "build_probes_sha256": sha(ROOT / "build_probes.sh"),
        "generator_sha256": sha(Path(__file__)), "comparator_sha256": sha(ROOT / "compare.py"),
        "schema_sha256": sha(SCHEMA), "build_environment": (work / "environment.txt").read_text() + "\nProbe:\n" + (work / "oracle-environment.txt").read_text(),
        "python_version": platform.python_version(), "jsonschema_version": importlib.metadata.version("jsonschema"),
        "linked_inputs": objects,
    }
    validate(data)
    output.mkdir(parents=True, exist_ok=True)
    target = output / f"{case_id}.json"
    if target.exists() and not overwrite:
        old = load(target)
        found = difference(old, data)
        require(found is None, f"Frozen fixture differs: {found}; review before --overwrite")
    else:
        temporary = target.with_suffix(".json.tmp")
        temporary.write_text(json.dumps(data, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n",
                             encoding="utf-8", newline="\n")
        temporary.replace(target)
    analysis = data["susy_analysis"]
    allowed = sum(t["allowed_couplings"] for t in analysis.get("trials", []))
    audit = tachyon_audit(data)
    if audit:
        print(f"Tachyon audit: {sum(bool(x['right_oscillators']) for x in audit)} R-excited fields, "
              f"{sum(x['label_mismatch'] for x in audit)} native enum/oscillator mismatches", flush=True)
    print(f"PASS {case_id}: {len(data['sectors'])} sectors, {data['spectrum']['field_count']} fields, "
          f"{data['tachyonic_spectrum']['field_count']} tachyon fields, anomalies={data['anomalies']['check_passed']}, "
          f"{allowed} sampled cubic couplings; two identical probe runs", flush=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", action="append", choices=[name for name, _ in CASES])
    parser.add_argument("--build-root", type=Path, default=Path(os.environ.get("LEGACY_BUILD_ROOT", ROOT / ".build")))
    parser.add_argument("--output", type=Path, default=ROOT / "fixtures")
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--build-probes", action="store_true")
    parser.add_argument("--overwrite", action="store_true", help="Explicitly replace reviewed fixtures")
    args = parser.parse_args()
    if args.build_probes:
        for variant in ("susy", "non_susy"):
            command = ["env", f"LEGACY_BUILD_ROOT={linux_path(args.build_root)}", "bash", linux_path(ROOT / "build_probes.sh"), variant]
            built = run_linux(command, REPO, args.timeout)
            require(built.returncode == 0, built.stderr.decode(errors="replace"))
            print(built.stdout.decode(), end="", flush=True)
    prepare_cases()
    for case_id, variant in CASES:
        if not args.case or case_id in args.case:
            generate_case(case_id, variant, args.build_root.resolve(), args.output.resolve(), args.timeout, args.overwrite)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, StopIteration, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error)) from error
