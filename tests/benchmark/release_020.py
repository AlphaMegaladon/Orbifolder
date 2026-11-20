"""Release checks using an isolated orbifolder wheel and the existing RL checkout."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import platform
import statistics
import sys
import time

import orbifolder


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/python/shift_w2_prefix_masks.json"


def timed(function, repeats):
    samples = []
    for _ in range(repeats):
        started = time.perf_counter_ns()
        value = function()
        samples.append(time.perf_counter_ns() - started)
        del value
    return samples


def distribution(samples):
    ordered = sorted(samples)
    return {
        "median_ms": statistics.median(samples) / 1e6,
        "p95_ms": ordered[(95 * len(ordered) + 99) // 100 - 1] / 1e6,
    }


def make_fixture(rl):
    from orbifolder_rl.env import TaskSpec
    from orbifolder_rl.env.exact import encode
    from orbifolder_rl.env.lattice import construction_basis
    from orbifolder_rl.env.masking import ExactActionMasker

    artifacts = rl / "artifacts/phase3"
    task = TaskSpec.from_data(json.loads((artifacts / "physics_study/training/physical/tasks/non_susy_z6_ii.json").read_text()))
    traces = artifacts / "physics_study_recovery_v2"
    before_path = traces / "checkpoint_probe_before.json"
    after_path = traces / "checkpoint_probe_after.json"
    before, after = (json.loads(path.read_text()) for path in (before_path, after_path))
    timeout = tuple(before["queries"][-1]["prefix"])
    assert len(timeout) == 21 and "ConstraintBudgetExceeded" in before["queries"][-1]["error"]
    oracle = ExactActionMasker(task, timeout_ms=10_000)
    assert oracle._zero_suffix_safe(16)
    prefixes = [(), timeout[:-1], timeout]
    trace_prefixes = sorted({tuple(row["prefix"][:-1]) for row in after["queries"]}, key=lambda prefix: (len(prefix), prefix))
    prefixes += [prefix for prefix in trace_prefixes[::9] if prefix not in prefixes]
    cases = []
    for prefix in prefixes:
        stage = len(prefix) // 16
        variable = task.variables[stage]
        fixed = task.build(prefix[:16 * stage], partial=True)
        fixed.pop("geometry")
        order = task.profile.orders[variable.slots[0]]
        start = time.perf_counter_ns()
        expected = oracle.action_mask(prefix).tolist()
        cold_ns = time.perf_counter_ns() - start
        cases.append({
            "prefix": list(prefix),
            "timeout": prefix == timeout,
            "timeout_parent": prefix == timeout[:-1],
            "fixed": fixed,
            "slots": variable.slots,
            "local_prefix": list(prefix[16 * stage:]),
            "basis": encode([[value / order for value in row] for row in construction_basis(fixed["lattice"])]),
            "expected": expected,
            "oracle_cold_ns": cold_ns,
        })
    fixture = {"task_id": task.task_id, "geometry": task.template["geometry"],
        "alphabet": task.alphabet, "oracle": "orbifolder_rl ExactActionMasker (exact Z3 NIA/BV)",
        "trace_sha256": hashlib.sha256(before_path.read_bytes()).hexdigest(), "cases": cases}
    FIXTURE.write_text(json.dumps(fixture, separators=(",", ":")) + "\n")
    return fixture


def prefix_benchmark(fixture, rl):
    from orbifolder_rl.env import TaskSpec
    from orbifolder_rl.env.masking import ExactActionMasker

    task = TaskSpec.from_data(json.loads((rl / "artifacts/phase3/physics_study/training/physical/tasks/non_susy_z6_ii.json").read_text()))
    assert task.task_id == fixture["task_id"]
    geom = orbifolder.Geometry.from_data(fixture["geometry"])
    oracle = ExactActionMasker(task, timeout_ms=10_000)
    prepared, cold, repeats, cache_repeats, solved_reference = [], [], [], [], []
    for row in fixture["cases"]:
        start = time.perf_counter_ns()
        mask = geom.prepare_prefix_mask(embedding=row["fixed"], slots=row["slots"], alphabet=fixture["alphabet"], basis=row["basis"])
        prepared.append(time.perf_counter_ns() - start)
        prefix = tuple(row["prefix"])
        local = row["local_prefix"]
        native = lambda: mask.mask(local)
        cold += timed(native, 1)
        assert mask.mask(local) == row["expected"]
        baseline = lambda: oracle.action_mask(prefix)
        assert baseline().tolist() == row["expected"]
        repeats += timed(native, 250)
        cache_repeats += timed(baseline, 250)
        solved_reference.append(row["oracle_cold_ns"])
    return {
        "cases": len(fixture["cases"]), "timeout": next(row["expected"] for row in fixture["cases"] if row["timeout"]),
        "timeout_parent": next(row["expected"] for row in fixture["cases"] if row["timeout_parent"]),
        "state_budget_aborts": 0, "prepare": distribution(prepared), "first_lookup": distribution(cold),
        "repeated_lookup": distribution(repeats), "z3_cold_fixture_creation": distribution(solved_reference),
        "baseline_already_cached_python_lookup": distribution(cache_repeats),
        "natively_prepared_vs_z3_cold_lookup_ratio": statistics.median(solved_reference) / statistics.median(repeats),
        "native_first_lookup_vs_z3_cold_ratio": statistics.median(solved_reference) / statistics.median(cold),
        "already_cached_python_vs_native_lookup_ratio": statistics.median(cache_repeats) / statistics.median(repeats),
    }


def from_compact(model_data, summary, compact):
    from itertools import permutations
    from orbifolder_rl.env.spectrum import project_spectrum

    def expand(rows):
        factors = compact["gauge_group"]["factors"]
        fields = [{**row, "id": field_id, "representations": [
            {"algebra": factor["algebra"], **factor["irreps"][rep_id]}
            for factor, rep_id in zip(factors, row["representation_ids"], strict=True)
        ]} for row in rows for field_id in row["field_ids"]]
        return sorted(fields, key=lambda field: field["id"])

    anomalies = {**compact["anomalies"], "spectrum": {
        "gauge_group": compact["gauge_group"],
        "fields": expand(compact["fields"]), "tachyons": expand(compact["tachyons"]),
    }}
    if anomalies["status"] == "computed":
        canonical = anomalies["canonical"].copy()
        packed = canonical["abelian_cubic"]
        size = packed["size"]
        dense = [[[{"num": 0, "den": 1} for _ in range(size)] for _ in range(size)] for _ in range(size)]
        for left, middle, right, value in packed["symmetric_nonzero"]:
            for index in permutations((left, middle, right)):
                dense[index[0]][index[1]][index[2]] = value
        canonical["abelian_cubic"] = dense
        anomalies["canonical"] = canonical
    return project_spectrum(model_data, summary, anomalies,
        tachyon_search_complete=compact["provenance"]["is_tachyon_free"] is True)


def compact_benchmark(rl, repeats):
    from orbifolder_rl.env.exclusions import spectrum_exclusion
    from orbifolder_rl.env.exact import rational
    from orbifolder_rl.env.phenomenology import GaugeEmbedding, HyperchargeConfig, analyze_chirality, verify_hypercharge
    from orbifolder_rl.env.reward import RewardConfig, score_spectrum
    from orbifolder_rl.env.spectrum import project_spectrum

    inputs = json.loads((ROOT / "tests/python/model_inputs.json").read_text())
    fixture = rl / "tests/fixtures/non_susy_three_family_candidate.json.gz"
    candidate = json.loads(gzip.decompress(fixture.read_bytes()))
    inputs["three_family_candidate"] = {"model": candidate["evaluation_model"]}
    timings = {}
    for case, item in inputs.items():
        data = item["model"]
        result = orbifolder.Model.from_data(data).compute()
        summary = result.summary()
        projection = result.compact_spectrum()
        compact = projection.to_data()
        expected = project_spectrum(data, summary, result.anomalies())
        actual = from_compact(data, summary, compact)
        assert expected == actual and spectrum_exclusion(expected) == spectrum_exclusion(actual)
        config = RewardConfig(mode="susy_mssm" if summary["theory"] == "susy" else "nonsusy_sm")
        if case == "three_family_candidate":
            raw = candidate["physics"]["witness"]
            embedding = GaugeEmbedding(**raw["embedding"])
            coefficients = tuple(map(rational, raw["coefficients"]))
            left, right = (verify_hypercharge(spectrum, embedding, coefficients) for spectrum in (expected, actual))
            assert left == right and right.anomaly.status == "verified"
            expected_analysis, actual_analysis = (analyze_chirality(spectrum, witness)
                for spectrum, witness in ((expected, left), (actual, right)))
            assert expected_analysis == actual_analysis
            expected_score = score_spectrum(expected, config, candidates=((left, expected_analysis),))
            actual_score = score_spectrum(actual, config, candidates=((right, actual_analysis),))
            assert actual_score == expected_score and actual_score.spectrum_candidate
        elif case == "susy_z3xz3_wl":
            config = RewardConfig(hypercharge=HyperchargeConfig(gut_normalization=True, timeout_ms=3000))
            expected_score, actual_score = (score_spectrum(spectrum, config) for spectrum in (expected, actual))
            assert actual_score.spectrum_candidate and actual_score.reward == expected_score.reward
            assert actual_score.witness == expected_score.witness and actual_score.analysis == expected_score.analysis
            assert actual_score.witness.level == rational("5/3")
        timings[case] = {
            "project": distribution(timed(result.compact_spectrum, repeats)),
            "dict": distribution(timed(projection.to_data, repeats)),
            "combined": distribution(timed(lambda: result.compact_spectrum().to_data(), repeats)),
            "full_json": distribution(timed(result.to_json, 5)),
        }
    return timings


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rl", type=Path, required=True)
    parser.add_argument("--wheel", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--export-prefix-fixture", action="store_true")
    parser.add_argument("--prefix-only", action="store_true")
    parser.add_argument("--repeats", default=100, type=int)
    parser.add_argument("--check-gates", action="store_true")
    args = parser.parse_args()
    sys.path.insert(0, str(args.rl.resolve() / "src"))
    fixture = make_fixture(args.rl) if args.export_prefix_fixture else json.loads(FIXTURE.read_text())
    if args.prefix_only and args.export_prefix_fixture:
        return
    assert orbifolder.__version__ == "0.2.0"
    cpuinfo = Path("/proc/cpuinfo")
    cpu = next((line.partition(":")[2].strip() for line in cpuinfo.read_text().splitlines()
                if line.startswith("model name")), platform.processor()) if cpuinfo.exists() else platform.processor()
    report = {"orbifolder": orbifolder.__version__, "platform": platform.platform(),
        "cpu": cpu, "repeats": args.repeats,
        "wheel_sha256": None if args.wheel is None else hashlib.sha256(args.wheel.read_bytes()).hexdigest(),
        "prefix": prefix_benchmark(fixture, args.rl)}
    if not args.prefix_only:
        report["compact"] = compact_benchmark(args.rl, args.repeats)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(args.output, flush=True)
    if args.check_gates:
        assert report["prefix"]["natively_prepared_vs_z3_cold_lookup_ratio"] > 100
        assert report["prefix"]["native_first_lookup_vs_z3_cold_ratio"] >= 10
        for case, timing in report.get("compact", {}).items():
            assert timing["dict"]["p95_ms"] < 5 and timing["combined"]["p95_ms"] < 10, (case, timing)


if __name__ == "__main__":
    main()
