#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing aarch64 Linux cross toolchain"

echo "Step 1: Updating package lists..."
if ! retry_with_backoff sudo apt-get update; then
	echo "::error::apt-get update failed after retries" >&2
	exit 1
fi

echo "Step 2: Installing cross-compilation packages..."
echo "  - gcc-aarch64-linux-gnu"
echo "  - g++-aarch64-linux-gnu"
echo "  - binutils-aarch64-linux-gnu"
echo "  - pkg-config (host native, used for cross-compilation)"
if ! retry_with_backoff_timeout 900 sudo apt-get install -y \
	gcc-aarch64-linux-gnu \
	g++-aarch64-linux-gnu \
	binutils-aarch64-linux-gnu \
	pkg-config; then
	echo "::error::apt-get install cross toolchain failed" >&2
	exit 1
fi
echo "✓ Packages installed successfully"

echo ""
echo "Step 3: Locating cross-compilation toolchain binaries..."

gcc_bin="$(command -v aarch64-linux-gnu-gcc || command -v aarch64-linux-gnu-gcc-14 || command -v aarch64-linux-gnu-gcc-13 || true)"
gpp_bin="$(command -v aarch64-linux-gnu-g++ || command -v aarch64-linux-gnu-g++-14 || command -v aarch64-linux-gnu-g++-13 || true)"
ar_bin="$(command -v aarch64-linux-gnu-ar || true)"
pkg_config_bin="$(command -v aarch64-linux-gnu-pkg-config || true)"
pkg_config_host_bin="$(command -v pkg-config || true)"

echo "Found binaries:"
echo "  aarch64-linux-gnu-gcc: ${gcc_bin:-[NOT FOUND]}"
echo "  aarch64-linux-gnu-g++: ${gpp_bin:-[NOT FOUND]}"
echo "  aarch64-linux-gnu-ar: ${ar_bin:-[NOT FOUND]}"
echo "  aarch64-linux-gnu-pkg-config: ${pkg_config_bin:-[NOT FOUND - this is optional]}"
echo "  pkg-config (host): ${pkg_config_host_bin:-[NOT FOUND]}"

if [[ -z "$gcc_bin" || -z "$gpp_bin" || -z "$ar_bin" ]]; then
	echo ""
	echo "::error::Cross toolchain binaries not found after install" >&2
	echo "Required binaries missing:" >&2
	[[ -z "$gcc_bin" ]] && echo "  ✗ aarch64-linux-gnu-gcc" >&2
	[[ -z "$gpp_bin" ]] && echo "  ✗ aarch64-linux-gnu-g++" >&2
	[[ -z "$ar_bin" ]] && echo "  ✗ aarch64-linux-gnu-ar" >&2
	echo ""
	echo "Available aarch64-linux-gnu binaries in /usr/bin:" >&2
	ls -la /usr/bin/aarch64-linux-gnu-* 2>/dev/null || echo "  (none found)" >&2
	echo ""
	echo "Available gcc binaries in /usr/bin:" >&2
	found_any=0
	for bin in /usr/bin/*gcc*; do
		if [[ -f "$bin" ]]; then
			if [[ "$bin" =~ aarch64 || "$bin" =~ gcc-[0-9] ]]; then
				ls -la "$bin"
				found_any=1
			fi
		fi
	done >&2
	[[ $found_any -eq 0 ]] && echo "  (none found)" >&2
	exit 1
fi

echo "✓ All required binaries found"

echo ""
echo "Step 4: Creating pkg-config symlink for cross-compilation..."
if [[ -z "$pkg_config_bin" && -n "$pkg_config_host_bin" ]]; then
	echo "  Creating symlink: /usr/bin/aarch64-linux-gnu-pkg-config -> $pkg_config_host_bin"
	sudo ln -sf "$pkg_config_host_bin" /usr/bin/aarch64-linux-gnu-pkg-config
	pkg_config_bin="/usr/bin/aarch64-linux-gnu-pkg-config"
	echo "  ✓ Symlink created successfully"
else
	echo "  ℹ aarch64-linux-gnu-pkg-config already exists at: $pkg_config_bin"
fi

echo ""
echo "Step 5: Configuring environment variables for Rust cross-compilation..."
echo "Setting GITHUB_ENV variables:"
echo "  CC_aarch64_unknown_linux_gnu=$gcc_bin"
echo "  CXX_aarch64_unknown_linux_gnu=$gpp_bin"
echo "  AR_aarch64_unknown_linux_gnu=$ar_bin"
echo "  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=$gcc_bin"
echo "  PKG_CONFIG_ALLOW_CROSS=1"
if [[ -n "$pkg_config_bin" ]]; then
	echo "  PKG_CONFIG_aarch64_unknown_linux_gnu=$pkg_config_bin"
fi

{
	echo "CC_aarch64_unknown_linux_gnu=$gcc_bin"
	echo "CXX_aarch64_unknown_linux_gnu=$gpp_bin"
	echo "AR_aarch64_unknown_linux_gnu=$ar_bin"
	echo "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=$gcc_bin"
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"

{
	echo "PKG_CONFIG_ALLOW_CROSS=1"
	if [[ -n "$pkg_config_bin" ]]; then
		echo "  PKG_CONFIG_aarch64_unknown_linux_gnu=$pkg_config_bin"
		echo "PKG_CONFIG_aarch64_unknown_linux_gnu=$pkg_config_bin"
	elif [[ -n "$pkg_config_host_bin" ]]; then
		echo "  PKG_CONFIG=$pkg_config_host_bin (using host pkg-config for cross-compilation)"
		echo "PKG_CONFIG=$pkg_config_host_bin"
	fi
} >>"${GITHUB_ENV:?GITHUB_ENV not set}"

echo "✓ Environment configured successfully"

echo "::endgroup::"
