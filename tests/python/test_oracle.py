"""C++ comparisons are external to the binding; no expected value enters a solver."""
from collections import defaultdict
from functools import lru_cache
from fractions import Fraction as F
import math
import pytest
from conftest import CASES


@lru_cache(maxsize=None)
def cpp_number(x):
    if isinstance(x, int):
        return F(x)
    assert math.isfinite(x)
    value = F(x).limit_denominator(72)
    budget = 256 * math.ulp(1.0) * (1 + abs(x))
    assert abs(float(value) - x) <= budget
    # Two distinct rationals with denominator <=72 differ by >=1/72^2.
    assert 2 * budget < 1 / 72**2
    return value


def exact(x):
    if isinstance(x, dict) and set(x) == {"num", "den"}:
        return F(x["num"], x["den"])
    if isinstance(x, list):
        return tuple(exact(a) for a in x)
    if isinstance(x, (int, float)) and not isinstance(x, bool):
        return cpp_number(x)
    return x


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def invert(matrix):
    n = len(matrix)
    a = [list(row) + [F(i == j) for j in range(n)] for i, row in enumerate(matrix)]
    for j in range(n):
        pivot = next(i for i in range(j, n) if a[i][j])
        a[j], a[pivot] = a[pivot], a[j]
        scale = a[j][j]
        a[j] = [v / scale for v in a[j]]
        for i in range(n):
            if i != j:
                scale = a[i][j]
                a[i] = [x - scale * y for x, y in zip(a[i], a[j])]
    return [r[n:] for r in a]


def transform(old, new):
    inverse = invert([[dot(a, b) for b in old] for a in old])
    return [[sum(dot(n, old[k]) * inverse[k][j] for k in range(len(old))) for j in range(len(old))] for n in new]


def alignment(snapshot, fixture):
    factors = snapshot["spectrum"]["gauge_group"]["factors"]
    result = []
    for native in fixture["gauge_group"]["factors"]:
        roots = exact(native["simple_roots"])
        index = next(i for i, f in enumerate(factors) if set(exact(f["simple_roots"])) == set(roots))
        factor = factors[index]
        assert factor["algebra"] == native["algebra"]
        permutation = [exact(factor["simple_roots"]).index(r) for r in roots]
        result.append((index, permutation))
    return result


def code(field):
    m = field["multiplet"]
    if m["family"] == "tachyon":
        return 21 + int(m["conjugate"]) + 2 * int(m["excited"])
    if m["family"] == "susy":
        return {"LeftChiral": 2, "RightChiral": 3, "Vector": 4, "ConjugateVector": 5, "HalfHyper": 7, "Gravity": 8, "ConjugateGravity": 9, "LeftModulus": 10, "RightModulus": 11}[m["kind"]]
    return {"Gauge": 13, "ConjugateGauge": 14, "Scalar": 15, "ConjugateScalar": 16, "LeftFermion": 19, "RightFermion": 20, "Gravity": 8, "ConjugateGravity": 9, "Modulus": 17, "ConjugateModulus": 18}[m["kind"]]


def compare_fields(snapshot, fixture):
    spectrum = snapshot["spectrum"]
    actual = spectrum["fields"] + spectrum["tachyons"]
    expected = fixture["spectrum"]["fields"]
    assert len(actual) == len(expected)
    assert len(spectrum["tachyons"]) == len(fixture["tachyonic_spectrum"]["fields"])
    factors = alignment(snapshot, fixture)
    generators = exact(fixture["gauge_group"]["u1_generators"])
    matched = defaultdict(list)
    for field in actual:
        matched[(tuple(field["source"]), tuple(field["weight_indices"]), code(field))].append(field)
    mapping = {}
    for native in expected:
        si, bi, state_id = native["source_index"]
        brane = fixture["sectors"][si]["fixed_branes"][bi]
        tachyon = native["multiplet"] >= 21
        field_index = native["index"] - (len(spectrum["fields"]) if tachyon else 0)
        state = (brane["tachyonic_states"] + brane["excited_tachyonic_states"] if tachyon else brane["states"])[state_id]
        position = state["field_indices"].index(field_index)
        right = exact(state["q_charges"][position % len(state["q_charges"])])
        native_code = native["multiplet"]
        if tachyon:
            # Only the accepted Excited enum defect is normalized; oscillators are exact.
            projected = snapshot["sectors"][si]["branes"][bi]["tachyons"][state_id]
            excited = bool(snapshot["sectors"][si]["tachyon_shells"][projected["right"]["shell"]]["excitation"]["oscillators"])
            native_code = 21 + (native_code - 21) % 2 + 2 * int(excited)
        candidates = matched[(tuple(native["source_index"]), tuple(native["weight_indices"]), native_code)]
        field = next(f for f in candidates if exact(f["right_weights"]) == right)
        candidates.remove(field)
        assert exact(field["element"]["translation"]) == exact(native["element"]["translation"])
        assert field["element"]["factors"] == native["element"]["factors"]
        assert exact(field["gamma"]) == exact(native["gamma_phases"])
        assert exact(field["oscillator_contribution"]) == exact(native["oscillator_contribution"])
        for ni, (ri, permutation) in enumerate(factors):
            rep = field["representations"][ri]
            assert rep["dimension"] == abs(native["representations"][ni]["dimension"])
            assert [rep["dynkin"][i] for i in permutation] == native["highest_weights"][ni]
        assert tuple(dot(g, exact(field["highest_weight"])) for g in generators) == exact(native["u1_charges"])
        if native["q_sh"] is not None:
            assert exact(field["scalar_charge"]) == exact(native["q_sh"])
        mapping[native["index"]] = field["id"]
    assert not any(matched.values())
    return mapping


@pytest.fixture(scope="session")
def field_maps(snapshots, fixtures):
    return {case: compare_fields(snapshots[case], fixtures[case]) for case in CASES}


def compare_shells(actual, expected):
    assert len(actual) == len(expected)
    for a, e in zip(actual, expected):
        assert exact(a["weights"]) == exact(e["weights"])
        assert a["excitation"]["oscillators"] == e["excitation"]["oscillators"]
        assert exact(a["excitation"]["energy"]) == exact(e["excitation"]["number_operator"])
        assert exact(a["excitation"]["target"]) == exact(e["excitation"]["squared_weight_target"])


def compare_half(a, e):
    assert a["shell"] == e["solution_index"]
    assert a["weight_indices"] == e["weight_indices"]
    phases = exact(e["eigenphases"])
    assert exact(a["phases"][:len(phases)]) == phases
    assert all(exact(x) == 0 for x in a["phases"][len(phases):])


@pytest.mark.parametrize("case", CASES)
def test_geometry_states_and_all_fields(case, snapshots, fixtures, field_maps):
    a, e = snapshots[case], fixtures[case]
    assert a["geometry"]["orders"] == e["geometry"]["orders"]
    assert exact(a["geometry"]["gram"]) == exact(e["geometry"]["exact_metric"]["gram_matrix"])
    assert exact(a["orbifold_group"]["supercharges"]) == exact(e["orbifold_group"]["invariant_supercharges"])
    for x, y in zip(a["orbifold_group"]["elements"], e["orbifold_group"]["elements"]):
        assert exact(x["shift"]) == exact(y["shift"]) and exact(x["twist"]) == exact(y["twist"])
    assert len(a["orbifold_group"]["elements"]) == len(e["orbifold_group"]["elements"])
    assert len(a["sectors"]) == len(e["sectors"])
    for s, sector in zip(a["sectors"], e["sectors"]):
        assert s["factors"] == sector["factors"]
        for key in ("twist", "a_left", "a_right"):
            assert exact(s[key]) == exact(sector[key])
        for key, native in (("left_modes", "left_oscillators"), ("right_modes", "right_oscillators")):
            assert len(s[key]) == len(sector[native])
            for mode, oscillator in zip(s[key], sector[native]):
                assert mode["direction"] == oscillator["direction"] and mode["conjugate"] == oscillator["complex_conjugate"]
                assert exact(mode["frequency"]) == exact(oscillator["frequency"])
        compare_shells(s["right_shells"], sector["right_solutions"])
        assert len(s["right_movers"]) == len(sector["right_movers"])
        for h, n in zip(s["right_movers"], sector["right_movers"]):
            compare_half(h["state"], n)
        assert len(s["branes"]) == len(sector["fixed_branes"])
        for b, brane in zip(s["branes"], sector["fixed_branes"]):
            assert b["element"]["factors"] == brane["element"]["factors"]
            assert exact(b["element"]["translation"]) == exact(brane["element"]["translation"])
            compare_shells(b["left_shells"], brane["left_solutions"])
            assert len(b["left_movers"]) == len(brane["left_movers"])
            for h, n in zip(b["left_movers"], brane["left_movers"]):
                compare_half(h["state"], n)
            for key, native in (("states", brane["states"]), ("tachyons", brane["tachyonic_states"] + brane["excited_tachyonic_states"])):
                assert len(b[key]) == len(native)
                for state, n in zip(b[key], native):
                    compare_half(state["left"], n["left"])
                    compare_half(state["right"], n["right"])
                    assert exact(state["gamma"]) == exact(n["gamma_phases"])
                    assert exact(state["oscillator_contribution"]) == exact(n["oscillator_contribution"])
    assert len(field_maps[case]) == len(e["spectrum"]["fields"])


@pytest.mark.parametrize("case", CASES)
def test_all_anomaly_coefficients_and_fi(case, snapshots, fixtures):
    a, e = snapshots[case], fixtures[case]
    report = a["anomalies"]
    assert report["is_anomaly_free"] == e["anomalies"]["check_passed"]
    assert exact(report["fi"]["legacy_d0"]) == exact(e["anomalies"]["gauge_after_check"]["fi_term"])
    old = exact(a["spectrum"]["gauge_group"]["u1_generators"])
    new = exact(e["anomalies"]["gauge_after_check"]["u1_generators"])
    b = transform(old, new)
    raw = report["raw"]
    factors = alignment(a, e)
    occurrences = defaultdict(int)
    def qi(label):
        return int(label.removeprefix("Q_").split("^")[0]) - 1
    def cubic(x, y, z):
        tensor = exact(raw["abelian_cubic"])
        return sum(x[i]*y[j]*z[k]*tensor[i][j][k] for i in range(len(x)) for j in range(len(y)) for k in range(len(z)))
    for coefficient in e["anomalies"]["coefficients"]:
        label, expected = coefficient["label"], exact(coefficient["value"])
        if "-th gauge group factor" in label:
            ni = int(label.split("-")[0]) - 1
            g = factors[ni][0]
            actual = raw["su2_doublets"][g] if "doublets" in label else exact(raw["gauge_cubic"][g])
        elif label.startswith("tr l(rep. of "):
            algebra = label.split("of ")[1].split(")")[0]
            u = qi(label.split()[-1])
            matches = [i for i, f in enumerate(e["gauge_group"]["factors"]) if f["algebra"] == algebra]
            g = factors[matches[occurrences[(algebra, u)]]][0]
            occurrences[(algebra, u)] += 1
            actual = 2 * sum(b[u][j] * exact(raw["mixed"][j][g]) for j in range(len(old)))
        else:
            words = label.split()
            x = b[qi(words[1])]
            if len(words) == 3:
                y = b[qi(words[2])]
                actual = cubic(x, y, y)
            elif "^" in words[1]:
                actual = cubic(x, x, x)
            else:
                actual = dot(x, exact(raw["gravitational"]))
        assert actual == expected, (case, label, actual, expected)
    if report["rotation"]["anomalous_index"] is not None:
        assert exact(report["rotation"]["generators"][0]) == new[0]


@pytest.mark.parametrize("case", CASES)
def test_susy_couplings_and_not_applicable(case, results, fixtures, field_maps):
    e = fixtures[case]
    if case.startswith("non_"):
        assert results[case].couplings([])["status"] == "not_applicable"
        assert results[case].superpotential([])["status"] == "not_applicable"
        return
    trials = e["susy_analysis"]["trials"]
    ids = [[field_maps[case][i] for i in t["field_indices"]] for t in trials]
    analysis = results[case].couplings(ids)
    assert analysis["status"] == "computed"
    for a, n in zip(analysis["trials"], trials):
        assert a["native_allowed"] == bool(n["allowed_couplings"])
        assert a["native_gauge_invariant"] == n["gauge_invariant"]
        assert dict(a["fields"]) == {field_maps[case][fields[0]]: power for fields, power in zip(n["monomial_fields"], n["monomial_exponents"])}
    pool = [field_maps[case][i] for i in e["susy_analysis"]["field_pool"]]
    assert results[case].superpotential(pool)["status"] == "computed"
