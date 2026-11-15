#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
build_root=${LEGACY_BUILD_ROOT:-$script_dir/.build}
for variant in "${@:-susy}"; do
  case "$variant" in
    susy) define=() ;;
    non_susy) define=(-DNON_SUSY) ;;
    *) printf 'Unknown variant: %s\n' "$variant" >&2; exit 2 ;;
  esac
  source_dir="$build_root/$variant/source"
  build_dir="$build_root/$variant/build"
  test -f "$build_root/$variant/cpp-built.sha256"
  mapfile -d '' objects < <(find "$build_dir/src/orbifolder" -maxdepth 1 \
    -name '*.o' ! -name main.o -print0 | sort -z)
  ((${#objects[@]} > 0))
  printf 'Linking %s probe with %s existing objects (excluding main.o).\n' "$variant" "${#objects[@]}"
  g++ -std=c++11 -O2 -g -fno-fast-math "${define[@]}" \
    -I"$source_dir/src/orbifolder" -I"$source_dir/src/io" \
    -I"$source_dir/src/linalg" -I"$source_dir/src/groupTheory" \
    "$script_dir/probes/oracle.cpp" "${objects[@]}" \
    "$build_dir/src/groupTheory/libgroupTheory.a" "$build_dir/src/linalg/liblinalg.a" \
    "$build_dir/src/io/libio.a" -lgsl -lgslcblas -lm -lreadline \
    -o "$build_root/$variant/oracle"
  sha256sum "$build_root/$variant/oracle" > "$build_root/$variant/oracle.sha256"
  sha256sum "$script_dir/probes/oracle.cpp" > "$build_root/$variant/oracle-source.sha256"
  sha256sum "$script_dir/build_probes.sh" > "$build_root/$variant/oracle-build-script.sha256"
  sha256sum "${objects[@]}" "$build_dir/src/groupTheory/libgroupTheory.a" \
    "$build_dir/src/linalg/liblinalg.a" "$build_dir/src/io/libio.a" \
    > "$build_root/$variant/oracle-inputs.sha256"
  {
    g++ --version
    printf 'variant=%s\nCXXFLAGS=-std=c++11 -O2 -g -fno-fast-math\n' "$variant"
    printf 'NON_SUSY=%s\n' "${define[*]}"
  } > "$build_root/$variant/oracle-environment.txt"
done
