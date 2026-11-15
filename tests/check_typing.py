"""Validate the installed native API and prove that its stubs reject bad calls."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    root = Path(__file__).resolve().parents[1]
    env = os.environ.copy()
    env["PATH"] = str(Path(sys.executable).parent) + os.pathsep + env.get("PATH", "")
    def run(*args, cwd=root):
        return subprocess.run([sys.executable, "-m", *args], cwd=cwd, env=env, text=True, capture_output=True)
    commands = [
        ("mypy", "--strict", "--python-version", "3.11", "tests/typing/api_usage.py"),
        ("mypy.stubtest", "orbifolder"),
        ("pyright", "--pythonpath", sys.executable, "tests/typing/api_usage.py"),
    ]
    for args in commands:
        result = run(*args)
        print(result.stdout + result.stderr, end="")
        result.check_returncode()
    with tempfile.TemporaryDirectory(prefix="orbifolder-types-") as temp:
        invalid = Path(temp) / "invalid_usage.py"
        invalid.write_text((root / "tests/typing/invalid_usage.txt").read_text())
        mypy = run("mypy", "--strict", "--python-version", "3.11", str(invalid), cwd=temp)
        assert mypy.returncode == 1 and mypy.stdout.count(": error:") == 3, mypy.stdout + mypy.stderr
        pyright = run("pyright", "--pythonpath", sys.executable, "--pythonversion", "3.11", "--outputjson", str(invalid), cwd=temp)
        assert pyright.returncode == 1 and json.loads(pyright.stdout)["summary"]["errorCount"] == 3, pyright.stdout + pyright.stderr
        print("Both checkers reject all three invalid API calls.")
        verified = run("pyright", "--verifytypes", "orbifolder", "--outputjson", cwd=temp)
        report = json.loads(verified.stdout)
        assert not report["generalDiagnostics"], report["generalDiagnostics"]
        info = report["typeCompleteness"]
        unknown = [s["name"] for s in info["symbols"] if s["isExported"] and not s["isTypeKnown"]]
        # Pyright's bundled stdlib has incomplete numbers/Fraction declarations.
        # Preserve the real Fraction identity and report this external boundary.
        assert set(unknown) <= {"orbifolder.Fraction", "orbifolder.exact_data"}, unknown
        print(f"Unfiltered VerifyTypes: {info['completenessScore']:.1%}; external-boundary symbols: {unknown}")


if __name__ == "__main__":
    main()
