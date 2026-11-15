"""Freeze native coupling/rule and character probes in isolated runtime folders."""
import argparse
import json
import shutil
import tempfile
from pathlib import Path
from fractions import Fraction

from generate import ROOT, REPO, check_sources, linked_inputs, linux_path, run_linux, sha
from compare import difference, load, require

CASES = ("susy_z3", "susy_z6_ii", "susy_z3xz3_wl")


def validate_native(probe: str, payload: dict, fixture: dict) -> None:
    if probe == "couplings":
        require(len(payload["trials"]) >= 56, "Incomplete cubic trial set")
        for t, frozen in zip(payload["trials"][:56], fixture["susy_analysis"]["trials"]):
            for new, old in (("fields", "field_indices"), ("original_add", "allowed_couplings"),
                             ("monomial_allowed", "gauge_invariant"), ("exponents", "monomial_exponents"),
                             ("monomial_fields", "monomial_fields"), ("charge_sum", "u1_charge_sum")):
                require(t[new] == frozen[old], f"Original cubic result changed: {new}")
        for t in payload["trials"]:
            require(1 <= len(t["fields"]) <= 3, "Unverified degree")
            require(all(fixture["spectrum"]["fields"][i]["multiplet"] == 2 for i in t["fields"]), "Non-chiral trial")
            require(len(t["r_charges"]) == 3, "Missing explicit R charges")
            for charges in t["r_charges"]:
                require(len(charges) == len(t["fields"]), "Missing field R charge")
                for q in charges:
                    require(Fraction(q["num"], q["den"]).denominator == q["den"], "Unreduced R charge")
        return
    records = payload["characters"]
    require(bool(records), "Missing native characters")

    def dimension(weights: list, rank: int) -> int:
        require(all(len(w["weight"]) == rank and w["multiplicity"] > 0 for w in weights), "Invalid weight")
        require(len({tuple(w["weight"]) for w in weights}) == len(weights), "Duplicate weights")
        return sum(w["multiplicity"] for w in weights)

    for r in records:
        require(dimension(r["weights"], len(r["highest"])) == r["dimension"], "Incomplete character")
    for p in payload["pairs"]:
        a, b = records[p["left"]], records[p["right"]]
        require(a["factor"] == b["factor"], "Cross-factor tensor")
        rank, d = len(a["highest"]), a["dimension"]
        require(dimension(p["tensor_character"], rank) == d * b["dimension"], "Incomplete tensor")
        require(p["native_decomposition_complete"] == ("tensor" in p), "Unqualified native decomposition")
        if p["left"] == p["right"]:
            require(dimension(p["symmetric_character"], rank) == d * (d + 1) // 2, "Incomplete Sym2")
            require(dimension(p["antisymmetric_character"], rank) == d * (d - 1) // 2, "Incomplete Alt2")
            if d <= 8:
                require(dimension(p["symmetric_cube_character"], rank) == d * (d + 1) * (d + 2) // 6, "Incomplete Sym3")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=CASES)
    parser.add_argument("--overwrite", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    work = ROOT / ".build/susy"
    check_sources(work, "susy")
    result = {"schema_version": 1, "cases": {}, "provenance": {
        "independent_processes": 2, "linked_inputs": linked_inputs(work),
        "cpp_manifest_sha256": sha(work / "cpp-built.sha256"),
        "legacy_build_environment": (work / "environment.txt").read_text(),
    }}
    runs = work / "runs/couplings"
    runs.mkdir(parents=True, exist_ok=True)
    for case in (args.case,) if args.case else CASES:
        model = ROOT / "cases" / f"{case}.txt"
        fixture = ROOT / "fixtures" / f"{case}.json"
        require(load(fixture)["input"]["model_sha256"] == sha(model), "Frozen input changed")
        observed = {}
        for probe in ("couplings", "characters"):
            payloads = []
            for i in (1, 2):
                runtime = Path(tempfile.mkdtemp(prefix=f"{case}-{probe}-{i}-", dir=runs))
                for directory in ("Geometry", "GaugeInvariance"):
                    shutil.copytree(work / "source" / directory, runtime / directory)
                raw = runtime / "native.json"
                command = ["timeout", "--kill-after=5s", "180", linux_path(work / f"{probe}-oracle"), linux_path(model), linux_path(raw)]
                if probe == "couplings":
                    command.append(case)
                execution = run_linux(command, runtime, 180)
                (runtime / "stderr.txt").write_bytes(execution.stderr)
                require(execution.returncode == 0, f"{case} {probe}: exit={execution.returncode}: {execution.stderr.decode(errors='replace')[-2000:]}")
                payload = load(raw)
                validate_native(probe, payload, load(fixture))
                payloads.append(payload)
            require(difference(*payloads) is None, f"Non-repeatable {case} {probe}")
            observed[probe] = payloads[0]
        require(observed["couplings"]["field_pool"] == load(fixture)["susy_analysis"]["field_pool"], "Native pool changed")
        result["cases"][case] = {"native": observed, "fixture_sha256": sha(fixture)}
        print(f"PASS {case}: {len(observed['couplings']['trials'])} rule trials, native characters/tensors, two processes", flush=True)
    for probe in ("couplings", "characters"):
        result["provenance"][probe] = {"binary_sha256": sha(work / f"{probe}-oracle"), "source_sha256": sha(ROOT / f"probes/{probe}_probe.cpp")}
    compiler = run_linux(["g++", "--version"], REPO, 30)
    require(compiler.returncode == 0, "Cannot identify compiler")
    result["provenance"].update({"compiler": compiler.stdout.decode(),
        "build_script_sha256": sha(ROOT / "build_couplings_probe.sh"), "generator_sha256": sha(ROOT / "generate_couplings.py")})
    target = args.output or (runs / "partial.json" if args.case else ROOT / "couplings_oracle.json")
    if target.exists() and not args.overwrite:
        require(difference(load(target), result) is None, "Frozen coupling oracle differs; review before --overwrite")
    else:
        target.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8", newline="\n")
    check_sources(work, "susy")


if __name__ == "__main__":
    main()
