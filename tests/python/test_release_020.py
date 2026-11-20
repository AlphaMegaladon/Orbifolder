import json
from fractions import Fraction
from itertools import combinations_with_replacement, permutations, product
from pathlib import Path

import orbifolder
import pytest


def structured(data):
    return {
        "label": data["label"], "lattice": data["lattice"],
        "shifts": data["shifts"], "wilson_lines": data["wilson_lines"],
        "torsion": data["torsion"], "rules": data["rules"],
    }


def compact_cubic(packed):
    count = packed["size"]
    dense = [[[{"num": 0, "den": 1} for _ in range(count)] for _ in range(count)] for _ in range(count)]
    for left, middle, right, value in packed["symmetric_nonzero"]:
        for a, b, c in permutations((left, middle, right)):
            dense[a][b][c] = value
    return dense


def test_compact_matches_all_oracle_field_rows_and_anomaly_basis(results):
    for name, result in results.items():
        value = result.compact_spectrum().to_data()
        reference = result.anomalies()
        for key, expected in reference.items():
            if key not in {"spectrum", "raw", "canonical"}:
                assert value["anomalies"][key] == expected, name
        compact = value["anomalies"]["canonical"]
        assert {**compact, "abelian_cubic": compact_cubic(compact["abelian_cubic"])} == reference["canonical"]
        gauge, expected_gauge = value["gauge_group"], reference["spectrum"]["gauge_group"]
        assert gauge["u1_generators"] == expected_gauge["u1_generators"]
        assert gauge["u1_gram"] == reference["rotation"]["gram"]
        assert gauge["u1_basis"] == "canonical_anomaly"
        assert [factor["id"] for factor in gauge["factors"]] == list(range(len(gauge["factors"])))
        for factor, expected in zip(gauge["factors"], expected_gauge["factors"], strict=True):
            assert factor["simple_roots"] == expected["simple_roots"] and factor["algebra"] == expected["algebra"]
        for sector in ("fields", "tachyons"):
            members = reference["spectrum"][sector]
            all_ids = [field_id for row in value[sector] for field_id in row["field_ids"]]
            assert sorted(all_ids) == list(range(len(members)))
            for row in value[sector]:
                associated = [members[index] for index in row["field_ids"]]
                assert row["field_multiplicity"] == len(associated)
                assert row["state_multiplicity"] == sum(len(field["weight_indices"]) * len(field["right_weights"]) for field in associated)
                reps = [{"algebra": factor["algebra"], **factor["irreps"][rep_id]}
                        for factor, rep_id in zip(gauge["factors"], row["representation_ids"], strict=True)]
                for field in associated:
                    assert reps == field["representations"]
                    assert all(row[key] == field[key] for key in ("u1_charges", "multiplet", "mass"))
        assert value["provenance"] == result.summary()["provenance"] == result.provenance()
        coverage = value["provenance"]["anomaly_checks"]
        triples = list(combinations_with_replacement(range(coverage["u1_count"]), 3))
        for check in reference["checks"]:
            identity, verdict = check["check"], check["result"]
            family = identity["kind"]
            if family == "abelian_universal":
                index = triples.index(tuple(identity["indices"]))
            else:
                index = identity.get("factor", 0)
                index += identity.get("u1", 0) * (coverage["factor_count"] if "factor" in identity else 1)
            expected_status = {"not_applicable": "N", "unsupported": "U"}.get(verdict["status"])
            if expected_status is None:
                expected_status = "P" if verdict["value"] else "F"
            assert coverage[family][index] == expected_status


def test_prepared_geometry_and_batch_options(inputs, results):
    name = "non_susy_z3"
    case = inputs[name]
    data = case["model"]
    geometry = orbifolder.Geometry.from_preset("Z3", theory_name="non_susy")
    parameters = structured(data)
    model = geometry.build_model(**parameters)
    assert model.to_data() == data
    assert orbifolder.Model.from_text(case["model_text"], geometry).to_data() == model.to_data()
    matching = orbifolder.Geometry.from_data(geometry.to_data(), theory_name="non_susy")
    assert orbifolder.parse_legacy_models(case["model_text"], matching)[0].to_data() == data
    assert geometry.build_model(**parameters, theory_name="susy").to_data()["theory"] == "susy"
    profile = geometry.embedding_profile()
    assert profile["shift_orders"] == [2, 3, 1]
    assert len(profile["modular_congruences"]) == 45
    quotient = profile["wilson_quotient"]
    transform = orbifolder.exact_data(quotient["coordinate_transform"])
    basis = orbifolder.exact_data(quotient["basis_columns"])
    assert [[sum(a * b for a, b in zip(row, col)) for col in zip(*basis)] for row in transform] == [
        [int(left == right) for right in range(6)] for left in range(6)]
    assert geometry.validate_embedding(**parameters)["status"] == "verified"
    options = orbifolder.ComputeOptions(anomalies=False, tachyon_search="off")
    batch = orbifolder.Executor(threads=2)
    expected = model.compute(options=options)
    items = [geometry.request(**parameters), model]
    assert [row.value.provenance() for row in batch.compute_many(items, options=options)] == [expected.provenance()] * 2
    compact = expected.compact_spectrum().to_data()
    assert compact["gauge_group"]["u1_basis"] == "rref" and compact["anomalies"]["status"] == "not_requested"
    assert [row["compact"] for row in batch.compute_compact(items, options=options)] == [compact] * 2
    assert [row["summary"] for row in batch.compute_summaries(items, options=options)] == [expected.summary()] * 2
    assert model.compute().summary() == results[name].summary()


def test_tachyon_early_exit_and_provenance_in_mixed_batch(inputs, models, results):
    geometry = orbifolder.Geometry.from_preset("Z3", theory_name="non_susy")
    good_data = structured(inputs["non_susy_z3"]["model"])
    malformed = geometry.request(**{**good_data, "shifts": []})
    invalid_shifts = [row.copy() for row in good_data["shifts"]]
    invalid_shifts[1][0] = "1/7"
    invalid = geometry.request(**{**good_data, "shifts": invalid_shifts})
    tachyonic = models["non_susy_tachyonic"]
    survivor = models["non_susy_z3"]
    options = orbifolder.ComputeOptions(stop_on_tachyon=True, stop_on_anomaly=True)
    items = [survivor, malformed, invalid, tachyonic]
    serial = orbifolder.Executor().compute_summaries(items, options=options)
    parallel = orbifolder.Executor(threads=3).compute_summaries(items, options=options)
    assert serial == parallel
    assert [row["index"] for row in serial] == [0, 1, 2, 3]
    assert [row["status"] for row in serial] == ["ok", "error", "error", "filtered"]
    assert [row["error"]["category"] for row in serial[1:3]] == ["input_error", "physics_invalid"]
    filtered = tachyonic.compute(options=options)
    assert filtered.to_data() == serial[3]["filtered"]
    assert filtered.reason == "tachyon"
    proof = filtered.provenance()
    assert proof["stage"] == "tachyon_projection" and proof["anomaly_status"] == "skipped"
    assert not any(proof[name] for name in ("massless_sectors_solved", "representations_assembled", "susy_view_computed"))
    assert proof["is_tachyon_free"] is False and proof["min_negative_mass_is_observed_only"]
    assert filtered.to_data()["massless_count"] is None and serial[3]["summary"] is None
    assert tachyonic.compute().tachyon_count == results["non_susy_tachyonic"].tachyon_count > 0
    assert survivor.compute(options=options).to_json() == results["non_susy_z3"].to_json()
    assert survivor.compute(tachyon_search="legacy").summary()["is_tachyon_free"] is None
    legacy_hit = tachyonic.compute(tachyon_search="legacy").provenance()
    assert legacy_hit["is_tachyon_free"] is False and not legacy_hit["tachyon_search_complete"]
    assert survivor.compute(tachyon_search="off").provenance()["tachyon_search_mode"] == "none"
    assert results["susy_z3"].provenance()["preserved_supercharge_count"] > 0
    assert results["susy_z3"].provenance()["tachyon_status"] == "excluded_by_theory"
    assert survivor.compute().provenance()["preserved_supercharge_count"] == 0
    budget = orbifolder.Executor().compute_many([survivor], options=orbifolder.ComputeOptions(search_nodes=0))[0]
    assert budget.error["category"] == "resource_limit"
    assert budget.error["provenance"]["is_tachyon_free"] is None
    with pytest.raises(orbifolder.InputError):
        orbifolder.ComputeOptions(tachyon_search="off", stop_on_tachyon=True)


def test_production_prefixes_match_frozen_z3_masks_without_resource_failure():
    fixture = json.loads(Path(__file__).with_name("shift_w2_prefix_masks.json").read_text())
    geo = orbifolder.Geometry.from_data(fixture["geometry"], theory_name="non_susy")
    results = {}
    for row in fixture["cases"]:
        mask = geo.prepare_prefix_mask(embedding=row["fixed"], slots=row["slots"], alphabet=fixture["alphabet"], basis=row["basis"])
        answer = mask.mask(row["local_prefix"])
        assert answer == row["expected"]
        assert mask.states < 100_000
        if row["timeout"]:
            results["timeout"] = answer
        if row["timeout_parent"]:
            results["parent"] = answer
    assert results == {"parent": [True, False, True], "timeout": [False, False, False]}


@pytest.mark.parametrize("lattice,shared_wilson", [("E8xE8", False), ("Spin32", False), ("E8xE8", True)])
def test_exact_masks_against_all_terminals_in_small_rational_domain(inputs, lattice, shared_wilson):
    geo = orbifolder.Geometry.from_preset("Z3xZ3" if shared_wilson else "Z3")
    data = structured(inputs["susy_z3xz3_wl" if shared_wilson else "susy_z3"]["model"])
    data["lattice"] = lattice
    slots = [3, 4] if shared_wilson else [0]
    assert geo.validate_embedding(**data)["status"] == "verified"
    alphabet = [Fraction(-1, 2), Fraction(0), Fraction(1, 3), Fraction(1, 2)]
    axis = [tuple(int(row == col) for col in range(16)) for row in range(16)]
    basis = [tuple(Fraction(1, 3) * value for value in row) for row in axis]
    basis[0] = tuple(Fraction(1, 3) for _ in range(16))
    domains = [alphabet, alphabet[:3], alphabet] + [[0]] * 13
    masker = geo.prepare_prefix_mask(embedding=data, slots=slots, alphabet=alphabet, basis=basis, coordinate_domains=domains)
    valid = []
    for terminal in product(*domains):
        rows = {name: [list(orbifolder.exact_data(row)) for row in data[name]] for name in ("shifts", "wilson_lines")}
        for slot in slots:
            name, index = ("shifts", slot) if slot < 3 else ("wilson_lines", slot - 3)
            rows[name][index] = [center + sum(choice * row[col] for choice, row in zip(terminal, basis))
                                  for col, center in enumerate(rows[name][index])]
        good = geo.validate_embedding(**{**data, **rows})["status"] == "verified"
        assert masker.is_feasible(terminal) == good
        if good:
            valid.append(terminal)
    assert valid and len(valid) < 48
    for length in range(16):
        for prefix in product(*domains[:length]):
            assert masker.mask(prefix) == [any(terminal[:length+1] == (*prefix, action) for terminal in valid) for action in alphabet]
    with pytest.raises(orbifolder.ExecutionError) as error:
        geo.prepare_prefix_mask(embedding=data, slots=slots, alphabet=alphabet, basis=basis, coordinate_domains=domains, max_states=1)
    assert error.value.category == "resource_limit"


def test_direct_coordinates_replace_selected_vector_and_report_preflight_details(inputs):
    geometry = orbifolder.Geometry.from_preset("Z3")
    data = structured(inputs["susy_z3"]["model"])
    original = orbifolder.exact_data(data["shifts"][0])
    data["shifts"] = [list(row) for row in data["shifts"]]
    data["shifts"][0][0] = "1/7"
    error = geometry.validate_embedding(**data)["error"]
    assert error["slots"] == [0] and error["code"] == "embedding_order"
    assert error["scaled_vector"][0] == {"num": 3, "den": 7}
    alphabet = sorted(set(original))
    mask = geometry.prepare_prefix_mask(embedding=data, slots=[0], alphabet=alphabet,
                                        coordinate_domains=[[value] for value in original])
    assert mask.is_feasible(original)


@pytest.mark.parametrize("lattice,width", [("E8xE8", 8), ("Spin32", 16)])
def test_integral_and_spinor_cosets_remain_distinct_in_mask(inputs, lattice, width):
    geometry = orbifolder.Geometry.from_preset("Z3")
    data = {**structured(inputs["susy_z3"]["model"]), "lattice": lattice}
    original = orbifolder.exact_data(data["shifts"][0])
    spinor = [center + Fraction(1, 2) if index < width else center
              for index, center in enumerate(original)]
    domains = [sorted({center, shifted}) for center, shifted in zip(original, spinor, strict=True)]
    alphabet = sorted({value for domain in domains for value in domain})
    mask = geometry.prepare_prefix_mask(embedding=data, slots=[0], alphabet=alphabet, coordinate_domains=domains)
    assert mask.is_feasible(original) and mask.is_feasible(spinor)
    assert not mask.is_feasible([spinor[0], *original[1:]])
