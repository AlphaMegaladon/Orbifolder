import json
from pathlib import Path
import pytest
import orbifolder

ROOT = Path(__file__).resolve().parents[2]
CASES = ("susy_z3", "susy_z6_ii", "susy_z3xz3_wl", "non_susy_z3", "non_susy_z6_ii", "non_susy_z3xz3_wl", "non_susy_tachyonic")


@pytest.fixture(scope="session")
def inputs():
    return json.loads((ROOT / "tests/python/model_inputs.json").read_text())


@pytest.fixture(scope="session")
def models(inputs):
    return {case: orbifolder.Model.from_data(inputs[case]["model"]) for case in CASES}


@pytest.fixture(scope="session")
def results(models):
    batch = orbifolder.Executor().compute_many([models[case] for case in CASES])
    assert all(item.ok for item in batch), [item.error for item in batch]
    return {case: item.value for case, item in zip(CASES, batch)}


@pytest.fixture(scope="session")
def snapshots(results):
    return {case: result.to_data() for case, result in results.items()}


@pytest.fixture(scope="session")
def fixtures():
    return {case: json.loads((ROOT / f"tests/ground_truth/fixtures/{case}.json").read_text()) for case in CASES}
