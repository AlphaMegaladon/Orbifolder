"""Run wheel tests from a temporary copy, denying source-tree file access.

Only the installed interpreter environment may remain inside the repository.
The test inputs/fixtures are copied before starting the isolated child process.
"""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main():
    root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="orbifolder-wheel-") as temp:
        work = Path(temp)
        shutil.copytree(root / "tests/python", work / "tests/python", ignore=shutil.ignore_patterns("__pycache__"))
        shutil.copytree(root / "tests/ground_truth/fixtures", work / "tests/ground_truth/fixtures")
        script = work / "run.py"
        script.write_text(
            "import os, pathlib, sys\n"
            f"root = pathlib.Path({str(root)!r}).resolve()\n"
            "prefix = pathlib.Path(sys.prefix).resolve()\n"
            "def audit(event, args):\n"
            "    if event == 'open' and isinstance(args[0], (str, bytes, os.PathLike)):\n"
            "        path = pathlib.Path(os.fsdecode(args[0])).resolve()\n"
            "        if path.is_relative_to(root) and not path.is_relative_to(prefix):\n"
            "            raise RuntimeError(f'Repository access forbidden: {path}')\n"
            "sys.addaudithook(audit)\n"
            "import orbifolder, pytest\n"
            "installed = pathlib.Path(orbifolder.__file__).resolve()\n"
            "assert 'site-packages' in installed.parts, installed\n"
            "print(f'Installed package: {installed}', flush=True)\n"
            "raise SystemExit(pytest.main(['tests/python', '-q', *sys.argv[1:]]))\n"
        )
        env = os.environ.copy()
        env.pop("PYTHONPATH", None)
        env["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
        subprocess.run([sys.executable, "-I", str(script), *sys.argv[1:]], cwd=work, env=env, check=True)


if __name__ == "__main__":
    main()
