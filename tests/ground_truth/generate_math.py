"""Freeze supplemental native math observations without altering model fixtures."""
import argparse
import json

from generate import CASES, ROOT, REPO, check_sources, linked_inputs, linux_path, run_linux, sha
from compare import difference, load, require


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()
    result = {"schema_version": 1, "primitives": {}, "cases": {}, "provenance": {}}
    for variant in ("susy", "non_susy"):
        work = ROOT / ".build" / variant
        check_sources(work, variant)
        result["provenance"][variant] = {"binary_sha256": sha(work / "math-oracle"),
            "cpp_manifest_sha256": sha(work / "cpp-built.sha256"), "linked_inputs": linked_inputs(work),
            "legacy_build_environment": (work / "environment.txt").read_text()}
    compiler = run_linux(["g++", "--version"], REPO, 30)
    require(compiler.returncode == 0, "Cannot identify mathematical probe compiler")
    result["provenance"]["math_probe_compiler"] = compiler.stdout.decode()
    for case, variant in CASES:
        work = ROOT / ".build" / variant
        model = ROOT / "cases" / f"{case}.txt"
        runs = work / "runs" / case
        payloads = []
        for i in (1, 2):
            raw = runs / f"math-{i}.json"
            execution = run_linux(["timeout", "--kill-after=5s", "180", linux_path(work / "math-oracle"),
                                   linux_path(model), linux_path(raw)], work / "source", 180)
            (runs / f"math-{i}.stderr").write_bytes(execution.stderr)
            require(execution.returncode == 0, execution.stderr.decode(errors="replace"))
            payloads.append(load(raw))
        require(difference(*payloads) is None, f"Non-repeatable native math for {case}")
        data = payloads[0]
        if variant in result["primitives"]:
            require(difference(result["primitives"][variant], data["primitives"]) is None, "Math primitives depend on model")
        result["primitives"][variant] = data["primitives"]
        fixture = ROOT / "fixtures" / f"{case}.json"
        require(load(fixture)["input"]["model_sha256"] == sha(model), "Frozen model changed")
        result["cases"][case] = {"factors": data["factors"], "fixture_sha256": sha(fixture)}
        print(f"PASS {case}: native Cartan/Dynkin data, two identical math probes", flush=True)
    result["provenance"].update({"probe_source_sha256": sha(ROOT / "probes/math_probe.cpp"),
        "build_script_sha256": sha(ROOT / "build_math_probes.sh"), "generator_sha256": sha(ROOT / "generate_math.py")})
    target = ROOT / "math_oracle.json"
    if target.exists() and not args.overwrite:
        require(difference(load(target), result) is None, "Frozen math oracle differs; review before --overwrite")
    else:
        target.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
