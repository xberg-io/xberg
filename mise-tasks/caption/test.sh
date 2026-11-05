#!/usr/bin/env bash
#MISE description="Test offline caption export and decoder diagnostic contracts"
set -euo pipefail

project="$(cd "$(dirname "$0")/../.." && pwd)/tools/benchmark-harness/caption_export"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 HF_HUB_DISABLE_TELEMETRY=1
export OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 TOKENIZERS_PARALLELISM=false
export UV_OFFLINE=1 PYTHONDONTWRITEBYTECODE=1
exec uv run --frozen --no-sync --project "$project" python -m pytest "$project/tests" "$@"
