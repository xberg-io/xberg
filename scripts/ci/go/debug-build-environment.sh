#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_section() {
	echo ""
	echo -e "${BLUE}========================================${NC}"
	echo -e "${BLUE}$1${NC}"
	echo -e "${BLUE}========================================${NC}"
	echo ""
}

print_status() {
	local status="$1"
	local message="$2"
	case "$status" in
	OK)
		echo -e "${GREEN}[OK]${NC} $message"
		;;
	WARN)
		echo -e "${YELLOW}[WARN]${NC} $message"
		;;
	FAIL)
		echo -e "${RED}[FAIL]${NC} $message"
		;;
	INFO)
		echo -e "${BLUE}[INFO]${NC} $message"
		;;
	esac
}

print_section "Go CI Build Environment Diagnostics"

print_section "System Information"
print_status INFO "OS: $(uname -s)"
print_status INFO "Architecture: $(uname -m)"
print_status INFO "Kernel: $(uname -r)"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	print_status INFO "Running on Windows (OSTYPE=$OSTYPE)"
	print_status INFO "Windows Version:"
	cmd /c ver 2>/dev/null || echo "Could not determine Windows version"
else
	print_status INFO "Running on Unix-like system"
fi
echo ""

print_section "Tool Versions"
print_status INFO "Rust:"
rustc --version || print_status WARN "rustc not found"
cargo --version || print_status WARN "cargo not found"
print_status INFO "Go:"
go version || print_status WARN "go not found"
print_status INFO "C Compiler:"
gcc --version 2>&1 | head -1 || print_status WARN "gcc not found"
print_status INFO "pkg-config:"
pkg-config --version 2>&1 || print_status WARN "pkg-config not found"
echo ""

print_section "Cargo Configuration"
print_status INFO "Cargo.toml location: $REPO_ROOT/Cargo.toml"
if [ -f "$REPO_ROOT/Cargo.toml" ]; then
	print_status OK "Cargo.toml exists"
	print_status INFO "Workspace members:"
	grep '^\[workspace\]' -A 20 "$REPO_ROOT/Cargo.toml" | grep 'members' -A 5 | sed 's/^/  /'
else
	print_status FAIL "Cargo.toml not found"
fi
echo ""

print_section "Build Targets and Output Directories"
print_status INFO "Default target:"
rustc --version --verbose | grep host || print_status WARN "Could not determine default target"

print_status INFO "Build directories:"
if [ -d "$REPO_ROOT/target/release" ]; then
	print_status OK "target/release exists"
	echo "    Contents:"
	if find "$REPO_ROOT/target/release" -maxdepth 1 -type f \( -name "*libkreuzberg_ffi*" -o -name "*.pc" \) -exec ls -lh {} \; | sed 's/^/      /'; then
		:
	else
		print_status WARN "No FFI library artifacts found"
	fi
else
	print_status WARN "target/release does not exist"
fi

if [ -d "$REPO_ROOT/target/x86_64-pc-windows-gnu/release" ]; then
	print_status OK "target/x86_64-pc-windows-gnu/release exists (Windows MinGW)"
	echo "    Contents:"
	if find "$REPO_ROOT/target/x86_64-pc-windows-gnu/release" -maxdepth 1 -type f -name "*libkreuzberg_ffi*" -exec ls -lh {} \; | sed 's/^/      /'; then
		:
	else
		print_status WARN "No FFI library artifacts found"
	fi
else
	print_status INFO "target/x86_64-pc-windows-gnu/release does not exist (expected on non-Windows)"
fi
echo ""

print_section "Environment Variables - Rust Build"
print_status INFO "RUSTFLAGS: ${RUSTFLAGS:-<not set>}"
print_status INFO "RUSTC_WRAPPER: ${RUSTC_WRAPPER:-<not set>}"
print_status INFO "CARGO_BUILD_TARGET: ${CARGO_BUILD_TARGET:-<not set>}"
print_status INFO "CARGO_TARGET_DIR: ${CARGO_TARGET_DIR:-<not set>}"
print_status INFO "CARGO_INCREMENTAL: ${CARGO_INCREMENTAL:-<not set>}"
echo ""

print_section "Environment Variables - CGO/Go"
print_status INFO "CGO_ENABLED: ${CGO_ENABLED:-<not set>}"
print_status INFO "CGO_CFLAGS: ${CGO_CFLAGS:-<not set>}"
print_status INFO "CGO_CXXFLAGS: ${CGO_CXXFLAGS:-<not set>}"
print_status INFO "CGO_LDFLAGS: ${CGO_LDFLAGS:-<not set>}"
print_status INFO "PKG_CONFIG_PATH: ${PKG_CONFIG_PATH:-<not set>}"
print_status INFO "GOFLAGS: ${GOFLAGS:-<not set>}"
print_status INFO "GOARCH: ${GOARCH:-<not set>}"
print_status INFO "GOOS: ${GOOS:-<not set>}"
echo ""

print_section "Environment Variables - Library Paths"
print_status INFO "LD_LIBRARY_PATH: ${LD_LIBRARY_PATH:-<not set>}"
print_status INFO "DYLD_LIBRARY_PATH: ${DYLD_LIBRARY_PATH:-<not set>}"
print_status INFO "DYLD_FALLBACK_LIBRARY_PATH: ${DYLD_FALLBACK_LIBRARY_PATH:-<not set>}"
print_status INFO "PATH (first 500 chars): ${PATH:0:500}..."
echo ""

if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
	print_section "Windows Build Environment"
	print_status INFO "MSYS2 Installation:"
	if [ -d "C:\\msys64" ]; then
		print_status OK "MSYS2 found at C:\\msys64"
		echo "    MinGW64 bin:"
		if find "C:\\msys64\\mingw64\\bin" -maxdepth 1 -type f 2>/dev/null | head -5 | xargs -r ls -lh; then
			:
		else
			print_status WARN "Could not list MSYS2 bins"
		fi
	else
		print_status WARN "MSYS2 not found at standard location"
	fi

	print_status INFO "Windows Toolchain:"
	print_status INFO "CC: ${CC:-<not set>}"
	print_status INFO "CXX: ${CXX:-<not set>}"
	print_status INFO "AR: ${AR:-<not set>}"
	print_status INFO "RANLIB: ${RANLIB:-<not set>}"
	print_status INFO "TARGET_CC: ${TARGET_CC:-<not set>}"
	print_status INFO "TARGET_AR: ${TARGET_AR:-<not set>}"
	print_status INFO "CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER: ${CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER:-<not set>}"

	echo ""
	print_status INFO "Verifying toolchain:"
	if which gcc >/dev/null 2>&1; then
		print_status OK "gcc found"
	else
		print_status WARN "gcc not in PATH"
	fi
	if which ar >/dev/null 2>&1; then
		print_status OK "ar found"
	else
		print_status WARN "ar not in PATH"
	fi
	if which g++ >/dev/null 2>&1; then
		print_status OK "g++ found"
	else
		print_status WARN "g++ not in PATH"
	fi
fi
echo ""

print_section "FFI Library Configuration"
print_status INFO "FFI source directory: $REPO_ROOT/crates/kreuzberg-ffi"
if [ -d "$REPO_ROOT/crates/kreuzberg-ffi" ]; then
	print_status OK "FFI directory exists"
	print_status INFO "FFI Cargo.toml:"
	grep '^name\|^version' "$REPO_ROOT/crates/kreuzberg-ffi/Cargo.toml" | head -2 | sed 's/^/  /'
fi

print_status INFO "FFI header file: $REPO_ROOT/crates/kreuzberg-ffi/kreuzberg.h"
if [ -f "$REPO_ROOT/crates/kreuzberg-ffi/kreuzberg.h" ]; then
	print_status OK "FFI header exists ($(wc -l <"$REPO_ROOT/crates/kreuzberg-ffi/kreuzberg.h") lines)"
else
	print_status FAIL "FFI header not found"
fi

print_status INFO "pkg-config file: $REPO_ROOT/crates/kreuzberg-ffi/kreuzberg-ffi.pc"
if [ -f "$REPO_ROOT/crates/kreuzberg-ffi/kreuzberg-ffi.pc" ]; then
	print_status OK "pkg-config file exists"
	echo "    Contents:"
	sed 's/^/    /' "$REPO_ROOT/crates/kreuzberg-ffi/kreuzberg-ffi.pc"
else
	print_status WARN "pkg-config file not found (will be generated during Rust build)"
fi
echo ""

print_section "Go Module Configuration"
print_status INFO "Go module location: $REPO_ROOT/packages/go/v4"
if [ -f "$REPO_ROOT/packages/go/v4/go.mod" ]; then
	print_status OK "go.mod exists"
	echo "    Contents:"
	head -10 "$REPO_ROOT/packages/go/v4/go.mod" | sed 's/^/    /'
else
	print_status FAIL "go.mod not found"
fi

if [ -d "$REPO_ROOT/packages/go/v4" ]; then
	print_status INFO "Go packages in v4:"
	find "$REPO_ROOT/packages/go/v4" -maxdepth 1 -type f -name "*.go" | head -5 | sed 's/^/  /'
fi
echo ""

print_section "Dependency Status"
print_status INFO "Checking PDFium..."
if [ -n "${KREUZBERG_PDFIUM_PREBUILT:-}" ] && [ -d "$KREUZBERG_PDFIUM_PREBUILT" ]; then
	print_status OK "PDFium found at $KREUZBERG_PDFIUM_PREBUILT"
	if find "$KREUZBERG_PDFIUM_PREBUILT/lib" -maxdepth 1 -type f 2>/dev/null | head -3 | xargs -r ls -lh; then
		:
	else
		echo "  Could not list PDFium libs"
	fi
else
	print_status WARN "PDFium not configured (KREUZBERG_PDFIUM_PREBUILT not set or invalid)"
fi

print_status INFO "Checking ONNX Runtime..."
if [ -n "${ORT_LIB_LOCATION:-}" ] && [ -d "$ORT_LIB_LOCATION" ]; then
	print_status OK "ONNX Runtime found at $ORT_LIB_LOCATION"
	if find "$ORT_LIB_LOCATION" -maxdepth 1 -type f 2>/dev/null | head -3 | xargs -r ls -lh; then
		:
	else
		echo "  Could not list ORT libs"
	fi
else
	print_status WARN "ONNX Runtime not configured (ORT_LIB_LOCATION not set or invalid)"
fi
echo ""

print_section "Timing Information"
START_TIME=$(date +%s)
print_status INFO "Script execution started at: $(date)"
echo ""

print_section "End of Diagnostics"
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
print_status INFO "Diagnostics completed in ${DURATION}s at $(date)"
