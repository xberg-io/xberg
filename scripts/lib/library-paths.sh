#!/usr/bin/env bash

set -euo pipefail

_get_path_separator() {
	local platform="${1:-$(uname -s)}"
	case "$platform" in
	MINGW* | MSYS* | CYGWIN* | Windows)
		echo ";"
		;;
	*)
		echo ":"
		;;
	esac
}

setup_pdfium_paths() {
	local pdfium_lib="${KREUZBERG_PDFIUM_PREBUILT:-}"
	[ -z "$pdfium_lib" ] && return 0

	local platform="${RUNNER_OS:-$(uname -s)}"
	case "$platform" in
	Linux)
		export LD_LIBRARY_PATH="${pdfium_lib}/lib:${LD_LIBRARY_PATH:-}"
		echo "✓ Set LD_LIBRARY_PATH for PDFium"
		;;
	macOS | Darwin)
		export DYLD_LIBRARY_PATH="${pdfium_lib}/lib:${DYLD_LIBRARY_PATH:-}"
		export DYLD_FALLBACK_LIBRARY_PATH="${pdfium_lib}/lib:${DYLD_FALLBACK_LIBRARY_PATH:-}"
		echo "✓ Set DYLD_LIBRARY_PATH for PDFium on macOS"
		;;
	Windows | MINGW* | MSYS* | CYGWIN*)
		export PATH="${pdfium_lib}/bin;${PATH:-}"
		echo "✓ Set PATH for PDFium on Windows"
		;;
	esac
}

setup_onnx_paths() {
	local ort_lib="${ORT_LIB_LOCATION:-}"
	[ -z "$ort_lib" ] && return 0

	local platform="${RUNNER_OS:-$(uname -s)}"
	case "$platform" in
	Linux)
		export LD_LIBRARY_PATH="${ort_lib}:${LD_LIBRARY_PATH:-}"
		echo "✓ Set LD_LIBRARY_PATH for ONNX Runtime"
		;;
	macOS | Darwin)
		export DYLD_LIBRARY_PATH="${ort_lib}:${DYLD_LIBRARY_PATH:-}"
		export DYLD_FALLBACK_LIBRARY_PATH="${ort_lib}:${DYLD_FALLBACK_LIBRARY_PATH:-}"
		echo "✓ Set DYLD_LIBRARY_PATH for ONNX Runtime on macOS"
		;;
	Windows | MINGW* | MSYS* | CYGWIN*)
		export PATH="${ort_lib};${PATH:-}"
		echo "✓ Set PATH for ONNX Runtime on Windows"
		;;
	esac
}

setup_rust_ffi_paths() {
	local repo_root="${1:-${REPO_ROOT:-}}"
	[ -z "$repo_root" ] && return 0

	local ffi_lib="$repo_root/target/release"
	[ ! -d "$ffi_lib" ] && return 0

	local platform="${RUNNER_OS:-$(uname -s)}"
	case "$platform" in
	Linux)
		export LD_LIBRARY_PATH="${ffi_lib}:${LD_LIBRARY_PATH:-}"
		echo "✓ Set LD_LIBRARY_PATH for Rust FFI"
		;;
	macOS | Darwin)
		export DYLD_LIBRARY_PATH="${ffi_lib}:${DYLD_LIBRARY_PATH:-}"
		export DYLD_FALLBACK_LIBRARY_PATH="${ffi_lib}:${DYLD_FALLBACK_LIBRARY_PATH:-}"
		echo "✓ Set DYLD_LIBRARY_PATH for Rust FFI on macOS"
		;;
	esac
}

verify_pkg_config() {
	if pkg-config --exists kreuzberg-ffi 2>/dev/null; then
		return 0
	else
		{
			echo "Error: pkg-config cannot find kreuzberg-ffi"
			echo "PKG_CONFIG_PATH=${PKG_CONFIG_PATH:-<not set>}"
			echo "Run 'pkg-config --list-all' to see available packages"
		} >&2
		return 1
	fi
}

setup_go_paths_windows() {
	local repo_root="${1:-${REPO_ROOT:-}}"
	[ -z "$repo_root" ] && return 0

	local gnu_target="${repo_root}/target/x86_64-pc-windows-gnu/release"
	local release_target="${repo_root}/target/release"

	export PKG_CONFIG_PATH="${repo_root}/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"

	export PATH="${gnu_target};${release_target};${PATH:-}"

	export CGO_ENABLED=1
	export CGO_CFLAGS="-I${repo_root}/crates/kreuzberg-ffi/include"
	export CGO_LDFLAGS="-L${gnu_target} -L${release_target} -lkreuzberg_ffi -static-libgcc -static-libstdc++"

	echo "✓ Configured Go cgo environment for Windows"
}

# NOTE: CGO_LDFLAGS is set by setup-go-cgo-env action on Windows in CI, or by this script on Unix
setup_go_paths() {
	local repo_root="${1:-${REPO_ROOT:-}}"
	[ -z "$repo_root" ] && return 0

	local pc_path="${repo_root}/crates/kreuzberg-ffi/kreuzberg-ffi.pc"
	if [ ! -f "$pc_path" ]; then
		local version=""
		version="$(sed -n 's/^version = \"\\(.*\\)\"/\\1/p' "${repo_root}/Cargo.toml" | head -n 1 || true)"
		[ -z "$version" ] && version="unknown"

		local platform="${RUNNER_OS:-$(uname -s)}"
		local libs_private=""
		case "$platform" in
		Linux)
			libs_private="-lpthread -ldl -lm"
			;;
		macOS | Darwin)
			libs_private="-framework CoreFoundation -framework Security -lpthread"
			;;
		Windows | MINGW* | MSYS* | CYGWIN*)
			libs_private="-lws2_32 -luserenv -lbcrypt"
			;;
		esac

		mkdir -p "$(dirname "$pc_path")"
		cat >"$pc_path" <<EOF
prefix=${repo_root}
exec_prefix=\${prefix}
libdir=${repo_root}/target/release
includedir=${repo_root}/crates/kreuzberg-ffi

Name: kreuzberg-ffi
Description: C FFI bindings for Kreuzberg document intelligence library
Version: ${version}
URL: https://kreuzberg.dev
Libs: -L\${libdir} -lkreuzberg_ffi
Libs.private: ${libs_private}
Cflags: -I\${includedir}
EOF
	fi

	export PKG_CONFIG_PATH="${repo_root}/crates/kreuzberg-ffi:${PKG_CONFIG_PATH:-}"

	export CGO_ENABLED=1
	export CGO_CFLAGS="-I${repo_root}/crates/kreuzberg-ffi/include"

	local platform="${RUNNER_OS:-$(uname -s)}"
	case "$platform" in
	Linux)
		export LD_LIBRARY_PATH="${repo_root}/target/release:${LD_LIBRARY_PATH:-}"
		export CGO_LDFLAGS="-L${repo_root}/target/release -lkreuzberg_ffi -Wl,-rpath,${repo_root}/target/release"
		;;
	macOS | Darwin)
		export DYLD_LIBRARY_PATH="${repo_root}/target/release:${DYLD_LIBRARY_PATH:-}"
		export DYLD_FALLBACK_LIBRARY_PATH="${repo_root}/target/release:${DYLD_FALLBACK_LIBRARY_PATH:-}"
		export CGO_LDFLAGS="-L${repo_root}/target/release -lkreuzberg_ffi -Wl,-rpath,${repo_root}/target/release"
		;;
	Windows | MINGW* | MSYS* | CYGWIN*)
		if [ -z "${CGO_LDFLAGS:-}" ] && [ -z "${GITHUB_ENV:-}" ]; then
			export CGO_LDFLAGS="-L${repo_root}/target/x86_64-pc-windows-gnu/release -lkreuzberg_ffi -static-libgcc -static-libstdc++ -lws2_32 -luserenv -lbcrypt"
		fi
		;;
	esac

	echo "✓ Configured Go cgo environment"
}

setup_all_library_paths() {
	local repo_root="${1:-${REPO_ROOT:-}}"

	echo "Setting up library paths..."
	setup_pdfium_paths
	setup_onnx_paths
	setup_rust_ffi_paths "$repo_root"
	setup_go_paths "$repo_root"
	echo "✓ All library paths configured"
}

export -f setup_pdfium_paths
export -f setup_onnx_paths
export -f setup_rust_ffi_paths
export -f verify_pkg_config
export -f setup_go_paths_windows
export -f setup_go_paths
export -f setup_all_library_paths
export -f _get_path_separator
