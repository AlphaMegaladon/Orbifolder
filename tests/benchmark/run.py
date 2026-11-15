"""Reproduce measured seven-model timings; C++ preparation/validation is untimed."""
import argparse
import datetime
import hashlib
import json
import os
import platform
import resource
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/ground_truth"))
from generate import check_sources, linked_inputs, sha


def command(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, cwd=ROOT, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--output", type=Path, default=ROOT / "tests/benchmark/latest.json")
    parser.add_argument("--build-root", type=Path,
                        default=os.environ.get("LEGACY_BUILD_ROOT", ROOT / "tests/ground_truth/.build"),
                        help="Fresh C++ build root (also accepts LEGACY_BUILD_ROOT)")
    args = parser.parse_args()
    if args.repeats < 1 or sys.platform != "linux":
        parser.error("Use Linux/WSL and repeats >=1")
    work = args.build_root.expanduser().resolve()
    cpp_provenance = {}
    for variant in ("susy", "non_susy"):
        directory = work / variant
        check_sources(directory, variant)
        cpp_provenance[variant] = {
            "binary_sha256": sha(directory / "oracle"),
            "linked_inputs": linked_inputs(directory),
        }
    inputs = ROOT / "tests/python/model_inputs.json"
    rust = ROOT / "target/release/examples/snapshot"
    start = datetime.datetime.now(datetime.timezone.utc)
    measured = command([str(rust), str(inputs), "bench", str(args.repeats)])
    report = json.loads(measured.stdout)
    report["rust_peak_child_rss_kib"] = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    source_hash = hashlib.sha256()
    for file in sorted((ROOT / "crates").rglob("*.rs")):
        source_hash.update(file.relative_to(ROOT).as_posix().encode())
        source_hash.update(file.read_bytes())
    report["provenance"] = {
        "started_utc": start.isoformat(), "python": sys.version,
        "platform": platform.platform(), "logical_cpus": os.cpu_count(),
        "cpu": command(["lscpu"]).stdout,
        "rustc": command([str(Path.home() / ".cargo/bin/rustc"), "--version"]).stdout.strip(),
        "g++": command(["g++", "--version"]).stdout.splitlines()[0],
        "rust_sources_sha256": source_hash.hexdigest(), "cargo_lock_sha256": sha(ROOT / "Cargo.lock"),
        "inputs_sha256": sha(inputs), "runner_sha256": sha(Path(__file__)),
        "rust_profile": "release; overflow-checks=true", "cpp": cpp_provenance,
    }
    entries = json.loads(inputs.read_text())
    fixtures = {case: json.loads((ROOT / f"tests/ground_truth/fixtures/{case}.json").read_text()) for case in entries}
    samples = []
    per_model = {case: [] for case in entries}
    for _ in range(args.repeats):
        elapsed = 0.0
        for case in entries:
            directory = work / ("non_susy" if case.startswith("non_") else "susy")
            with tempfile.TemporaryDirectory(prefix="orbifolder-benchmark-") as tmp:
                runtime = Path(tmp)
                for folder in ("Geometry", "GaugeInvariance"):
                    shutil.copytree(directory / "source" / folder, runtime / folder)
                model = ROOT / f"tests/ground_truth/cases/{case}.txt"
                output = runtime / "result.json"
                before = time.perf_counter()
                subprocess.run([str(directory / "oracle"), str(model), str(output)], cwd=runtime,
                               env={**os.environ, "LC_ALL": "C", "TZ": "UTC"}, stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, check=True, timeout=180)
                seconds = time.perf_counter() - before
                native = json.loads(output.read_text())
                assert native["spectrum"]["field_count"] == fixtures[case]["spectrum"]["field_count"]
                assert native["tachyonic_spectrum"]["field_count"] == fixtures[case]["tachyonic_spectrum"]["field_count"]
                per_model[case].append(seconds)
                elapsed += seconds
        samples.append(elapsed)
    report["cpp"] = {"seconds": samples, "per_model_seconds": per_model,
        "scope": "fresh C++ process: model/geometry load, native solve/anomalies/cubic candidate checks and JSON write; isolated directory copying and validation excluded"}
    import orbifolder
    model_objects = [orbifolder.Model.from_data(v["model"]) for v in entries.values()]
    results = [m.compute() for m in model_objects]
    before = time.perf_counter()
    payloads = [r.to_data() for r in results]
    report["python_export_seconds"] = time.perf_counter() - before
    report["massless_fields"] = sum(r.massless_count for r in results)
    report["tachyon_fields"] = sum(r.tachyon_count for r in results)
    report["scope"] = "Rust timings exclude input validation, pool startup and canonical/Python export; no discarded warmup; raw repetitions retained"
    assert len(payloads) == 7
    for variant in ("susy", "non_susy"):
        check_sources(work / variant, variant)
    report["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, sort_keys=True, indent=2) + "\n")
    print(f"PASS benchmark: {args.output}", flush=True)


if __name__ == "__main__":
    main()
