#!/usr/bin/env bash
# Exercise the original CLI and one archived model, without JSON probes.
set -euo pipefail
export LC_ALL=C

variant=${1:?Usage: bash tests/ground_truth/smoke_legacy.sh susy|non_susy}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
work_dir="${LEGACY_BUILD_ROOT:-$script_dir/.build}/$variant"
source_dir="$work_dir/source"
case "$variant" in
  susy)
    executable=orbifolder
    banner='The C++ Orbifolder'
    model_label=MSSM0
    awk '{print} /^end model/{exit}' \
      "$source_dir/Z3xZ3_1_1-1WL-77PatricksModels.txt" > "$work_dir/smoke-model.txt"
    ;;
  non_susy)
    executable=nonSUSYorbifolder
    banner='Non-SUSY Orbifolder'
    model_label=Z3_1_1
    cp -- "$source_dir/Models/ZN_models/modelZ3_1_1.txt" "$work_dir/smoke-model.txt"
    ;;
  *) printf 'Unknown variant: %s\n' "$variant" >&2; exit 2 ;;
esac

binary="$work_dir/build/src/orbifolder/$executable"
test -x "$binary"
ldd "$binary" > "$work_dir/logs/ldd.log"
if grep -q 'not found' "$work_dir/logs/ldd.log"; then
  cat "$work_dir/logs/ldd.log" >&2
  exit 1
fi

printf 'help\nexit orbifolder\n' > "$work_dir/smoke-cli.stdin"
(
  cd -- "$source_dir"
  timeout --kill-after=5s 30s "$binary" < "$work_dir/smoke-cli.stdin"
) > "$work_dir/logs/smoke-cli.log" 2>&1
grep -Fq "$banner" "$work_dir/logs/smoke-cli.log"
grep -Fq 'exit program' "$work_dir/logs/smoke-cli.log"
printf '%s: CLI help and clean exit passed.\n' "$variant"

printf 'exit orbifolder\n' > "$work_dir/smoke-model.stdin"
(
  cd -- "$source_dir"
  timeout --kill-after=5s 120s "$binary" "$work_dir/smoke-model.txt" \
    < "$work_dir/smoke-model.stdin"
) > "$work_dir/logs/smoke-model.log" 2>&1
grep -Fq "Orbifold \"$model_label\" loaded." "$work_dir/logs/smoke-model.log"
printf '%s: archived model %s loaded; clean exit passed.\n' "$variant" "$model_label"
