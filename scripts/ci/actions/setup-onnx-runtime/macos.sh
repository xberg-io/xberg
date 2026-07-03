#!/usr/bin/env bash
set -euo pipefail

ort_version="${1:?ort-version required}"
dest_dir="${2:-crates/xberg-node}"
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

ort_root="$extract_dir/onnxruntime-osx-${ort_arch}-${ort_version}"

if [ ! -d "$ort_root" ]; then
  if [ "$ort_arch" = "x86_64" ]; then
    # Microsoft dropped onnxruntime-osx-x86_64 after 1.23, below the 1.24+ ort
    # needs; use Homebrew's x86_64 build instead of a download that would 404.
    echo "Installing x86_64 macOS ONNX Runtime via Homebrew"
    export HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK=1
    brew install --bottle-tag=sonoma onnxruntime || brew install onnxruntime
    mkdir -p "$ort_root/lib"
    cp -f "$(brew --prefix onnxruntime)/lib"/libonnxruntime*.dylib "$ort_root/lib/"
  else
    echo "Cache miss: Downloading ONNX Runtime ${ort_version} for macOS ${ort_arch}"
    archive="onnxruntime-osx-${ort_arch}-${ort_version}.tgz"
    mkdir -p "$extract_dir"
    curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors -o "$RUNNER_TEMP/$archive" \
      "https://github.com/microsoft/onnxruntime/releases/download/v${ort_version}/$archive"
    tar -xzf "$RUNNER_TEMP/$archive" -C "$extract_dir"
  fi
else
  echo "Cache hit: Using cached ONNX Runtime ${ort_version}"
fi

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

# `-L` lets the linker find libonnxruntime at build time. The ORT dylib's
# install_name is `@rpath/libonnxruntime.<ver>.dylib`, so the consuming binary
# must carry an LC_RPATH for that lookup to resolve at load time. The dylib is
# bundled next to the .node file in the npm package, so add an `@loader_path`
# rpath: dlopen then resolves @rpath relative to the directory holding the
# .node binary. Without this the published binary has no LC_RPATH and fails
# with ERR_DLOPEN_FAILED at require() time.
rpath_flag="-C link-arg=-Wl,-rpath,@loader_path"
if [ -n "${RUSTFLAGS:-}" ]; then
  rustflags="$RUSTFLAGS -L $ort_root/lib $rpath_flag"
else
  rustflags="-L $ort_root/lib $rpath_flag"
fi

if [ "$strategy" = "bundled" ]; then
  echo "Using bundled ORT strategy — letting ort-sys download-binaries handle static linking"
  {
    echo "DYLD_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_LIBRARY_PATH:-}"
    echo "DYLD_FALLBACK_LIBRARY_PATH=$ort_root/lib:$dest:${DYLD_FALLBACK_LIBRARY_PATH:-}"
    echo "LIBRARY_PATH=$ort_root/lib:$dest:${LIBRARY_PATH:-}"
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
