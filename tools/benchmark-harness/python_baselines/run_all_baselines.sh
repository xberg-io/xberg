#!/bin/bash
# Run all VLM-OCR baseline generation scripts.
#
# Usage:
#   ./run_all_baselines.sh              # Run all three models
#   MODELS="deepseek" ./run_all_baselines.sh  # Run DeepSeek only
#   FIXTURES=/path/to/fixtures ./run_all_baselines.sh
#
# Environment:
#   MODELS          Space-separated model names (deepseek, hunyuan, paddleocr)
#   FIXTURES        Path to fixtures directory (default: ../../fixtures)
#   OUTPUT_BASE     Base output directory (default: baselines/)
#   DEVICE          CUDA device or CPU (default: cuda)
#   HF_TOKEN        HuggingFace API token (for gated models)
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
MODELS="${MODELS:-deepseek hunyuan paddleocr}"
FIXTURES="${FIXTURES:-../../fixtures}"
OUTPUT_BASE="${OUTPUT_BASE:-baselines}"
DEVICE="${DEVICE:-cuda}"

# Ensure output base exists
mkdir -p "$OUTPUT_BASE"

echo "======================================================================"
echo "VLM-OCR Baseline Generation Runner"
echo "======================================================================"
echo "Models to run: $MODELS"
echo "Fixtures: $FIXTURES"
echo "Output base: $OUTPUT_BASE"
echo "Device: $DEVICE"
echo "======================================================================"

# Verify fixtures directory
if [ ! -d "$FIXTURES" ]; then
  echo "ERROR: Fixtures directory not found: $FIXTURES"
  exit 1
fi

# Track overall status
all_success=true

# Run each model
for model in $MODELS; do
  case "$model" in
  deepseek)
    echo ""
    echo ">>> Running DeepSeek-OCR baseline..."
    output_dir="$OUTPUT_BASE/deepseek_ocr"
    if python deepseek_ocr_baseline.py \
      --fixtures "$FIXTURES" \
      --output "$output_dir" \
      --device "$DEVICE"; then
      echo "✓ DeepSeek-OCR complete"
    else
      echo "✗ DeepSeek-OCR failed"
      all_success=false
    fi
    ;;

  hunyuan)
    echo ""
    echo ">>> Running Hunyuan-OCR baseline..."
    output_dir="$OUTPUT_BASE/hunyuan_ocr"
    if python hunyuan_ocr_baseline.py \
      --fixtures "$FIXTURES" \
      --output "$output_dir"; then
      echo "✓ Hunyuan-OCR complete"
    else
      echo "✗ Hunyuan-OCR failed"
      all_success=false
    fi
    ;;

  paddleocr)
    echo ""
    echo ">>> Running PaddleOCR-VL baseline..."
    output_dir="$OUTPUT_BASE/paddleocr_vl"
    if python paddleocr_vl_baseline.py \
      --fixtures "$FIXTURES" \
      --output "$output_dir" \
      --device "$DEVICE"; then
      echo "✓ PaddleOCR-VL complete"
    else
      echo "✗ PaddleOCR-VL failed"
      all_success=false
    fi
    ;;

  *)
    echo "WARNING: Unknown model: $model (skipping)"
    ;;
  esac
done

# Summary
echo ""
echo "======================================================================"
echo "Baseline Generation Summary"
echo "======================================================================"
if [ "$all_success" = true ]; then
  echo "✓ All baseline generation runs completed successfully"
  echo "  Check $OUTPUT_BASE/ for generated baseline files"
  echo "======================================================================"
  exit 0
else
  echo "✗ One or more baseline generation runs failed"
  echo "  Check logs above for details"
  echo "======================================================================"
  exit 1
fi
