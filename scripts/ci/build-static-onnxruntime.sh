#!/usr/bin/env bash
# Build a static (non-shared) ONNX Runtime from source and stage its libraries for
# ort-sys's "system strategy" static linking (crates/xberg's `ort-static` feature).
#
# Usage:
#   scripts/ci/build-static-onnxruntime.sh <output-dir>
#
# Env overrides:
#   TARGET_TRIPLE   Rust target triple to build for. Only x86_64-unknown-linux-gnu is
#                    supported in this pass; any other value fails fast (#1284 follow-up:
#                    generalize once this prototype is validated in CI).
#   ORT_VERSION     ONNX Runtime git tag to build, without the leading `v`. Defaults to
#                    1.28.0 — the exact version ort-sys 2.0.0-rc.13's own `download-binaries`
#                    fallback pins for every target (see
#                    ort-sys-2.0.0-rc.13/build/download/dist.tsv, every row's
#                    `pyke:ort-rs/ms@1.28.0` URL segment). Building against this exact
#                    version keeps the from-source static libraries ABI-matched to what
#                    ort-sys's pregenerated bindings expect.
#   ORT_GIT_URL     Upstream repo to clone. Defaults to microsoft/onnxruntime.
#   WORK_DIR        Scratch directory for the clone + CMake build. Defaults to a fresh
#                    mktemp -d; NOT cleaned up on success so CI can inspect it on failure
#                    (the output-dir copy is a separate, much smaller artifact — see below).
#
# ---------------------------------------------------------------------------------------
# Output layout (read before changing this): ort-sys's static_link() (in the `ort-sys`
# crate's build/static_link/mod.rs) does NOT expect a hand-flattened directory of *.a
# files. It expects the *raw* ONNX Runtime CMake build output directory, as CMake
# produces it: the top-level `onnxruntime_*.a` files directly under the directory, plus
# a nested `_deps/<dep>-build/...` subtree for every vendored dependency (protobuf, onnx,
# google_nsync, pytorch_cpuinfo, re2, abseil — abseil alone spans ~10 subdirectories
# under `_deps/abseil_cpp-build/absl/*`). Reproducing that nested layout by hand in a
# truly flat staging directory would mean hardcoding every one of those subpaths and
# keeping them in lockstep with upstream ONNX Runtime's CMake structure — fragile, and
# liable to silently drift out of sync on a version bump.
#
# Instead, this script stages a *mirror* of the real build output: it walks the actual
# CMake build directory and copies every `*.a` file it finds, preserving each file's path
# relative to that directory (via `cp --parents`). The result is byte-for-byte the layout
# static_link() already knows how to search — just without the many GB of object files,
# downloaded dependency source trees, and (skipped, via --skip_tests) test binaries that
# live alongside the *.a files in the real build tree. ORT_LIB_PROFILE is deliberately
# left UNSET when consuming this output: ONNX Runtime's Linux build is a single-config
# generator (no per-config subdirectory), so the *.a files sit directly under
# `build/Linux/Release`, and mirroring that 1:1 means `<output-dir>/libonnxruntime_common.a`
# etc. directly — not `<output-dir>/Release/...`.
# ---------------------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
source "$REPO_ROOT/scripts/lib/retry.sh"

OUTPUT_DIR="${1:?usage: $0 <output-dir>}"

TARGET_TRIPLE="${TARGET_TRIPLE:-x86_64-unknown-linux-gnu}"
ORT_VERSION="${ORT_VERSION:-1.28.0}"
ORT_GIT_URL="${ORT_GIT_URL:-https://github.com/microsoft/onnxruntime.git}"
WORK_DIR="${WORK_DIR:-$(mktemp -d -t xberg-static-onnxruntime.XXXXXX)}"

if [[ "$TARGET_TRIPLE" != "x86_64-unknown-linux-gnu" ]]; then
  echo "::error::build-static-onnxruntime.sh only supports x86_64-unknown-linux-gnu in this pass, got '$TARGET_TRIPLE'"
  exit 1
fi

echo "::group::Cloning onnxruntime v${ORT_VERSION}"
ORT_SRC_DIR="$WORK_DIR/onnxruntime"
if [[ ! -d "$ORT_SRC_DIR" ]]; then
  retry_with_backoff git clone --branch "v${ORT_VERSION}" --depth 1 --recurse-submodules --shallow-submodules \
    "$ORT_GIT_URL" "$ORT_SRC_DIR"
fi
echo "::endgroup::"

echo "::group::Installing onnxruntime's Python build prerequisites"
# build.sh's CMake configure step shells out to a few Python helpers (tools/python/util)
# that need onnxruntime's own requirements.txt installed; best-effort since a CI image
# may already provision them.
if [[ -f "$ORT_SRC_DIR/requirements.txt" ]]; then
  python3 -m pip install --user -r "$ORT_SRC_DIR/requirements.txt"
fi
echo "::endgroup::"

echo "::group::Building onnxruntime v${ORT_VERSION} (static, CPU EP only)"
EXTRA_BUILD_ARGS=()
if [[ "$(id -u)" -eq 0 ]]; then
  EXTRA_BUILD_ARGS+=(--allow_running_as_root)
fi

# --build_shared_lib is deliberately OMITTED (static build). CPU execution provider
# only: no --use_cuda/--use_tensorrt/--use_coreml/etc — this prototype targets a plain
# statically-linked CPU binary; GPU EPs are a follow-up.
"$ORT_SRC_DIR/build.sh" \
  --config Release \
  --parallel \
  --compile_no_warning_as_error \
  --skip_tests \
  "${EXTRA_BUILD_ARGS[@]}"
echo "::endgroup::"

ORT_BUILD_DIR="$ORT_SRC_DIR/build/Linux/Release"
if [[ ! -d "$ORT_BUILD_DIR" ]]; then
  echo "::error::expected onnxruntime build output at $ORT_BUILD_DIR, but it does not exist"
  exit 1
fi

echo "::group::Staging static libraries into $OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"
# `cp --parents` preserves each *.a file's path relative to $ORT_BUILD_DIR, so the
# `_deps/<dep>-build/...` subtree static_link() searches comes along automatically —
# see the layout note above.
(cd "$ORT_BUILD_DIR" && find . -name '*.a' -exec cp --parents -t "$OUTPUT_DIR" {} +)
echo "::endgroup::"

echo "::group::Verifying staged artifacts"
# The top-level onnxruntime_* component libraries static_link() unconditionally requires
# (build/static_link/mod.rs: the `for lib in &["common", "flatbuffers", ...]` loop).
REQUIRED_RELATIVE_PATHS=(
  "libonnxruntime_common.a"
  "libonnxruntime_flatbuffers.a"
  "libonnxruntime_framework.a"
  "libonnxruntime_graph.a"
  "libonnxruntime_lora.a"
  "libonnxruntime_mlas.a"
  "libonnxruntime_optimizer.a"
  "libonnxruntime_providers.a"
  "libonnxruntime_session.a"
  "libonnxruntime_util.a"
  # Vendored dependencies static_link() links unconditionally (not behind
  # optional_link_lib): onnx, cpuinfo, re2. protobuf is one of three candidate names
  # (protobuf-lited/protobuf-lite/protobuf); at least one must exist.
  "_deps/onnx-build/libonnx.a"
  "_deps/onnx-build/libonnx_proto.a"
  "_deps/pytorch_cpuinfo-build/libcpuinfo.a"
  "_deps/re2-build/libre2.a"
)

missing=()
for rel in "${REQUIRED_RELATIVE_PATHS[@]}"; do
  if [[ ! -f "$OUTPUT_DIR/$rel" ]]; then
    missing+=("$rel")
  fi
done

if [[ ! -f "$OUTPUT_DIR/_deps/protobuf-build/libprotobuf-lited.a" \
  && ! -f "$OUTPUT_DIR/_deps/protobuf-build/libprotobuf-lite.a" \
  && ! -f "$OUTPUT_DIR/_deps/protobuf-build/libprotobuf.a" ]]; then
  missing+=("_deps/protobuf-build/libprotobuf{,-lite,-lited}.a (none found)")
fi

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "::error::static ONNX Runtime build is missing expected artifacts under $OUTPUT_DIR:"
  for rel in "${missing[@]}"; do
    echo "::error::  - $rel"
  done
  exit 1
fi
echo "::endgroup::"

echo "Static ONNX Runtime v${ORT_VERSION} staged at $OUTPUT_DIR"
echo "Set ORT_LIB_LOCATION=$OUTPUT_DIR and ORT_PREFER_DYNAMIC_LINK=0 when building xberg-cli."
