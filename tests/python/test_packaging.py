"""Installed-package ergonomics; these tests also run with repository I/O denied."""
import copy
from importlib.resources import files
import json
from pathlib import Path
import pytest
import orbifolder
from conftest import CASES


def preset_for(case):
    return "Z3xZ3" if "z3xz3" in case else ("Z6-II" if "z6_ii" in case or "tachyonic" in case else "Z3")


@pytest.mark.parametrize("case", CASES)
def test_installed_presets_reproduce_every_reference(case, inputs, results):
    entry = inputs[case]
    theory = entry["model"]["theory"]
    geometry = orbifolder.Geometry.from_preset(preset_for(case), theory_name=theory)
    assert geometry.to_data() == entry["model"]["geometry"]
    model, = orbifolder.parse_legacy_models(entry["model_text"], geometry)
    assert model.to_data() == entry["model"]
    assert model.compute().to_json() == results[case].to_json()
    shortcut, = orbifolder.parse_legacy_models(entry["model_text"], preset_for(case), theory_name=theory)
    assert shortcut.to_data() == model.to_data()


def test_wheel_resources_and_api_surface():
    assert orbifolder.Geometry.available_presets() == ["Z3", "Z6-II", "Z3xZ3"]
    assert orbifolder.Geometry.from_preset("Z6_II").to_data() == orbifolder.Geometry.from_preset("Z6-II").to_data()
    for name in orbifolder.Geometry.available_presets():
        resource = json.loads(files("orbifolder").joinpath(f"presets/{name}.json").read_text())
        assert resource["name"] == name
        for theory in ("susy", "non_susy"):
            assert resource[theory]["geometry"] == orbifolder.Geometry.from_preset(name, theory_name=theory).to_data()
    for filename in ("__init__.pyi", "_native.pyi", "py.typed"):
        assert files("orbifolder").joinpath(filename).is_file()
    assert all(hasattr(orbifolder, name) for name in orbifolder.__all__)
    with pytest.raises(orbifolder.InputError):
        orbifolder.Geometry.from_preset("Z5")
    with pytest.raises(orbifolder.InputError):
        orbifolder.Geometry.from_preset("Z3", theory_name="unknown")


def test_catalogs_file_loading_and_no_geometry_file_access(inputs, tmp_path):
    geometry = orbifolder.Geometry.from_preset("Z3")
    text = inputs["susy_z3"]["model_text"].replace("Geometry/Geometry_Z3_1_1.txt", "/does/not/exist")
    text = text.replace("end model", "Program filename:/never/execute/this\nend model")
    catalog = "Label:DO_NOT_READ_OUTSIDE_BLOCK\n" + text + "\nprint commands are not executed\n" + text
    models = orbifolder.parse_legacy_models(catalog, geometry)
    assert len(models) == 2 and models[0].to_data() == inputs["susy_z3"]["model"]
    path = tmp_path / "models with spaces.txt"
    path.write_text(catalog, encoding="utf-8")
    assert [m.to_data() for m in orbifolder.load_legacy_file(path, "Z3")] == [m.to_data() for m in models]
    assert len(orbifolder.load_legacy_file(str(path), geometry)) == 2
    with pytest.raises(FileNotFoundError):
        orbifolder.load_legacy_file(tmp_path / "missing.txt")
    with pytest.raises(TypeError):
        orbifolder.parse_legacy_models(text, 3)
    for invalid in ("", text.replace("end model", ""), "begin model\n" + text, "end model\n" + text, text.replace("Label:", "Label:a\nLabel:")):
        with pytest.raises(orbifolder.InputError):
            orbifolder.parse_legacy_models(invalid, geometry)
    for unsupported in ("discrete torsion", "use freely acting WL:", "begin U1 generators"):
        for position in ("Shifts and Wilsonlines:", "end model"):
            with pytest.raises(orbifolder.InputError, match="Unsupported legacy section"):
                orbifolder.parse_legacy_models(text.replace(position, unsupported + "\n" + position), geometry)


def test_legacy_evaluation_and_preset_selection_rules(inputs, results):
    entry = inputs["susy_z3"]
    geometry = orbifolder.Geometry.from_preset("Z3")
    model = orbifolder.Model.from_text(entry["model_text"], geometry)
    assert model.to_data()["rules"] == entry["model"]["rules"]
    item, = geometry.evaluate_legacy_text(entry["model_text"], threads=2)
    assert item.ok and item.value.to_json() == results["susy_z3"].to_json()
    model, = orbifolder.parse_legacy_models(entry["model_text"], geometry, rules=[])
    assert model.to_data()["rules"] == []


@pytest.mark.parametrize("case", CASES)
def test_summary_matches_full_result(case, results, snapshots):
    result, full = results[case], snapshots[case]
    summary = result.summary()
    assert summary["theory"] == ("susy" if case.startswith("susy_") else "non_susy")
    assert summary["massless_count"] == len(full["spectrum"]["fields"])
    assert summary["tachyon_count"] == len(full["spectrum"]["tachyons"])
    assert summary["is_tachyon_free"] == (case != "non_susy_tachyonic")
    assert summary["anomaly_status"] == "computed"
    assert summary["is_anomaly_free"] == full["anomalies"]["is_anomaly_free"]
    assert summary["gauge_factors"] == [f["algebra"] for f in full["spectrum"]["gauge_group"]["factors"]]
    assert summary["u1_count"] == len(full["spectrum"]["gauge_group"]["u1_generators"])
    assert summary["charged_singlet_count"] == sum(all(r["dimension"] == 1 for r in f["representations"]) and any(q["num"] != 0 for q in f["u1_charges"]) for f in full["spectrum"]["fields"])
    fi = full["anomalies"]["fi"]["susy_coefficient"].copy()
    fi["has_anomalous_u1"] = full["anomalies"]["rotation"]["anomalous_index"] is not None
    assert summary["fi"] == fi
    assert len(json.dumps(summary)) < 1500
    summary["gauge_factors"].clear()
    assert result.summary()["gauge_factors"]


@pytest.mark.parametrize("threads", [None, 1, 2, 4, 8])
def test_parallel_summaries_and_errors(threads, models, inputs, results):
    broken = copy.deepcopy(inputs["susy_z3"]["model"])
    broken["wilson_lines"][0][0] = (1, 997)
    requests = [models[c] for c in CASES] + [orbifolder.Model.from_data(broken)]
    executor = orbifolder.Executor(threads)
    summaries = executor.compute_summaries(requests)
    assert [s["index"] for s in summaries] == list(range(8))
    for case, item in zip(CASES, summaries):
        assert item == {"index": item["index"], "status": "ok", "summary": results[case].summary(), "error": None}
    assert summaries[-1]["status"] == "error"
    error, = executor.compute_many(requests[-1:])
    assert summaries[-1]["error"] == error.error


def test_shared_geometry_summaries_and_optional_anomalies(models, results):
    geometry = orbifolder.Geometry.from_preset("Z3")
    items = geometry.evaluate_batch_summaries([models["susy_z3"]] * 2, threads=2)
    assert [s["summary"] for s in items] == [results["susy_z3"].summary()] * 2
    unchecked, = geometry.evaluate_batch_summaries([models["susy_z3"]], anomalies=False)
    assert unchecked["summary"]["is_anomaly_free"] is None
    assert unchecked["summary"]["anomaly_status"] == "not_requested"
    assert unchecked["summary"]["fi"] == {"status": "not_requested", "has_anomalous_u1": None}
    assert geometry.evaluate_batch_summaries([]) == []
    with pytest.raises(orbifolder.InputError, match="Shared geometry"):
        geometry.evaluate_batch_summaries([models["susy_z6_ii"]])
    with pytest.raises(orbifolder.ExecutionError):
        geometry.evaluate_batch_summaries([], threads=0)
