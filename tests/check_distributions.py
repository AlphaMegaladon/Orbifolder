"""Check release payloads and metadata without importing the native extension."""
from email.parser import BytesParser
from pathlib import Path, PurePosixPath
import sys
import tarfile
import zipfile


def main():
    paths = sorted(Path(sys.argv[1] if len(sys.argv) > 1 else "dist").glob("orbifolder-*"))
    assert paths, "No distributions found"
    for path in paths:
        wheel = path.suffix == ".whl"
        if wheel:
            with zipfile.ZipFile(path) as archive:
                names = archive.namelist()
                metadata = archive.read(next(n for n in names if n.endswith(".dist-info/METADATA")))
        else:
            assert path.name.endswith(".tar.gz"), path
            with tarfile.open(path) as archive:
                names = [name.split("/", 1)[1] for name in archive.getnames() if "/" in name]
                member = next(m for m in archive.getmembers() if len(PurePosixPath(m.name).parts) == 2 and m.name.endswith("/PKG-INFO"))
                metadata = archive.extractfile(member).read()
        for name in names:
            parts = PurePosixPath(name).parts
            assert not any(p in ("legacy_cpp", ".build", "target", "dist", "__pycache__", ".pytest_cache") or p.startswith(".venv") for p in parts), (path, name)
            assert not name.endswith((".pyc", ":Zone.Identifier", ".o", ".a")), (path, name)
            if not wheel:
                assert not name.endswith((".so", ".pyd", ".whl")), (path, name)
        prefix = "orbifolder/" if wheel else "python/orbifolder/"
        for name in ("__init__.py", "__init__.pyi", "_native.pyi", "py.typed", "presets/Z3.json", "presets/Z6-II.json", "presets/Z3xZ3.json"):
            assert prefix + name in names, (path, name)
        if not wheel:
            assert "tests/ground_truth/geometry_inputs.json" in names
            assert len([n for n in names if n.startswith("tests/ground_truth/fixtures/") and n.endswith(".json")]) == 7
        for notice in ("LICENSE", "CITATION.cff", "THIRD_PARTY_NOTICES.md"):
            assert any(n == notice or n.endswith("/" + notice) for n in names), (path, notice)
        msg = BytesParser().parsebytes(metadata)
        assert msg["Name"] == "orbifolder"
        assert msg["Author"] == "Martin Wimmer"
        assert msg["License"] == "GPL-3.0-or-later"
        assert msg["Requires-Python"] == ">=3.11"
        print(f"Validated {path.name}: {len(names)} entries; exact presets/types/notices present, no legacy/build payload")


if __name__ == "__main__":
    main()
