#!/usr/bin/env bash
set -euo pipefail

ort_version="${1:?ort-version required}"
dest_dir="${2:-crates/kreuzberg-node}"
arch_id="${3:-}"
strategy="${4:-system}"

extract_dir="$RUNNER_TEMP/onnxruntime"

if [ -z "$arch_id" ]; then
  case "$(uname -m)" in
  x86_64 | amd64) arch_id="x64" ;;
  arm64 | aarch64) arch_id="arm64" ;;
  *)
    echo "Unsupported Linux architecture: $(uname -m)" >&2
    exit 1
    ;;
  esac
fi

case "$arch_id" in
x64)
  ort_dir_name="onnxruntime-linux-x64-${ort_version}"
  archive="onnxruntime-linux-x64-${ort_version}.tgz"
  ;;
arm64)
  ort_dir_name="onnxruntime-linux-aarch64-${ort_version}"
  archive="onnxruntime-linux-aarch64-${ort_version}.tgz"
  ;;
*)
  echo "Unsupported Linux arch-id: $arch_id" >&2
  exit 1
  ;;
esac

if [ ! -d "$extract_dir/$ort_dir_name" ]; then
  echo "Cache miss: Downloading ONNX Runtime ${ort_version}"
  curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors -o "$RUNNER_TEMP/$archive" "https://github.com/microsoft/onnxruntime/releases/download/v${ort_version}/$archive"
  mkdir -p "$extract_dir"
  tar -xzf "$RUNNER_TEMP/$archive" -C "$extract_dir"
else
  echo "Cache hit: Using cached ONNX Runtime ${ort_version}"
fi

ort_root="$extract_dir/$ort_dir_name"

if [ ! -d "$ort_root/lib" ]; then
  echo "ERROR: ONNX Runtime lib directory missing at $ort_root/lib" >&2
  echo "Available directories:" >&2
  ls -la "$extract_dir" >&2 || true
  exit 1
fi

if ! ls "$ort_root/lib"/*.so* 1>/dev/null 2>&1; then
  echo "ERROR: No ONNX Runtime libraries found in $ort_root/lib" >&2
  echo "Directory contents:" >&2
  ls -la "$ort_root/lib" >&2 || true
  exit 1
fi

dest="$GITHUB_WORKSPACE/$dest_dir"
mkdir -p "$dest"
cp -f "$ort_root/lib/"*.so* "$dest/"

if [ -n "${RUSTFLAGS:-}" ]; then
  rustflags="$RUSTFLAGS -L $ort_root/lib"
else
  rustflags="-L $ort_root/lib"
fi

if [ "$strategy" = "bundled" ]; then
  echo "Using bundled ORT strategy — skipping system env vars so ort-bundled cargo feature takes effect"
  {
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "LD_LIBRARY_PATH=$ort_root/lib:$dest:${LD_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
else
  {
    ort_lib=$(find "$ort_root/lib" -name "libonnxruntime*.so*" -print -quit)
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "ORT_PREFER_DYNAMIC_LINK=1"
    echo "ORT_SKIP_DOWNLOAD=1"
    echo "ORT_STRATEGY=system"
    echo "ORT_DYLIB_PATH=$ort_root/lib/${ort_lib##*/}"
    echo "LD_LIBRARY_PATH=$ort_root/lib:$dest:${LD_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
fi
