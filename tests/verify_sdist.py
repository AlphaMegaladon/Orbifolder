"""Run the entire Rust suite in a freshly extracted sdist without source caches."""
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile


def main():
    archive_path = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="orbifolder-source-") as temp:
        with tarfile.open(archive_path) as archive:
            archive.extractall(temp, filter="data")
        source, = Path(temp).iterdir()
        assert (source / "Cargo.toml").is_file()
        assert not (source / "legacy_cpp").exists()
        env = os.environ.copy()
        # A fresh target is essential: cached test binaries can embed an old
        # CARGO_MANIFEST_DIR and accidentally read fixtures from the checkout.
        env["CARGO_TARGET_DIR"] = str(source / "target")
        print(f"Fresh Rust build/tests from {source} (no legacy directory)", flush=True)
        subprocess.run(["cargo", "test", "--workspace", "--locked"], cwd=source, env=env, check=True)


if __name__ == "__main__":
    main()
