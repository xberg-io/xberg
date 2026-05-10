#!/usr/bin/env bash
set -euo pipefail

FRAMEWORK="${FRAMEWORK:-}"
MODE="${MODE:-}"
ITERATIONS="${ITERATIONS:-3}"
TIMEOUT="${TIMEOUT:-900}"
FIXTURES_DIR="${FIXTURES_DIR:-tools/benchmark-harness/fixtures}"
HARNESS_PATH="${HARNESS_PATH:-./target/release/benchmark-harness}"
MEASURE_QUALITY="${MEASURE_QUALITY:-false}"
OCR_ENABLED="${OCR_ENABLED:-false}"
OUTPUT_FORMAT="${OUTPUT_FORMAT:-markdown}"

if [ -z "$FRAMEWORK" ] || [ -z "$MODE" ]; then
  echo "::error::FRAMEWORK and MODE environment variables are required" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

source "${REPO_ROOT}/scripts/lib/common.sh"
source "${REPO_ROOT}/scripts/lib/library-paths.sh"

validate_repo_root "$REPO_ROOT" || exit 1

setup_go_paths "$REPO_ROOT"
setup_onnx_paths

OUTPUT_DIR="benchmark-results/${FRAMEWORK}-${OUTPUT_FORMAT}-${MODE}"
rm -rf "${OUTPUT_DIR}"

MAX_CONCURRENT=$([[ "$MODE" == "single-file" ]] && echo 1 || echo 4)

SHARD="${SHARD:-}"

EXTRA_ARGS=()
if [ "$MEASURE_QUALITY" = "true" ]; then
  EXTRA_ARGS+=("--measure-quality")
fi
if [ "$OCR_ENABLED" = "true" ]; then
  EXTRA_ARGS+=("--ocr")
fi
if [ -n "$SHARD" ]; then
  EXTRA_ARGS+=("--shard" "${SHARD}")
fi

BENCHMARK_DEBUG=1 "${HARNESS_PATH}" \
  run \
  --fixtures "${FIXTURES_DIR}" \
  --frameworks "${FRAMEWORK}" \
  --output "${OUTPUT_DIR}" \
  --iterations "${ITERATIONS}" \
  --timeout "${TIMEOUT}" \
  --mode "${MODE}" \
  --max-concurrent "${MAX_CONCURRENT}" \
  --output-format "${OUTPUT_FORMAT}" \
  "${EXTRA_ARGS[@]+"${EXTRA_ARGS[@]}"}"
