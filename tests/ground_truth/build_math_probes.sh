#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
build_root=${LEGACY_BUILD_ROOT:-$script_dir/.build}
for variant in susy non_susy; do
  source_dir="$build_root/$variant/source"
  build_dir="$build_root/$variant/build"
  define=()
  if [[ "$variant" == non_susy ]]; then define=(-DNON_SUSY); fi
  mapfile -d '' objects < <(find "$build_dir/src/orbifolder" -maxdepth 1 -name '*.o' ! -name main.o -print0 | sort -z)
  printf 'Linking %s mathematical probe.\n' "$variant"
  g++ -std=c++11 -O2 -g -fno-fast-math "${define[@]}" \
    -I"$source_dir/src/orbifolder" -I"$source_dir/src/io" \
    -I"$source_dir/src/linalg" -I"$source_dir/src/groupTheory" \
    "$script_dir/probes/math_probe.cpp" "${objects[@]}" \
    "$build_dir/src/groupTheory/libgroupTheory.a" "$build_dir/src/linalg/liblinalg.a" \
    "$build_dir/src/io/libio.a" -lgsl -lgslcblas -lm -lreadline -o "$build_root/$variant/math-oracle"
done
