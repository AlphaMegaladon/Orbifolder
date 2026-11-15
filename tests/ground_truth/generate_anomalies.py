"""Freeze native index and polarized CheckAnomaly observations for seven models."""
import argparse
import json
import re

from generate import CASES, ROOT, REPO, check_sources, linked_inputs, linux_path, run_linux, sha
from compare import difference, load, require


def coefficients(report: str) -> list[dict]:
    result = []
    pattern = re.compile(r"^\s*(.*?)\s*=\s*(-?[0-9]+)(?:/([0-9]+))?\s*$")
    for line in report.splitlines():
        match = pattern.fullmatch(line)
        if match and ("tr " in match[1] or "#(doublets)" in match[1]):
            result.append({"label": match[1], "value": {"num": int(match[2]), "den": int(match[3] or 1)}})
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()
    result = {"schema_version": 1, "cases": {}, "provenance": {"independent_processes": 2}}
    for variant in ("susy", "non_susy"):
        work = ROOT / ".build" / variant
        check_sources(work, variant)
        result["provenance"][variant] = {
            "binary_sha256": sha(work / "anomalies-oracle"),
            "cpp_manifest_sha256": sha(work / "cpp-built.sha256"),
            "linked_inputs": linked_inputs(work),
            "legacy_build_environment": (work / "environment.txt").read_text(),
        }
    for case, variant in CASES:
        work = ROOT / ".build" / variant
        runs = work / "runs" / case
        model = ROOT / "cases" / f"{case}.txt"
        payloads = []
        for i in (1, 2):
            raw = runs / f"anomalies-{i}.json"
            execution = run_linux(["timeout", "--kill-after=5s", "180", linux_path(work / "anomalies-oracle"),
                                   linux_path(model), linux_path(raw)], work / "source", 180)
            (runs / f"anomalies-{i}.stderr").write_bytes(execution.stderr)
            require(execution.returncode == 0, execution.stderr.decode(errors="replace"))
            payloads.append(load(raw))
        require(difference(*payloads) is None, f"Non-repeatable anomaly oracle: {case}")
        data = payloads[0]
        for polarization in data["polarizations"]:
            polarization["native"]["coefficients"] = coefficients(polarization["native"]["report"])
            require(any(c["label"] == "tr Q_1^3" for c in polarization["native"]["coefficients"]), "Missing native cubic trace")
        if data["removed_chiral"] is not None:
            native = data["removed_chiral"]["native"]
            native["coefficients"] = coefficients(native["report"])
        fixture = ROOT / "fixtures" / f"{case}.json"
        require(load(fixture)["input"]["model_sha256"] == sha(model), "Frozen model changed")
        result["cases"][case] = {"native": data, "fixture_sha256": sha(fixture)}
        print(f"PASS {case}: indices, {len(data['polarizations'])} native cubic polarizations, two processes", flush=True)
    compiler = run_linux(["g++", "--version"], REPO, 30)
    require(compiler.returncode == 0, "Cannot identify compiler")
    result["provenance"].update({"compiler": compiler.stdout.decode(),
        "probe_source_sha256": sha(ROOT / "probes/anomalies_probe.cpp"),
        "build_script_sha256": sha(ROOT / "build_anomalies_probes.sh"),
        "generator_sha256": sha(ROOT / "generate_anomalies.py")})
    target = ROOT / "anomalies_oracle.json"
    if target.exists() and not args.overwrite:
        require(difference(load(target), result) is None, "Frozen anomaly oracle differs; review before --overwrite")
    else:
        target.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
