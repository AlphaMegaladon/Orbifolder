#!/usr/bin/env bash
# Build only the original C++ programs; no probes or fixture generation.
set -euo pipefail
export LC_ALL=C
export CONFIG_SHELL=/bin/bash

variant=${1:?Usage: bash tests/ground_truth/build_legacy.sh susy|non_susy}
case "$variant" in
  susy) executable=orbifolder ;;
  non_susy) executable=nonSUSYorbifolder ;;
  *) printf 'Unknown variant: %s\n' "$variant" >&2; exit 2 ;;
esac

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/../.." && pwd)
original="$repo_dir/legacy_cpp/${variant}_orbifolder"
work_dir="${LEGACY_BUILD_ROOT:-$script_dir/.build}/$variant"
source_dir="$work_dir/source"
build_dir="$work_dir/build"
logs_dir="$work_dir/logs"
include_flags="-I$source_dir/src/io -I$source_dir/src/linalg -I$source_dir/src/groupTheory -I$source_dir/src/orbifolder"
mkdir -p "$logs_dir" "$build_dir"
exec 9> "$work_dir/build.lock"
flock -w 300 9

source_manifest() {
  (
    cd -- "$original"
    find . -type f ! -name '*Zone.Identifier' -print0 |
      sort -z | xargs -0 sha256sum
  )
}

if [[ ! -d "$source_dir" ]]; then
  printf 'Preparing isolated %s source copy.\n' "$variant"
  source_manifest > "$work_dir/original.sha256"
  mkdir "$source_dir"
  tar -C "$original" \
    --exclude='*Zone.Identifier' --exclude='autom4te.cache' \
    --exclude='config.cache' --exclude='config.log' --exclude='config.status' \
    --exclude='Makefile' --exclude='config.h' --exclude='.deps' --exclude='.libs' \
    --exclude='*.o' --exclude='*.a' --exclude='*.lo' --exclude='*.la' \
    --exclude='./orbifolder' --exclude='./nonSUSYorbifolder' \
    --exclude='./src/orbifolder/orbifolder' \
    --exclude='./src/orbifolder/nonSUSYorbifolder' -cf - . |
    tar -C "$source_dir" -xf -
else
  source_manifest > "$work_dir/original-current.sha256"
  cmp "$work_dir/original.sha256" "$work_dir/original-current.sha256"
fi

# Normalize build machinery only; retain the C++ files byte-for-byte.
touch "$work_dir/normalized-build-files.txt"
while IFS= read -r -d '' build_file; do
  if grep -q $'\r$' "$build_file"; then
    printf '%s\n' "${build_file#"$source_dir"/}" >> "$work_dir/normalized-build-files.txt"
    sed -i 's/\r$//' "$build_file"
  fi
done < <(find "$source_dir" -type f \( -name configure -o -name Makefile.in \
  -o -name Makefile.am -o -name '*.m4' -o -name '*.ac' \
  -o -path "$source_dir/Config/*" \) -print0)

# Use the configure/Makefile.in files supplied with the archives. Do not
# trigger maintainer regeneration with an unavailable historical Automake.
# configure itself is executed explicitly on every invocation below.
maintenance_options=(--old-file="$source_dir/aclocal.m4"
  --old-file="$source_dir/configure" --old-file="$source_dir/Config/config.h.in")
while IFS= read -r -d '' template; do
  maintenance_options+=(--old-file="$template")
done < <(find "$source_dir" -name Makefile.in -type f -print0)

{
  date -u '+%Y-%m-%dT%H:%M:%SZ'
  uname -a
  g++ --version
  make --version
  autoconf --version
  automake --version
  libtoolize --version
  ldd --version
  dpkg-query -W g++ make autoconf automake libtool libboost-dev libgsl-dev libreadline-dev
  printf 'CXXFLAGS=-std=c++11 -O2 -g -fno-fast-math\n'
  printf 'CPPFLAGS=%s\n' "$include_flags"
  printf 'CONFIG_SHELL=/bin/bash\nSHELL=/bin/bash\n'
  printf 'Maintainer regeneration excluded:'
  printf ' %q' "${maintenance_options[@]}"
  printf '\n'
} > "$work_dir/environment.txt"

run_logged() {
  local stage=$1
  shift
  printf '%s: %s\n' "$variant" "$stage"
  if "$@" > "$logs_dir/$stage.log" 2>&1; then
    printf '%s: %s succeeded.\n' "$variant" "$stage"
  else
    local status=$?
    tail -n 70 "$logs_dir/$stage.log" >&2
    return "$status"
  fi
}

cd -- "$build_dir"
run_logged configure bash "$source_dir/configure" \
  CXX=g++ 'CXXFLAGS=-std=c++11 -O2 -g -fno-fast-math' \
  "CPPFLAGS=$include_flags" \
  --disable-dependency-tracking
# The root all-am target contains source-relative data wildcards, which do
# not work in a separate build directory. Build the original code targets.
for component in io linalg groupTheory orbifolder; do
  run_logged "make-$component" make -C "src/$component" \
    "${maintenance_options[@]}" -j "${BUILD_JOBS:-2}" \
    SHELL=/bin/bash 'CXXFLAGS=-std=c++11 -O2 -g -fno-fast-math' \
    "CPPFLAGS=$include_flags"
done

binary="$build_dir/src/orbifolder/$executable"
test -x "$binary"
sha256sum "$binary" > "$work_dir/binary.sha256"
cpp_manifest() {
  (
    cd -- "$1"
    find src -type f \( -name '*.cpp' -o -name '*.cc' -o -name '*.h' -o -name '*.hpp' \) \
      -print0 | sort -z | xargs -0 sha256sum
  )
}
cpp_manifest "$original" > "$work_dir/cpp-original.sha256"
cpp_manifest "$source_dir" > "$work_dir/cpp-built.sha256"
cmp "$work_dir/cpp-original.sha256" "$work_dir/cpp-built.sha256"
source_manifest > "$work_dir/original-after.sha256"
cmp "$work_dir/original.sha256" "$work_dir/original-after.sha256"
printf 'Built %s\n' "$binary"
