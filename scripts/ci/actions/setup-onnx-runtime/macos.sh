#!/usr/bin/env bash
set -euo pipefail

ort_version="${1:?ort-version required}"
dest_dir="${2:-crates/kreuzberg-node}"
arch_id="${3:-}"
strategy="${4:-system}"

extract_dir="$RUNNER_TEMP/onnxruntime"

if [ -z "$arch_id" ]; then
  arch="$(uname -m)"
  if [ "$arch" = "arm64" ]; then
    arch_id="arm64"
  else
    arch_id="x64"
  fi
fi

case "$arch_id" in
arm64) ort_arch="arm64" ;;
x64) ort_arch="x86_64" ;;
*)
  echo "Unsupported macOS arch-id: $arch_id" >&2
  exit 1
  ;;
esac
echo "Using macOS ONNX Runtime arch: $ort_arch"

if [ ! -d "$extract_dir/onnxruntime-osx-${ort_arch}-${ort_version}" ]; then
  echo "Cache miss: Downloading ONNX Runtime ${ort_version} for macOS ${ort_arch}"
  archive="onnxruntime-osx-${ort_arch}-${ort_version}.tgz"
  curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors -o "$RUNNER_TEMP/$archive" "https://github.com/microsoft/onnxruntime/releases/download/v${ort_version}/$archive"
  mkdir -p "$extract_dir"
  tar -xzf "$RUNNER_TEMP/$archive" -C "$extract_dir"
else
  echo "Cache hit: Using cached ONNX Runtime ${ort_version}"
fi

ort_root="$extract_dir/onnxruntime-osx-${ort_arch}-${ort_version}"

if [ ! -d "$ort_root/lib" ]; then
  echo "ERROR: ONNX Runtime lib directory missing at $ort_root/lib" >&2
  echo "Available directories:" >&2
  ls -la "$extract_dir" >&2 || true
  exit 1
fi

if ! ls "$ort_root/lib"/libonnxruntime*.dylib 1>/dev/null 2>&1; then
  echo "ERROR: No ONNX Runtime libraries found in $ort_root/lib" >&2
  echo "Directory contents:" >&2
  ls -la "$ort_root/lib" >&2 || true
  exit 1
fi

dest="$GITHUB_WORKSPACE/$dest_dir"
mkdir -p "$dest"
cp -f "$ort_root/lib/"libonnxruntime*.dylib "$dest/"

if [ -n "${RUSTFLAGS:-}" ]; then
  rustflags="$RUSTFLAGS -L $ort_root/lib"
else
  rustflags="-L $ort_root/lib"
fi

if [ "$strategy" = "bundled" ]; then
  echo "Using bundled ORT strategy — skipping system env vars so ort-bundled cargo feature takes effect"
  {
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "DYLD_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_LIBRARY_PATH:-}"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_FALLBACK_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
else
  {
    ort_lib=$(find "$ort_root/lib" -name "libonnxruntime*.dylib" -print -quit)
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "ORT_PREFER_DYNAMIC_LINK=1"
    echo "ORT_SKIP_DOWNLOAD=1"
    echo "ORT_STRATEGY=system"
    echo "ORT_DYLIB_PATH=$ort_root/lib/${ort_lib##*/}"
    echo "DYLD_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_LIBRARY_PATH:-}"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_FALLBACK_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
fi
