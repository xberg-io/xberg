#!/usr/bin/env bash
set -euo pipefail

ort_version="${1:?ort-version required}"
dest_dir="${2:-crates/xberg-node}"
arch_id="${3:-}"
strategy="${4:-system}"
verify_archive="${5:-false}"
script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# A custom version needs a trusted manifest with its NuGet package and PyPI wheel digests. ~keep
checksums="${ORT_CHECKSUMS_FILE:-$script_dir/checksums.sha256}"
case "$verify_archive" in
true | false) ;;
*)
  echo "verify-linux-archive must be true or false" >&2
  exit 2
  ;;
esac

if [[ ! "$ort_version" =~ ^[A-Za-z0-9]([A-Za-z0-9._-]*[A-Za-z0-9])?$ ]]; then
  echo "Invalid ONNX Runtime version: $ort_version" >&2
  exit 1
fi

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
  nuget_rid="linux-x64"
  ;;
arm64)
  ort_dir_name="onnxruntime-linux-aarch64-${ort_version}"
  nuget_rid="linux-arm64"
  ;;
*)
  echo "Unsupported Linux arch-id: $arch_id" >&2
  exit 1
  ;;
esac

stage_runtime() {
  bash "$script_dir/stage-runtime.sh" "$ort_version" "$nuget_rid" "$1/$ort_dir_name" "$checksums"
}

if [ "$verify_archive" = true ]; then
  # The extracted-tree cache keeps no package to re-verify, so stage freshly verified bytes. ~keep
  extract_dir="$(mktemp -d "$RUNNER_TEMP/onnxruntime-verified.XXXXXX")"
  stage_runtime "$extract_dir" || {
    status=$?
    rm -rf "$extract_dir"
    exit "$status"
  }
elif [ ! -d "$extract_dir/$ort_dir_name" ]; then
  echo "Cache miss: Downloading ONNX Runtime ${ort_version}"
  stage_runtime "$extract_dir"
else
  # The extracted-tree cache does not retain the package for re-verification. ~keep
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

rpath_flag="-C link-arg=-Wl,-rpath,\$ORIGIN"
if [ -n "${RUSTFLAGS:-}" ]; then
  rustflags="$RUSTFLAGS -L $ort_root/lib $rpath_flag"
else
  rustflags="-L $ort_root/lib $rpath_flag"
fi

if [ "$strategy" = "bundled" ]; then
  echo "Using bundled ORT strategy — letting ort-sys download-binaries handle static linking"
  {
    echo "LD_LIBRARY_PATH=$ort_root/lib:$dest:${LD_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
  } >>"$GITHUB_ENV"
else
  # `libonnxruntime*.so*` also matches libonnxruntime_providers_shared.so, which an
  # ort-dynamic consumer would then try to load as the runtime. ~keep
  ort_lib="$ort_root/lib/libonnxruntime.so"
  if [ ! -e "$ort_lib" ]; then
    echo "ERROR: $ort_lib missing from the staged ONNX Runtime" >&2
    exit 1
  fi
  {
    echo "ORT_LIB_LOCATION=$ort_root/lib"
    echo "ORT_PREFER_DYNAMIC_LINK=1"
    echo "ORT_SKIP_DOWNLOAD=1"
    echo "ORT_STRATEGY=system"
    echo "ORT_DYLIB_PATH=$ort_lib"
    echo "LD_LIBRARY_PATH=$ort_root/lib:$dest:${LD_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
    echo "RUSTFLAGS=$rustflags"
  } >>"$GITHUB_ENV"
fi
