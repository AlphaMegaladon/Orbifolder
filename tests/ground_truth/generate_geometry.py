"""Supplement seven frozen models with native geometry and rejected embeddings."""
import argparse
import json
from fractions import Fraction
from generate import CASES, ROOT, REPO, check_sources, linked_inputs, linux_path, run_linux, sha
from compare import difference, load, require


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()
    result = {"schema_version": 1, "cases": {}, "positive_cases": {}, "negative_cases": {}, "legacy_conflicts": {}, "provenance": {}}
    for variant in ("susy", "non_susy"):
        work = ROOT / ".build" / variant
        check_sources(work, variant)
        result["provenance"][variant] = {"binary_sha256": sha(work / "geometry-oracle"),
            "cpp_manifest_sha256": sha(work / "cpp-built.sha256"), "linked_inputs": linked_inputs(work),
            "legacy_build_environment": (work / "environment.txt").read_text()}
    jobs = [(case, variant, ROOT / "cases" / f"{case}.txt", False) for case, variant in CASES]
    directory = ROOT / ".build/geometry-negatives"
    directory.mkdir(exist_ok=True)
    for variant in ("susy", "non_susy"):
        path = directory / f"{variant}_spin32_z3.txt"
        path.write_text((ROOT / "cases" / f"{variant}_z3.txt").read_text().replace("Lattice:E8xE8", "Lattice:Spin32"), encoding="utf-8", newline="\n")
        jobs.append((f"{variant}_spin32_z3", variant, path, False))
        shifts = 2 if variant == "susy" else 3
        # Isolate a Wilson self-congruence: order three and alias relations hold;
        # all shift products vanish, but 3*W^2=2/3 is not an even integer.
        lines = (ROOT / "cases" / f"{variant}_z3.txt").read_text().splitlines()
        start = lines.index("Shifts and Wilsonlines:") + 1
        for i in (0, 1): lines[start + shifts + i] = " ".join(["0"] * 8 + ["1/3", "1/3"] + ["0"] * 6)
        path = directory / f"{variant}_wl_norm.txt"
        path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        jobs.append((f"{variant}_wl_norm", variant, path, variant == "susy"))
        for kind in ("shift_order", "shift_norm", "wl_order", "wl_relation", "shift_pair"):
            base = f"{variant}_z3xz3_wl" if kind == "shift_pair" else f"{variant}_z3"
            lines = (ROOT / "cases" / f"{base}.txt").read_text().splitlines()
            start = lines.index("Shifts and Wilsonlines:") + 1
            geometric = start + (1 if variant == "non_susy" else 0)
            if kind == "shift_order":
                values = lines[geometric].split(); values[0] = str(Fraction(values[0]) + Fraction(1, 2))
                lines[geometric] = " ".join(values)
            elif kind == "shift_norm":
                lines[geometric] = " ".join(["1/3", "1/3"] + ["0"] * 14)
            elif kind == "wl_order":
                for i in (0, 1): lines[start + shifts + i] = " ".join(["1/2", "-1/2"] + ["0"] * 14)
            elif kind == "wl_relation":
                lines[start + shifts] = " ".join(["1/3", "1/3", "-2/3"] + ["0"] * 13)
            else:
                lines[geometric + 1] = " ".join(str(-Fraction(x)) for x in lines[geometric + 1].split())
            name = f"{variant}_{kind}"
            path = directory / f"{name}.txt"
            path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
            jobs.append((name, variant, path, True))
    for name, variant, model, negative in jobs:
        work = ROOT / ".build" / variant
        runs = work / "runs" / name; runs.mkdir(parents=True, exist_ok=True)
        payloads = []
        for i in (1, 2):
            raw = runs / f"geometry-{i}.json"
            executed = run_linux(["timeout", "--kill-after=5s", "180", linux_path(work / "geometry-oracle"),
                                  linux_path(model), linux_path(raw)], work / "source", 180)
            (runs / f"geometry-{i}.stderr").write_bytes(executed.stderr)
            require(executed.returncode == 0, executed.stderr.decode(errors="replace"))
            payloads.append(load(raw))
        require(difference(*payloads) is None, f"Non-repeatable geometry oracle: {name}")
        require(payloads[0]["loaded"] != negative, f"Unexpected native acceptance: {name}")
        entry = {"native": payloads[0], "model_text": model.read_text(), "model_sha256": sha(model), "variant": variant}
        is_fixture = name in {case for case, _ in CASES}
        if is_fixture: entry["fixture_sha256"] = sha(ROOT / "fixtures" / f"{name}.json")
        category = "legacy_conflicts" if name == "non_susy_wl_norm" else "negative_cases" if negative else "cases" if is_fixture else "positive_cases"
        result[category][name] = entry
        print(f"PASS {name}: native loaded={payloads[0]['loaded']}, two identical geometry-only runs", flush=True)
    compiler = run_linux(["g++", "--version"], REPO, 30)
    require(compiler.returncode == 0, "Cannot identify C++ compiler")
    result["provenance"].update({"compiler": compiler.stdout.decode(),
        "probe_source_sha256": sha(ROOT / "probes/geometry_probe.cpp"),
        "build_script_sha256": sha(ROOT / "build_geometry_probes.sh"), "generator_sha256": sha(ROOT / "generate_geometry.py")})
    target = ROOT / "geometry_oracle.json"
    if target.exists() and not args.overwrite:
        require(difference(load(target), result) is None, "Frozen geometry oracle differs; review before --overwrite")
    else:
        target.write_text(json.dumps(result, sort_keys=True, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
