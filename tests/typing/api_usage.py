"""Valid static client checked against the installed package on Python 3.11."""
from fractions import Fraction
from pathlib import Path
from typing import assert_type
import orbifolder


def inspect_api(text: str, path: Path) -> None:
    geometry = orbifolder.Geometry.from_preset("Z3")
    assert_type(geometry, orbifolder.Geometry)
    models = orbifolder.parse_legacy_models(text, geometry)
    assert_type(models, list[orbifolder.Model])
    model = orbifolder.Model.from_data(models[0].to_data())
    assert_type(model.label, str)
    orbifolder.load_legacy_file(path, "Z3", theory_name="susy")
    orbifolder.Geometry.from_data(geometry.to_data())
    result = model.compute(tachyon_search="complete")
    assert_type(result, orbifolder.Result)
    summary = result.summary()
    assert_type(summary["massless_count"], int)
    assert_type(summary["is_anomaly_free"], bool | None)
    assert_type(result.fields[0].to_data()["u1_charges"][0]["num"], int)
    for item in geometry.evaluate_batch_summaries(models, threads=4):
        if item["status"] == "ok":
            assert_type(item["summary"]["gauge_factors"], list[str])
        else:
            assert_type(item["error"]["message"], str)
    for batch_item in orbifolder.Executor(2).compute_many(models):
        if batch_item.value is not None:
            batch_item.value.superpotential([1, 3], max_degree=3)
    pair = Fraction(1, 3)
    data = model.to_data()
    data["shifts"][0][0] = pair
    orbifolder.Model.from_data(data)
