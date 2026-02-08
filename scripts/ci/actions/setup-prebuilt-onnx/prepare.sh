#!/usr/bin/env bash
set -euo pipefail

target="${1:?target required}"

case "$target" in
aarch64-apple-darwin)
  ort_url="https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.1/aarch64-apple-darwin.tgz"
  ;;
x86_64-apple-darwin)
  ort_url="https://cdn.pyke.io/0/pyke:ort-rs/ms@1.24.1/x86_64-apple-darwin.tgz"
  ;;
*)
  echo "setup-prebuilt-onnx does not support target $target" >&2
  exit 1
  ;;
esac

ort_dir="${GITHUB_WORKSPACE}/target/onnxruntime/${target}"
ort_root="${ort_dir}/onnxruntime"
ort_lib="${ort_root}/lib"

write_env() {
  {
    echo "ORT_STRATEGY=system"
    echo "ORT_LIB_LOCATION=${ort_lib}"
    echo "ORT_SKIP_DOWNLOAD=1"
    echo "ORT_PREFER_DYNAMIC_LINK=1"
  } >>"${GITHUB_ENV}"
}

if [ ! -f "${ort_lib}/libonnxruntime.a" ]; then
  rm -rf "${ort_dir}"
  mkdir -p "${ort_lib}"

  echo "Attempting to download prebuilt ONNX Runtime for ${target}..." >&2
  if curl -fsSL --max-time 30 -o /tmp/ort.tgz "${ort_url}" 2>/dev/null; then
    tar -xz -C "${ort_lib}" -f /tmp/ort.tgz
    rm -f /tmp/ort.tgz
    write_env
  else
    echo "Warning: Prebuilt ONNX Runtime not available for ${target}" >&2
    echo "Will download and build ONNX Runtime during compilation" >&2
  fi
else
  echo "Using existing ONNX Runtime at ${ort_lib}" >&2
  write_env
fi
