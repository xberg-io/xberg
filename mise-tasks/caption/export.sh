#!/usr/bin/env bash
#MISE description="Export or validate an explicitly selected local caption source offline"
set -euo pipefail

project="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/tools/benchmark-harness/caption_export"
export PYTHONPATH="${project}/src"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1
export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 TOKENIZERS_PARALLELISM=false
export UV_OFFLINE=1
exec uv run --frozen --no-sync --project "$project" python -m caption_export "$@"
