#!/usr/bin/env bash
#MISE description="Run the benchmark through the CI-compatible wrapper"
set -euo pipefail

# The wrapper reads its whole configuration from the environment. FRAMEWORK and MODE are
# required; the rest default as below, and the fixture corpus and harness path are fixed. ~keep
: "${FRAMEWORK:?set FRAMEWORK to the framework to benchmark}"
: "${MODE:?set MODE to the benchmark mode}"
export FRAMEWORK MODE
export RUST_BACKTRACE=short
export FIXTURES_DIR="tools/benchmark-harness/fixtures"
export HARNESS_PATH="./target/release/benchmark-harness"
export ITERATIONS="${ITERATIONS:-3}"
export TIMEOUT="${TIMEOUT:-900}"
export OUTPUT_FORMAT="${OUTPUT_FORMAT:-markdown}"
export MEASURE_QUALITY="${MEASURE_QUALITY:-false}"
export OCR_ENABLED="${OCR_ENABLED:-false}"
export COHORT="${COHORT:-}"
export BATCH_SIZE="${BATCH_SIZE:-}"
export SHARD="${SHARD:-}"
export XBERG_MAX_THREADS="${XBERG_MAX_THREADS:-4}"
export MIN_SUCCESS_RATE="${MIN_SUCCESS_RATE:-0.5}"
exec scripts/benchmarks/run-benchmark.sh
