#!/usr/bin/env bash
set -euo pipefail
task_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$task_root"
export PATH="$HOME/.cargo/bin:$task_root/.venv/bin:/usr/bin:/bin"
export VIRTUAL_ENV="$task_root/.venv"
export PYO3_PYTHON="$task_root/.venv/bin/python"
if [[ ! -x "$PYO3_PYTHON" ]]; then
  printf 'Create .venv and install the development dependencies first; see docs/11_development_ubuntu.md.\n' >&2
  exit 1
fi
# A copied checkout may contain wheels from another machine or platform.
# Install only the wheel produced by this invocation.
wheel_dir=$(mktemp -d "${TMPDIR:-/tmp}/orbifolder-wheel.XXXXXXXX")
trap 'rm -rf -- "$wheel_dir"' EXIT
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
maturin build --release --locked --out "$wheel_dir"
python -m pip install --force-reinstall --no-deps "$wheel_dir"/*.whl
python tests/run_installed_tests.py
mkdir -p dist
cp -- "$wheel_dir"/*.whl dist/
