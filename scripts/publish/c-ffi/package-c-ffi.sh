#!/usr/bin/env bash

set -euo pipefail

#   package-c-ffi.sh <platform-label> <version> <output-dir>
#     LICENSE

platform_label="${1:?Platform label required (e.g. linux-x86_64, macos-arm64, windows-x86_64)}"
version="${2:?Version required (e.g. 4.3.6)}"
output_dir="${3:?Output directory required}"

repo_root="$(cd "$(dirname "$0")/../../.." && pwd)"
ffi_crate_dir="${repo_root}/crates/xberg-ffi"

case "$platform_label" in
linux-*)
  shared_lib="libxberg_ffi.so"
  static_lib="libxberg_ffi.a"
  libs_private="-lpthread -ldl -lm"
  ;;
macos-*)
  shared_lib="libxberg_ffi.dylib"
  static_lib="libxberg_ffi.a"
  libs_private="-framework CoreFoundation -framework Security -lpthread"
  ;;
windows-*)
  shared_lib="xberg_ffi.dll"
  static_lib="xberg_ffi.lib"
  libs_private="-lws2_32 -luserenv -lbcrypt"
  ;;
*)
  echo "Error: Unknown platform label '${platform_label}'" >&2
  exit 1
  ;;
esac

BUILD_ARGS=(build -p xberg-ffi --release)
if [ -n "${BUILD_FEATURES:-}" ]; then
  BUILD_ARGS+=(--features "${BUILD_FEATURES}")
fi
echo "Building xberg-ffi (release) ..."
cargo "${BUILD_ARGS[@]}"

if [ -n "${CARGO_BUILD_TARGET:-}" ]; then
  target_release="${repo_root}/target/${CARGO_BUILD_TARGET}/release"
  target_release_fallback="${repo_root}/target/release"
else
  target_release="${repo_root}/target/release"
  target_release_fallback=""
fi

find_lib() {
  local name="$1"
  local alt
  case "$name" in
  xberg_ffi.dll) alt="libxberg_ffi.dll" ;;
  xberg_ffi.lib) alt="libxberg_ffi.a" ;;
  *) alt="" ;;
  esac

  local dir
  for dir in "$target_release" ${target_release_fallback:+"$target_release_fallback"}; do
    if [ -f "${dir}/${name}" ]; then
      echo "${dir}/${name}"
      return
    fi
    if [ -n "$alt" ] && [ -f "${dir}/${alt}" ]; then
      echo "${dir}/${alt}"
      return
    fi
  done

  echo "Error: Cannot find ${name} in ${target_release}${target_release_fallback:+ or ${target_release_fallback}}" >&2
  exit 1
}

shared_lib_path="$(find_lib "$shared_lib")"
static_lib_path="$(find_lib "$static_lib")"

header_path="${ffi_crate_dir}/include/xberg.h"
if [ ! -f "$header_path" ]; then
  echo "Error: Header not found at ${header_path}" >&2
  exit 1
fi

IFS='.' read -r ver_major ver_minor ver_patch <<<"$version"
ver_major="${ver_major:-0}"
ver_minor="${ver_minor:-0}"
ver_patch="${ver_patch:-0}"

dist_name="xberg-ffi-${version}-${platform_label}"
stage_dir="${output_dir}/${dist_name}"

rm -rf "$stage_dir"
mkdir -p "${stage_dir}/include"
mkdir -p "${stage_dir}/lib/pkgconfig"
mkdir -p "${stage_dir}/cmake"

cp "$header_path" "${stage_dir}/include/xberg.h"

cp "$shared_lib_path" "${stage_dir}/lib/"
cp "$static_lib_path" "${stage_dir}/lib/"

if [[ "$platform_label" == windows-* ]]; then
  for dir in "$target_release" ${target_release_fallback:+"$target_release_fallback"}; do
    if [ -f "${dir}/xberg_ffi.dll.lib" ]; then
      cp "${dir}/xberg_ffi.dll.lib" "${stage_dir}/lib/"
      break
    elif [ -f "${dir}/libxberg_ffi.dll.a" ]; then
      cp "${dir}/libxberg_ffi.dll.a" "${stage_dir}/lib/xberg_ffi.dll.lib"
      break
    fi
  done
fi

python3 "${repo_root}/scripts/ci/package-notices.py" stage "$stage_dir"

cat >"${stage_dir}/lib/pkgconfig/xberg.pc" <<EOF
prefix=/usr/local
exec_prefix=\${prefix}
libdir=\${exec_prefix}/lib
includedir=\${prefix}/include

Name: xberg-ffi
Description: C FFI bindings for Xberg document intelligence library
Version: ${version}
URL: https://xberg.io
Libs: -L\${libdir} -lxberg_ffi
Libs.private: ${libs_private}
Cflags: -I\${includedir}
EOF

cp "${ffi_crate_dir}/cmake/xberg-ffi-config.cmake" "${stage_dir}/cmake/xberg-ffi-config.cmake"

cat >"${stage_dir}/cmake/xberg-ffi-config-version.cmake" <<EOF
# xberg-ffi version compatibility check
#
# Reads XBERG_VERSION_MAJOR/MINOR/PATCH from the installed header.

set(PACKAGE_VERSION_MAJOR ${ver_major})
set(PACKAGE_VERSION_MINOR ${ver_minor})
set(PACKAGE_VERSION_PATCH ${ver_patch})
set(PACKAGE_VERSION "\${PACKAGE_VERSION_MAJOR}.\${PACKAGE_VERSION_MINOR}.\${PACKAGE_VERSION_PATCH}")

if(PACKAGE_FIND_VERSION_MAJOR EQUAL PACKAGE_VERSION_MAJOR)
  if(PACKAGE_FIND_VERSION VERSION_LESS_EQUAL PACKAGE_VERSION)
    set(PACKAGE_VERSION_COMPATIBLE TRUE)
    if(PACKAGE_FIND_VERSION VERSION_EQUAL PACKAGE_VERSION)
      set(PACKAGE_VERSION_EXACT TRUE)
    endif()
  else()
    set(PACKAGE_VERSION_COMPATIBLE FALSE)
  endif()
elseif(NOT DEFINED PACKAGE_FIND_VERSION_MAJOR)
  set(PACKAGE_VERSION_COMPATIBLE TRUE)
else()
  set(PACKAGE_VERSION_COMPATIBLE FALSE)
endif()
EOF

mkdir -p "$output_dir"
tar -czf "${output_dir}/c-ffi-${platform_label}.tar.gz" -C "$output_dir" "${dist_name}/"

python3 "${repo_root}/scripts/ci/package-notices.py" verify "${output_dir}/c-ffi-${platform_label}.tar.gz" "$dist_name"

echo "Packaged: ${output_dir}/c-ffi-${platform_label}.tar.gz"
