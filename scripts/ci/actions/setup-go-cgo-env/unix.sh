#!/usr/bin/env bash
set -euo pipefail

ffi_lib_dir="${1:-target/release}"
enable_rpath="${2:-true}"

repo_root="${GITHUB_WORKSPACE}"
ffi_path="${repo_root}/${ffi_lib_dir}"

if [ ! -d "$ffi_path" ]; then
	echo "Error: FFI library directory not found: $ffi_path" >&2
	exit 1
fi

pkg_config_path="${repo_root}/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"
ld_library_path="${ffi_path}:${LD_LIBRARY_PATH:-}"
dyld_library_path="${ffi_path}:${DYLD_LIBRARY_PATH:-}"
dyld_fallback_library_path="${ffi_path}:${DYLD_FALLBACK_LIBRARY_PATH:-}"

cgo_enabled=1
cgo_cflags="-I${repo_root}/crates/kreuzberg-ffi/include"
cgo_ldflags="-L${ffi_path} -lkreuzberg_ffi"

if [ "$enable_rpath" = "true" ]; then
	cgo_ldflags="${cgo_ldflags} -Wl,-rpath,${ffi_path}"
fi

{
	echo "PKG_CONFIG_PATH=${pkg_config_path}"
	echo "LD_LIBRARY_PATH=${ld_library_path}"
	echo "DYLD_LIBRARY_PATH=${dyld_library_path}"
	echo "DYLD_FALLBACK_LIBRARY_PATH=${dyld_fallback_library_path}"
	echo "CGO_ENABLED=${cgo_enabled}"
	echo "CGO_CFLAGS=${cgo_cflags}"
	echo "# CRITICAL: Replace CGO_LDFLAGS entirely, never append"
	echo "CGO_LDFLAGS=${cgo_ldflags}"
} >>"$GITHUB_ENV"

echo "✓ Go cgo environment configured"
echo "  FFI Library Path: $ffi_path"
echo "  PKG_CONFIG_PATH: $pkg_config_path"
echo "  CGO_LDFLAGS: $cgo_ldflags"
