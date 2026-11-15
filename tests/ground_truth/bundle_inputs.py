"""Rebuild distributable geometry inputs; never modify the seven oracle fixtures.

Run only when deliberately refreshing the input bundle. Requires local archives.
The model inputs contain no expected spectra or calculated oracle answers.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    inputs = json.loads((ROOT / "tests/python/model_inputs.json").read_text())
    geometries = {}
    presets = ROOT / "python/orbifolder/presets"
    presets.mkdir(exist_ok=True)
    for name, suffix in (("Z3", "z3"), ("Z6-II", "z6_ii"), ("Z3xZ3", "z3xz3_wl")):
        bundle = {"name": name, "schema_version": 1}
        for variant in ("susy", "non_susy"):
            entry = inputs[f"{variant}_{suffix}"]
            model = entry["model"]
            relative = next(line.removeprefix("SpaceGroup:") for line in entry["model_text"].splitlines() if line.startswith("SpaceGroup:"))
            raw = (ROOT / f"legacy_cpp/{variant}_orbifolder" / relative).read_bytes()
            digest = hashlib.sha256(raw).hexdigest()
            assert digest == entry["geometry_sha256"]
            geometries[f"{variant}/{relative}"] = {"sha256": digest, "text": raw.decode()}
            bundle[variant] = {"geometry": model["geometry"], "rules": model["rules"], "geometry_sha256": digest}
        (presets / f"{name}.json").write_text(json.dumps(bundle, sort_keys=True, separators=(",", ":")) + "\n")
    (ROOT / "tests/ground_truth/geometry_inputs.json").write_text(json.dumps(geometries, sort_keys=True, indent=2) + "\n")


if __name__ == "__main__":
    main()
