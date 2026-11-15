"""Freeze supplemental right-sector observations from unmodified C++ objects."""
import argparse
import json

from generate import ROOT, REPO, check_sources, linked_inputs, linux_path, run_linux, sha
from compare import difference, load, require


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()
    work = ROOT / ".build/non_susy"
    check_sources(work, "non_susy")
    runs = work / "runs/states"
    runs.mkdir(parents=True, exist_ok=True)
    payloads = []
    for i in (1, 2):
        raw = runs / f"states-{i}.json"
        execution = run_linux(
            ["timeout", "--kill-after=5s", "180", linux_path(work / "states-oracle"), linux_path(raw)],
            work / "source", 180,
        )
        (runs / f"states-{i}.stderr").write_bytes(execution.stderr)
        require(execution.returncode == 0, execution.stderr.decode(errors="replace"))
        payloads.append(load(raw))
    require(difference(*payloads) is None, "Non-repeatable right-sector oracle")
    compiler = run_linux(["g++", "--version"], REPO, 30)
    require(compiler.returncode == 0, "Cannot identify probe compiler")
    result = {"schema_version": 1, "native": payloads[0], "provenance": {
        "binary_sha256": sha(work / "states-oracle"),
        "cpp_manifest_sha256": sha(work / "cpp-built.sha256"),
        "linked_inputs": linked_inputs(work),
        "legacy_build_environment": (work / "environment.txt").read_text(),
        "compiler": compiler.stdout.decode(),
        "probe_source_sha256": sha(ROOT / "probes/states_probe.cpp"),
        "build_script_sha256": sha(ROOT / "build_states_probe.sh"),
        "generator_sha256": sha(ROOT / "generate_states.py"),
        "independent_processes": 2,
    }}
    target = ROOT / "states_oracle.json"
    if target.exists() and not args.overwrite:
        require(difference(load(target), result) is None, "Frozen states oracle differs; review before --overwrite")
    else:
        target.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8", newline="\n")
    print("PASS: two independent C++ processes; excited tachyons and finite-window counterexample")


if __name__ == "__main__":
    main()
