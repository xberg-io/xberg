#!/usr/bin/env bash
#MISE description="Capture two pinned development-page V3 forwards through production ORT"
set -euo pipefail

: "${XBERG_V3_PARITY_ROOT:?set the absolute frozen parity artifact root}"
: "${XBERG_V3_PARITY_MODEL:?set the absolute pinned published ONNX model path}"
: "${XBERG_V3_CAPTURE_OUT:?set a new absolute capture output directory}"
export XBERG_V3_PARITY_ROOT XBERG_V3_PARITY_MODEL XBERG_V3_CAPTURE_OUT

if [[ -e "$XBERG_V3_CAPTURE_OUT" || -L "$XBERG_V3_CAPTURE_OUT" ]]; then
  echo 'V3 capture output must be new' >&2
  exit 1
fi
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
validator="$root/scripts/ci/verify_v3_capture.py"
temporary="$(mktemp -d)"
trap 'rm -f "$temporary/list" "$temporary/runtime"; rmdir "$temporary"' EXIT
name=layout::models::pp_doclayout_v3::production_parity::should_capture_two_production_v3_forwards
command=(cargo test --locked -p xberg --no-default-features --features "layout-detection,ort-dynamic" --lib "$name")
"${command[@]}" -- --ignored --exact --list --format terse >"$temporary/list"
python3 "$validator" scope "$temporary/list" "$name"
"${command[@]}" -- --ignored --exact --nocapture 2>&1 | tee "$temporary/runtime"
python3 "$validator" capture "$temporary/runtime" "$XBERG_V3_CAPTURE_OUT"
