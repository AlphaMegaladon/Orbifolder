import copy
import json
import sys
import threading
import time
from fractions import Fraction
from concurrent.futures import ThreadPoolExecutor
import pytest
import orbifolder
from conftest import CASES


@pytest.mark.parametrize("case", CASES)
def test_model_roundtrips(case, inputs, models, results):
    raw = models[case].to_data()
    assert raw == inputs[case]["model"]
    geometry = orbifolder.Geometry.from_data(raw["geometry"])
    imported = orbifolder.Model.from_text(inputs[case]["model_text"], geometry, raw["theory"], raw["rules"])
    assert imported.to_data() == raw
    assert imported.compute().to_json() == results[case].to_json()


def test_exact_rationals_and_input_errors(inputs):
    data = copy.deepcopy(inputs["susy_z3"]["model"])
    huge = 1 << 300
    data["shifts"][0][0] = Fraction(-huge, 997)
    model = orbifolder.Model.from_data(data)
    assert model.to_data()["shifts"][0][0] == {"num": -huge, "den": 997}
    pair = orbifolder.exact_data(model.to_data(), rational_format="pair")
    assert pair["shifts"][0][0] == (-huge, 997)
    assert orbifolder.exact_data(model.to_data())["shifts"][0][0] == Fraction(-huge, 997)
    # Beyond CPython's default 4,300-decimal-digit JSON conversion limit.
    gigantic = 1 << 20000
    data["shifts"][0][0] = (gigantic, -997)
    restored = orbifolder.Model.from_data(data).to_data()["shifts"][0][0]
    assert restored == {"num": -gigantic, "den": 997}
    for invalid in (1.0, True, {"num": 1, "den": 0}, [1, 0]):
        broken = copy.deepcopy(inputs["susy_z3"]["model"])
        broken["shifts"][0][0] = invalid
        with pytest.raises((orbifolder.InputError, TypeError, ValueError)):
            orbifolder.Model.from_data(broken)
    with pytest.raises(orbifolder.InputError):
        orbifolder.Model.from_data({"schema_version": 999})
    with pytest.raises(orbifolder.ExecutionError):
        orbifolder.Executor(0)
    cyclic = {}
    cyclic["cycle"] = cyclic
    with pytest.raises(orbifolder.InputError):
        orbifolder.Model.from_data(cyclic)


@pytest.mark.parametrize("threads", [1, 2, 4, 8])
def test_parallel_matches_serial(threads, inputs, models, results):
    broken = copy.deepcopy(inputs["susy_z3"]["model"])
    broken["wilson_lines"][0][0] = (1, 997)
    invalid = orbifolder.Model.from_data(broken)
    requests = [models[case] for case in CASES] + [invalid]
    reference = orbifolder.Executor().compute_many(requests)
    parallel = orbifolder.Executor(threads).compute_many(requests)
    assert [r.to_json() for r in reference] == [r.to_json() for r in parallel]
    assert [r.index for r in parallel] == list(range(8))
    assert not parallel[-1].ok and parallel[-1].error["category"] == "model"
    for case, item in zip(CASES, parallel):
        assert item.value.to_json() == results[case].to_json()


def test_parallel_callers_and_owned_fields(models, results):
    executor = orbifolder.Executor(2)
    requests = [models[case] for case in CASES]
    with ThreadPoolExecutor(2) as callers:
        batches = list(callers.map(lambda _: executor.compute_many(requests), range(2)))
    expected = [results[case].to_json() for case in CASES]
    assert all([i.value.to_json() for i in batch] == expected for batch in batches)
    field = models["susy_z3"].compute().fields[3]
    retained = field.to_data()
    assert field.id == 3 and not field.is_tachyon
    with pytest.raises(AttributeError):
        field.id = 5
    retained["u1_charges"].append({"num": 1, "den": 1})
    assert field.to_data() != retained


def test_gil_is_released(models):
    ready, stop = threading.Event(), threading.Event()
    progress = [0]

    def worker():
        ready.set()
        while not stop.is_set():
            progress[0] += 1
            time.sleep(0.001)

    old_interval = sys.getswitchinterval()
    sys.setswitchinterval(10.0)
    thread = threading.Thread(target=worker)
    try:
        thread.start()
        ready.wait()
        before = progress[0]
        models["susy_z3xz3_wl"].compute()
        assert progress[0] > before, "Python worker did not run during native calculation"
    finally:
        stop.set()
        thread.join()
        sys.setswitchinterval(old_interval)


def test_explicit_resource_errors(models):
    with pytest.raises(orbifolder.ModelError) as error:
        models["susy_z3"].compute(search_nodes=0)
    assert error.value.category in ("physics", "susy")
    with pytest.raises(orbifolder.InputError):
        models["susy_z3"].compute(tachyon_search="guessed")
