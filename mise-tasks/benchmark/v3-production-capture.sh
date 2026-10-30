#!/usr/bin/env bash
#MISE description="Capture two pinned development-page V3 forwards through production ORT"
set -euo pipefail

: "${XBERG_V3_PARITY_ROOT:?set the absolute frozen parity artifact root}"
: "${XBERG_V3_PARITY_MODEL:?set the absolute pinned published ONNX model path}"
: "${XBERG_V3_CAPTURE_OUT:?set a new absolute capture output directory}"
export XBERG_V3_PARITY_ROOT XBERG_V3_PARITY_MODEL XBERG_V3_CAPTURE_OUT
exec cargo test --locked -p xberg --no-default-features --features layout-detection,ort-dynamic \
  --lib layout::models::pp_doclayout_v3::production_parity::should_capture_two_production_v3_forwards \
  -- --ignored --exact --nocapture
