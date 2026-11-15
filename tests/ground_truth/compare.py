"""Validate complete C++ fixtures and compare without numeric tolerances."""
from __future__ import annotations

import argparse
import copy
import json
import math
from fractions import Fraction
from itertools import combinations_with_replacement
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent
SCHEMA = ROOT / "schema.json"


def unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"Duplicate JSON key: {key}")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ValueError(f"Non-finite JSON constant: {value}")


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_object,
                      parse_constant=reject_constant)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def tachyon_source(data: dict[str, Any], field: dict[str, Any]) -> tuple[str, dict[str, Any]]:
    """Resolve the native concatenated state index, never the unreliable enum."""
    si, bi, ti = field["source_index"]
    require(si < len(data["sectors"]), "Dangling tachyon sector")
    sector = data["sectors"][si]
    require(bi < len(sector["fixed_branes"]), "Dangling tachyon brane")
    brane = sector["fixed_branes"][bi]
    ground_count = len(brane["tachyonic_states"])
    key = "tachyonic_states" if ti < ground_count else "excited_tachyonic_states"
    index = ti if ti < ground_count else ti - ground_count
    require(index < len(brane[key]), "Dangling tachyon state")
    return key, brane[key][index]


def tachyon_audit(data: dict[str, Any]) -> list[dict[str, Any]]:
    """Derive R-excitation solely from the referenced original oscillator lists."""
    audit = []
    names = data["conventions"]["multiplets"]
    for field in data["tachyonic_spectrum"]["fields"]:
        container, state = tachyon_source(data, field)
        sector = data["sectors"][field["source_index"][0]]
        index = state["right"]["solution_index"]
        require(index < len(sector["tachyonic_right_solutions"]), "Dangling tachyon solution")
        right = sector["tachyonic_right_solutions"][index]
        oscillators = right["solution"]["excitation"]["oscillators"]
        require(all(i < len(sector["right_oscillators"]) for i in oscillators), "Dangling right oscillator")
        require(bool(oscillators) == (container == "excited_tachyonic_states"),
                "Tachyon container disagrees with actual right oscillator indices")
        masses = [right["masses"][i] for i in state["right"]["weight_indices"]]
        require(bool(masses) and all(m < 0 for m in masses), "Projected tachyon has no negative mass")
        raw = names[field["multiplet"]]
        require(raw in {"Tachyon", "bTachyon", "Excited_Tachyon", "Excited_bTachyon"}, "Unexpected tachyon multiplet")
        normalized = ("Excited_" if oscillators else "") + ("bTachyon" if raw.endswith("bTachyon") else "Tachyon")
        audit.append({"field_index": field["index"], "state_container": container,
                      "right_oscillators": oscillators,
                      "left_oscillators": state["left_solution"]["excitation"]["oscillators"],
                      "masses": masses, "legacy_multiplet": raw,
                      "oscillator_multiplet": normalized, "label_mismatch": raw != normalized})
    return audit


def tachyon_physics_view(data: dict[str, Any]) -> dict[str, Any]:
    """Explicit comparison mode: replace only the known unreliable enum bit."""
    result = copy.deepcopy(data)
    offset = len(result["spectrum"]["fields"]) - len(result["tachyonic_spectrum"]["fields"])
    names = result["conventions"]["multiplets"]
    for item in tachyon_audit(data):
        i = item["field_index"]
        multiplet = names.index(item["oscillator_multiplet"])
        result["tachyonic_spectrum"]["fields"][i]["multiplet"] = multiplet
        result["spectrum"]["fields"][offset + i]["multiplet"] = multiplet
    return result


def validate(data: dict[str, Any]) -> None:
    schema = load(SCHEMA)
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(data),
                    key=lambda error: str(list(error.absolute_path)))
    if errors:
        error = errors[0]
        raise ValueError(f"Schema {list(error.absolute_path)}: {error.message}")

    def numbers(value: Any) -> None:
        if isinstance(value, dict):
            if set(value) == {"num", "den"}:
                require(math.gcd(value["num"], value["den"]) == 1, "Unreduced rational")
            for item in value.values():
                numbers(item)
        elif isinstance(value, list):
            for item in value:
                numbers(item)
        elif isinstance(value, float):
            require(math.isfinite(value), "Non-finite number")

    numbers(data)
    sectors = data["sectors"]
    orders = data["geometry"]["orders"]
    group = data["orbifold_group"]
    gauge = data["gauge_group"]
    elements = group["elements"]
    geometry = data["geometry"]
    metric = geometry["exact_metric"]
    gram = [[Fraction(x["num"], x["den"]) for x in row] for row in metric["gram_matrix"]]
    z6 = metric["basis_interpretation"] == "two_A2_plus_sqrt3_axes"
    require(z6 == (data["input"]["geometry_file"] == "Geometry/Geometry_Z6-II_1_1.txt"),
            "Metric/geometry identity mismatch")
    # Each coordinate is the exact pair (a, b) for a + b*sqrt(3).
    symbolic_basis = [[(0, 0)] * 6 for _ in range(6)]
    for i in range(0, 4 if z6 else 6, 2):
        symbolic_basis[i][i] = (2, 0)
        symbolic_basis[i + 1][i] = (-1, 0)
        symbolic_basis[i + 1][i + 1] = (0, 1)
    if z6:
        symbolic_basis[4][4] = symbolic_basis[5][5] = (0, 1)
    for native, symbolic in zip(geometry["torus_basis"], symbolic_basis):
        require(all(abs(v - (a + b * math.sqrt(3))) <= metric["cartesian_component_abs_error_bound"]
                    for v, (a, b) in zip([v for pair in native for v in pair], symbolic)),
                "Native torus basis disagrees with symbolic interpretation")
    require(all(gram[i][j] == gram[j][i] for i in range(6) for j in range(6)), "Asymmetric Gram matrix")
    # Exact LDL elimination: all positive pivots prove positive definiteness.
    reduced = [row[:] for row in gram]
    for i in range(6):
        require(reduced[i][i] > 0, "Gram matrix is not positive definite")
        for j in range(i + 1, 6):
            for k in range(i + 1, 6):
                reduced[j][k] -= reduced[j][i] * reduced[i][k] / reduced[i][i]
    require(len(geometry["rotation_actions"]) == len(geometry["twist_generators"]), "Missing rotation action")
    for action, generator in zip(geometry["rotation_actions"], geometry["twist_generators"]):
        require(action["generator"] == generator, "Rotation/generator mismatch")
        rotation = [[Fraction(x["num"], x["den"]) for x in row] for row in action["matrix"]]
        require(all(sum(rotation[a][i] * gram[a][b] * rotation[b][j]
                        for a in range(6) for b in range(6)) == gram[i][j]
                    for i in range(6) for j in range(6)), "Rotation does not preserve exact Gram matrix")
    for i in range(6):
        for j in range(6):
            pairs = list(zip(symbolic_basis[i], symbolic_basis[j]))
            require(sum(a * c + 3 * b * d for (a, b), (c, d) in pairs) == gram[i][j]
                    and sum(a * d + b * c for (a, b), (c, d) in pairs) == 0,
                    "Gram matrix/symbolic basis mismatch")
    require(len(elements) == len(group["centralizers"]), "Centralizer count mismatch")
    require(data["spectrum"]["field_count"] > 0, "Empty baseline spectrum")

    def check_half(half: dict[str, Any], solution: dict[str, Any]) -> None:
        require(all(0 <= i < len(solution["weights"]) for i in half["weight_indices"]),
                "Dangling half-state weight index")

    for si, sector in enumerate(sectors):
        require(sector["index"] == si, "Unstable sector index")
        require(all(0 <= a < b for a, b in zip(sector["factors"], orders)), "Sector exponent out of range")
        for half in sector["right_movers"]:
            require(half["solution_index"] < len(sector["right_solutions"]), "Dangling right solution")
            check_half(half, sector["right_solutions"][half["solution_index"]])
        for candidate in sector["tachyonic_right_solutions"]:
            require(len(candidate["masses"]) == len(candidate["solution"]["weights"]), "Tachyon mass/weight mismatch")
        for half in sector["tachyonic_right_movers"]:
            require(half["solution_index"] < len(sector["tachyonic_right_solutions"]), "Dangling tachyonic right solution")
            check_half(half, sector["tachyonic_right_solutions"][half["solution_index"]]["solution"])
        for bi, brane in enumerate(sector["fixed_branes"]):
            require(brane["index"] == bi, "Unstable fixed-brane index")
            require(brane["constructing_index"] < len(elements), "Dangling constructing element")
            require(brane["element"] == elements[brane["constructing_index"]]["element"], "Constructing element mismatch")
            for half in brane["left_movers"]:
                require(half["solution_index"] < len(brane["left_solutions"]), "Dangling left solution")
                check_half(half, brane["left_solutions"][half["solution_index"]])
            for key in ("states", "tachyonic_states", "excited_tachyonic_states"):
                for ti, state in enumerate(brane[key]):
                    check_half(state["left"], state["left_solution"])
                    right = state["right"]["solution_index"]
                    solutions = sector["right_solutions"] if key == "states" else sector["tachyonic_right_solutions"]
                    require(right < len(solutions), "Dangling invariant-state right solution")
                    check_half(state["right"], solutions[right] if key == "states" else solutions[right]["solution"])
                    fields = data["spectrum" if key == "states" else "tachyonic_spectrum"]["fields"]
                    for fi in state["field_indices"]:
                        require(fi < len(fields), "Dangling state field index")
                        native_ti = ti + (len(brane["tachyonic_states"]) if key == "excited_tachyonic_states" else 0)
                        require(fields[fi]["source_index"] == [si, bi, native_ti], "State/field backlink mismatch")

    names = data["conventions"]["multiplets"]
    tachyons = data["tachyonic_spectrum"]["fields"]
    standard = data["spectrum"]["fields"]
    massless_count = len(standard) - len(tachyons)
    require(massless_count >= 0, "Tachyon count exceeds StandardConfig")
    for i, tachyon in enumerate(tachyons):
        appended = {**tachyon, "index": massless_count + i}
        require(standard[massless_count + i] == appended, "StandardConfig tachyon suffix differs from TachyonicStandardConfig")
    for key in ("spectrum", "tachyonic_spectrum"):
        spectrum = data[key]
        require(spectrum["field_count"] == len(spectrum["fields"]), "Field count mismatch")
        for fi, field in enumerate(spectrum["fields"]):
            require(field["index"] == fi, "Unstable field index")
            require(field["multiplet"] < len(names), "Unknown multiplet")
            require(field["q_sh"] is not None or data["variant"] == "non_susy",
                    "Missing SUSY field q_sh")
            require(len(field["u1_charges"]) == len(gauge["u1_generators"]), "U(1) charge dimension mismatch")
            require(len(field["representations"]) == len(gauge["factors"]), "Representation dimension mismatch")
            si, bi, ti = field["source_index"]
            require(si < len(sectors), "Dangling field sector")
            require(bi < len(sectors[si]["fixed_branes"]), "Dangling field brane")
            brane = sectors[si]["fixed_branes"][bi]
            if key == "tachyonic_spectrum" or fi >= massless_count:
                _, state = tachyon_source(data, field)
                native_fi = fi if key == "tachyonic_spectrum" else fi - massless_count
            else:
                require(ti < len(brane["states"]), "Dangling field state")
                state = brane["states"][ti]
                native_fi = fi
            require(native_fi in state["field_indices"], "Field/state backlink mismatch")
            require(set(field["weight_indices"]) <= set(state["left"]["weight_indices"]), "Field weight mismatch")
            require(field["element"] == brane["element"], "Field localization mismatch")
            require(len(field["discrete_non_r_charges"]) == len(data["geometry"]["discrete_symmetries"][0]), "Discrete charge count mismatch")
            require(len(field["discrete_r_charges"]) == len(data["geometry"]["discrete_symmetries"][1]), "R-charge count mismatch")
    audit = tachyon_audit(data)
    if data["case_id"] == "non_susy_tachyonic":
        require(bool(audit), "Positive tachyon fixture has no projected tachyon fields")

    analysis = data["susy_analysis"]
    if data["variant"] == "susy":
        require(analysis["status"] == "computed", "Missing SUSY check")
        pool = analysis["field_pool"]
        require(len(set(pool)) == len(pool) and all(0 <= i < len(data["spectrum"]["fields"]) for i in pool),
                "Invalid SUSY field pool")
        require(len(analysis["trials"]) == math.comb(len(pool) + 2, 3), "Incomplete cubic candidate scope")
        require([tuple(t["field_indices"]) for t in analysis["trials"]] == list(combinations_with_replacement(pool, 3)),
                "Duplicated or missing cubic candidate")
        for trial in analysis["trials"]:
            require(set(trial["field_indices"]) <= set(pool), "Coupling outside declared pool")
            require(all(names[data["spectrum"]["fields"][i]["multiplet"]] == "LeftChiral" for i in trial["field_indices"]), "Non-chiral superpotential input")
    else:
        require(analysis["status"] == "not_applicable", "non-SUSY superpotential status")


def difference(expected: Any, actual: Any, path: str = "$") -> str | None:
    if isinstance(expected, dict) and isinstance(actual, dict):
        if expected.keys() != actual.keys():
            return f"{path}: keys differ ({sorted(expected.keys() ^ actual.keys())})"
        for key in expected:
            found = difference(expected[key], actual[key], f"{path}.{key}")
            if found:
                return found
        return None
    if isinstance(expected, list) and isinstance(actual, list):
        if len(expected) != len(actual):
            return f"{path}: length {len(expected)} != {len(actual)}"
        for index, (left, right) in enumerate(zip(expected, actual)):
            found = difference(left, right, f"{path}[{index}]")
            if found:
                return found
        return None
    if type(expected) is not type(actual) or expected != actual:
        return f"{path}: {expected!r} != {actual!r}"
    return None


def self_test(directory: Path) -> None:
    files = sorted(directory.glob("*.json"))
    expected_cases = {"susy_z3", "susy_z6_ii", "non_susy_z3", "non_susy_z6_ii",
                      "susy_z3xz3_wl", "non_susy_z3xz3_wl", "non_susy_tachyonic"}
    require({p.stem for p in files} == expected_cases, "Expected the six core fixtures and one positive tachyon fixture")
    for path in files:
        data = load(path)
        validate(data)
        require(difference(data, load(path)) is None, "Self-comparison failed")
        require(path.stem == data["case_id"], "Fixture filename/case identity mismatch")
        for mutation in ("section", "denominator", "reference", "count", "metric"):
            broken = copy.deepcopy(data)
            if mutation == "section":
                del broken["anomalies"]
            elif mutation == "denominator":
                broken["geometry"]["rotation_actions"][0]["matrix"][0][0]["den"] = 0
            elif mutation == "reference":
                broken["spectrum"]["fields"][0]["source_index"][0] = len(data["sectors"])
            elif mutation == "count":
                broken["spectrum"]["field_count"] += 1
            else:
                broken["geometry"]["exact_metric"]["gram_matrix"][0][0]["num"] += 1
            try:
                validate(broken)
            except ValueError:
                pass
            else:
                raise ValueError(f"Self-test accepted corruption: {mutation}")
        changed = copy.deepcopy(data)
        changed["sectors"][0]["right_solutions"][0]["weights"][0][0] += 1 / 997
        require(difference(data, changed) is not None, "Quantum-number change was missed")
        audit = tachyon_audit(data)
        if audit:
            changed = copy.deepcopy(data)
            i = audit[0]["field_index"]
            names = data["conventions"]["multiplets"]
            raw = names[changed["tachyonic_spectrum"]["fields"][i]["multiplet"]]
            toggled = raw.removeprefix("Excited_") if raw.startswith("Excited_") else "Excited_" + raw
            offset = len(data["spectrum"]["fields"]) - len(audit)
            for key, index in (("tachyonic_spectrum", i), ("spectrum", offset + i)):
                changed[key]["fields"][index]["multiplet"] = names.index(toggled)
            validate(changed)
            require(difference(data, changed) is not None, "Strict comparator ignored a raw label change")
            require(difference(tachyon_physics_view(data), tachyon_physics_view(changed)) is None,
                    "Physics mode did not isolate the known legacy enum bit")
            state = tachyon_source(changed, changed["tachyonic_spectrum"]["fields"][i])[1]
            si = changed["tachyonic_spectrum"]["fields"][i]["source_index"][0]
            solution = changed["sectors"][si]["tachyonic_right_solutions"][state["right"]["solution_index"]]
            solution["solution"]["excitation"]["oscillators"].append(0)
            try:
                validate(changed)
            except ValueError:
                pass
            else:
                raise ValueError("Self-test accepted a false tachyon oscillator classification")
            print(f"PASS tachyon audit: {len(audit)} projected fields, "
                  f"{sum(x['label_mismatch'] for x in audit)} native label mismatches; enum/oscillator negative tests", flush=True)
        print(f"PASS {path.name}: schema, references, exact metric, exact comparison, six corruption checks")
    for invalid in ('{"x":NaN}', '{"x":1,"x":2}'):
        try:
            json.loads(invalid, object_pairs_hook=unique_object, parse_constant=reject_constant)
        except ValueError:
            pass
        else:
            raise ValueError("Invalid JSON was accepted")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("expected", type=Path, nargs="?")
    parser.add_argument("actual", type=Path, nargs="?")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--tachyon-physics", action="store_true",
                        help="Explicitly compare oscillator-derived tachyon excitation instead of the unreliable legacy enum bit")
    parser.add_argument("--fixtures", type=Path, default=ROOT / "fixtures")
    args = parser.parse_args()
    if args.self_test:
        self_test(args.fixtures)
    elif args.expected and args.actual:
        expected, actual = load(args.expected), load(args.actual)
        validate(expected)
        validate(actual)
        raw_difference = difference(expected, actual)
        found = difference(tachyon_physics_view(expected), tachyon_physics_view(actual)) if args.tachyon_physics else raw_difference
        require(found is None, found or "")
        print("PASS: fixtures match exactly" if not args.tachyon_physics else "PASS: oscillator-derived tachyon physics matches exactly")
        if args.tachyon_physics and raw_difference:
            print(f"Native difference retained in input: {raw_difference}")
    else:
        parser.error("Use --self-test or EXPECTED.json ACTUAL.json")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError) as error:
        raise SystemExit(str(error)) from error
