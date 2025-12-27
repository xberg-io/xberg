#!/usr/bin/env bash
set -euo pipefail

crate_name="${CRATE_NAME:?CRATE_NAME required}"
features="${FEATURES:-}"
target="${TARGET:-}"
build_profile="${BUILD_PROFILE:-release}"
verbose="${VERBOSE:-true}"
additional_flags="${ADDITIONAL_FLAGS:-}"

echo "=== Building Rust FFI library ==="

if [ "$crate_name" = "kreuzberg-rb" ]; then
	CRATE_DIR="packages/ruby/ext/kreuzberg_rb/native"
	CARGO_ARGS=("build" "--manifest-path" "$CRATE_DIR/Cargo.toml")
else
	CRATE_DIR="crates/${crate_name}"
	CARGO_ARGS=("build" "--package" "$crate_name")
fi
export CRATE_DIR

if [ "$build_profile" = "release" ]; then
	CARGO_ARGS+=("--release")
	PROFILE_DIR="release"
else
	PROFILE_DIR="debug"
fi

if [ -n "$features" ]; then
	CARGO_ARGS+=("--features" "$features")
fi

if [ -n "$target" ]; then
	CARGO_ARGS+=("--target" "$target")
	TARGET_SUBDIR="${target}/"
else
	TARGET_SUBDIR=""
fi

if [ "$crate_name" = "kreuzberg-rb" ]; then
	FFI_MANIFEST="packages/ruby/vendor/Cargo.toml"
	if [ -f "$FFI_MANIFEST" ]; then
		echo "Cleaning vendored FFI workspace..."
		CLEAN_ARGS=("clean" "--manifest-path" "$FFI_MANIFEST" "-p" "kreuzberg-ffi")
		if [ "$build_profile" = "release" ]; then
			CLEAN_ARGS+=("--release")
		fi
		if [ -n "$target" ]; then
			CLEAN_ARGS+=("--target" "$target")
		fi
		cargo "${CLEAN_ARGS[@]}" || echo "Clean command completed"

		FFI_ARGS=("build" "--manifest-path" "$FFI_MANIFEST" "-p" "kreuzberg-ffi")
	else
		FFI_ARGS=("build" "--package" "kreuzberg-ffi")
	fi

	if [ "$build_profile" = "release" ]; then
		FFI_ARGS+=("--release")
	fi

	if [ -n "$target" ]; then
		FFI_ARGS+=("--target" "$target")
	fi

	echo "Build command: cargo ${FFI_ARGS[*]}"
	cargo "${FFI_ARGS[@]}"

	if [ -n "${CARGO_TARGET_DIR:-}" ]; then
		FFI_TARGET_DIR="$CARGO_TARGET_DIR"
	else
		FFI_TARGET_DIR="target"
	fi

	FFI_LINK_DIR="${FFI_TARGET_DIR}/${TARGET_SUBDIR}${PROFILE_DIR}"
	if [ -d "$FFI_LINK_DIR" ]; then
		if [ -n "${RUSTFLAGS:-}" ]; then
			export RUSTFLAGS="${RUSTFLAGS} -L ${FFI_LINK_DIR}"
		else
			export RUSTFLAGS="-L ${FFI_LINK_DIR}"
		fi
		echo "RUSTFLAGS: ${RUSTFLAGS}"
	fi
fi

if [ "$verbose" = "true" ]; then
	CARGO_ARGS+=("-vv")
fi

if [ -n "$additional_flags" ]; then
	read -ra EXTRA_FLAGS <<<"$additional_flags"
	CARGO_ARGS+=("${EXTRA_FLAGS[@]}")
fi

echo "Build command: cargo ${CARGO_ARGS[*]}"
echo ""

echo "=== Build Environment ==="
echo "Rust version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo "Working directory: $(pwd)"
echo "CARGO_TARGET_DIR: ${CARGO_TARGET_DIR:-<not set>}"
echo "RUST_BACKTRACE: ${RUST_BACKTRACE:-<not set>}"
echo "RUST_LOG: ${RUST_LOG:-<not set>}"
echo "CARGO_LOG: ${CARGO_LOG:-<not set>}"
if [ -n "$target" ]; then
	echo "Target: $target"
	echo "Target installed: $(rustup target list --installed | grep -q "$target" && echo "yes" || echo "no")"
fi
echo ""

export RUSTC_WRAPPER=""
export CARGO_BUILD_RUSTC_WRAPPER=""
export SCCACHE_GHA_ENABLED="false"

if [ -n "${OPENSSL_DIR:-}" ]; then
	export OPENSSL_DIR
	echo "OPENSSL_DIR: $OPENSSL_DIR"
fi
if [ -n "${OPENSSL_LIB_DIR:-}" ]; then
	export OPENSSL_LIB_DIR
	echo "OPENSSL_LIB_DIR: $OPENSSL_LIB_DIR"
fi
if [ -n "${OPENSSL_INCLUDE_DIR:-}" ]; then
	export OPENSSL_INCLUDE_DIR
	echo "OPENSSL_INCLUDE_DIR: $OPENSSL_INCLUDE_DIR"
fi

BUILD_LOG="$(mktemp)"
trap 'rm -f "$BUILD_LOG"' EXIT

if ! cargo "${CARGO_ARGS[@]}" 2>&1 | tee "$BUILD_LOG"; then
	echo ""
	echo "=== Build Failed ==="
	echo "Command: cargo ${CARGO_ARGS[*]}"
	echo ""
	echo "Last 50 lines of build output:"
	tail -50 "$BUILD_LOG"
	echo ""
	echo "Checking for common errors:"

	if grep -i "link" "$BUILD_LOG" | grep -i "error" | head -5; then
		echo "⚠️ Linking errors detected. Check library paths and dependencies."
	fi

	if grep -i "could not find" "$BUILD_LOG" | head -5; then
		echo "⚠️ Missing dependencies detected."
	fi

	if grep -i "openssl" "$BUILD_LOG" | grep -i "error" | head -5; then
		echo "⚠️ OpenSSL errors detected. Verify OPENSSL_DIR is set correctly."
	fi

	exit 1
fi

if [ -n "${CARGO_TARGET_DIR:-}" ]; then
	TARGET_DIR="$CARGO_TARGET_DIR"
else
	TARGET_DIR="target"
fi

FULL_TARGET_DIR="${TARGET_DIR}/${TARGET_SUBDIR}${PROFILE_DIR}"

echo ""
echo "=== Build Successful ==="
echo "Target directory: $FULL_TARGET_DIR"
echo ""
echo "Searching for built library artifacts..."

case "$crate_name" in
kreuzberg-ffi)
	LIB_PATTERNS="libkreuzberg_ffi.so libkreuzberg_ffi.dylib kreuzberg_ffi.dll libkreuzberg_ffi.a libkreuzberg_ffi.rlib"
	;;
kreuzberg-py)
	LIB_PATTERNS="lib_internal_bindings.so lib_internal_bindings.dylib _internal_bindings.pyd _internal_bindings.dll"
	;;
kreuzberg-node)
	LIB_PATTERNS="libkreuzberg_node.so libkreuzberg_node.dylib kreuzberg_node.dll kreuzberg_node.node"
	;;
kreuzberg-rb)
	LIB_PATTERNS="libkreuzberg_rb.so libkreuzberg_rb.dylib kreuzberg_rb.dll"
	;;
*)
	LIB_PATTERNS="lib${crate_name}.so lib${crate_name}.dylib ${crate_name}.dll"
	;;
esac

FOUND_LIB=""
for pattern in $LIB_PATTERNS; do
	if [ -f "$FULL_TARGET_DIR/$pattern" ]; then
		FOUND_LIB="$FULL_TARGET_DIR/$pattern"
		echo "✓ Found library: $FOUND_LIB"
		ls -lh "$FOUND_LIB"
		break
	fi
done

if [ -z "$FOUND_LIB" ]; then
	echo "⚠️ Could not find expected library artifact. Listing all files:"
	shopt -s nullglob
	candidates=(
		"$FULL_TARGET_DIR"/*.so
		"$FULL_TARGET_DIR"/*.dylib
		"$FULL_TARGET_DIR"/*.dll
		"$FULL_TARGET_DIR"/*.a
		"$FULL_TARGET_DIR"/*.rlib
		"$FULL_TARGET_DIR"/*.pyd
	)
	if ((${#candidates[@]})); then
		ls -lh "${candidates[@]}"
	else
		echo "No library files found"
	fi
fi

echo "library-path=$FOUND_LIB" >>"$GITHUB_OUTPUT"
echo "target-dir=$FULL_TARGET_DIR" >>"$GITHUB_OUTPUT"

echo ""
echo "=== FFI Build Complete ==="
