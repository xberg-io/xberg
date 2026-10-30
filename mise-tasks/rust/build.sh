#!/usr/bin/env bash
#MISE description="Build all Rust crates with the BUILD_PROFILE profile (dev, release or ci)"
#USAGE arg "[profile]" env="BUILD_PROFILE" default="release" help="dev, release, or ci (release with debug info)"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# xberg-pdfium-render is excluded: --all-features enables its `bindings` feature, which runs
# bindgen from build.rs; the crate ships no pdfium headers (include/pdfium_7678/ holds only the
# 0-byte generated wrapper, which the scan skips), so build.rs fails rather than silently reusing
# the checked-in src/bindgen/pdfium_7678.rs. Static vs dynamic linking is NOT a feature here --
# it is selected by the PDFIUM_STATIC_LIB_PATH / PDFIUM_DYNAMIC_LIB_PATH env vars (build.rs:54-60).
# Windows builds xberg-candle-ocr and skips the benchmark harness instead. ~keep
excludes=(--exclude xberg-php --exclude xberg-node --exclude xberg-wasm --exclude xberg-pdfium-render)
case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*) excludes+=(--exclude benchmark-harness) ;;
*) excludes+=(--exclude xberg-candle-ocr) ;;
esac

case "$usage_profile" in
dev) profile=() ;;
release) profile=(--release) ;;
ci)
  profile=(--release)
  export RUSTFLAGS="-C debuginfo=2"
  ;;
*)
  echo "profile must be dev, release or ci, got: $usage_profile" >&2
  exit 2
  ;;
esac

cargo build --locked ${profile[@]+"${profile[@]}"} --workspace --all-features "${excludes[@]}"
