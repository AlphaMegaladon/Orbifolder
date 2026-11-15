#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
build_root=${LEGACY_BUILD_ROOT:-$script_dir/.build}
source_dir="$build_root/non_susy/source"
build_dir="$build_root/non_susy/build"
mapfile -d '' objects < <(find "$build_dir/src/orbifolder" -maxdepth 1 -name '*.o' ! -name main.o -print0 | sort -z)
g++ -std=c++11 -O2 -g -fno-fast-math \
  -I"$source_dir/src/orbifolder" -I"$source_dir/src/io" \
  -I"$source_dir/src/linalg" -I"$source_dir/src/groupTheory" \
  "$script_dir/probes/states_probe.cpp" "${objects[@]}" \
  "$build_dir/src/groupTheory/libgroupTheory.a" "$build_dir/src/linalg/liblinalg.a" \
  "$build_dir/src/io/libio.a" -lgsl -lgslcblas -lm -lreadline -o "$build_root/non_susy/states-oracle"
