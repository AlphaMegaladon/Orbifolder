"""Repeat the positive original C++ case under five glibc allocation profiles."""
from __future__ import annotations

import copy
import json

from compare import difference, load, require, tachyon_audit, validate
from generate import ROOT, check_sources, linked_inputs, linux_path, run_linux, sha


def main() -> None:
    work = ROOT / ".build/non_susy"
    model = ROOT / "cases/non_susy_tachyonic.txt"
    fixture = load(ROOT / "fixtures/non_susy_tachyonic.json")
    validate(fixture)
    check_sources(work, "non_susy")
    linked_inputs(work)
    require(sha(model) == fixture["input"]["model_sha256"], "Audit model differs from frozen input")
    require(sha(work / "oracle") == fixture["provenance"]["probe_binary_sha256"], "Audit probe differs from frozen oracle")
    expected = copy.deepcopy(fixture)
    for key in ("case_id", "input", "provenance"):
        del expected[key]
    del expected["geometry"]["exact_metric"]
    runs = work / "runs/non_susy_tachyonic"
    runs.mkdir(parents=True, exist_ok=True)
    results = []
    for i, perturb in enumerate((0, 1, 85, 170, 255) * 2):
        raw = runs / f"audit-{i}.json"
        execution = run_linux(["env", f"MALLOC_PERTURB_={perturb}", "timeout", "--kill-after=5s", "180",
                               linux_path(work / "oracle"), linux_path(model), linux_path(raw)], work / "source", 180)
        (runs / f"audit-{i}.stderr").write_bytes(execution.stderr)
        require(execution.returncode == 0, f"Audit probe failed: {execution.stderr.decode(errors='replace')}")
        data = load(raw)
        found = difference(expected, data)
        require(found is None, f"Native non-determinism or frozen-data difference, profile {perturb}: {found}")
        audit = tachyon_audit(data)
        require(bool(audit), "Audit has no projected tachyons")
        results.append({"run": i, "malloc_perturb": perturb, "raw_sha256": sha(raw),
                        "projected_fields": len(audit),
                        "r_excited_fields": sum(bool(x["right_oscillators"]) for x in audit),
                        "legacy_label_mismatches": sum(x["label_mismatch"] for x in audit)})
        print(f"PASS audit {i + 1}/10: MALLOC_PERTURB_={perturb}; "
              f"{len(audit)} projected tachyon fields; complete raw payload identical", flush=True)
    report = {"probe_binary_sha256": sha(work / "oracle"), "model_sha256": sha(model),
              "audit_source_sha256": sha(ROOT / "audit_tachyons.py"), "runs": results,
              "conclusion": "No process-to-process differences observed; allocator profiles do not prove absence of undefined behavior."}
    (runs / "audit-summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError) as error:
        raise SystemExit(str(error)) from error
