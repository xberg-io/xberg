#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_DIR="${SCRIPT_DIR}/../datasets/omnidocbench"
TARGET_DIR="${1:-$DEFAULT_DIR}"

HF_BASE="https://huggingface.co/datasets/opendatalab/OmniDocBench/resolve/main"

mkdir -p "$TARGET_DIR"

if [ -f "$TARGET_DIR/OmniDocBench.json" ]; then
  echo "OmniDocBench.json already exists, skipping"
else
  echo "Downloading OmniDocBench.json (65 MB)..."
  curl -f -L -o "$TARGET_DIR/OmniDocBench.json" "$HF_BASE/OmniDocBench.json"
fi

if [ -d "$TARGET_DIR/images" ] && [ "$(find "$TARGET_DIR/images" -maxdepth 1 -type f 2>/dev/null | wc -l)" -gt 100 ]; then
  echo "images/ directory already populated ($(find "$TARGET_DIR/images" -maxdepth 1 -type f | wc -l) files), skipping"
else
  if command -v huggingface-cli &>/dev/null; then
    echo "Downloading full dataset via huggingface-cli..."
    huggingface-cli download opendatalab/OmniDocBench \
      --repo-type dataset \
      --local-dir "$TARGET_DIR" \
      --include "images/*" "ori_pdfs/*" "OmniDocBench.json"
  elif command -v git-lfs &>/dev/null || git lfs version &>/dev/null 2>&1; then
    echo "Downloading via git-lfs clone..."
    TEMP_CLONE="$(mktemp -d)"
    git clone --depth 1 "https://huggingface.co/datasets/opendatalab/OmniDocBench" "$TEMP_CLONE"
    cd "$TEMP_CLONE" && git lfs pull
    cp -r "$TEMP_CLONE/images" "$TARGET_DIR/"
    cp -r "$TEMP_CLONE/ori_pdfs" "$TARGET_DIR/"
    rm -rf "$TEMP_CLONE"
  else
    echo "ERROR: Need either huggingface-cli or git-lfs to download images."
    echo ""
    echo "Install one of:"
    echo "  pip install huggingface-hub   # then: huggingface-cli"
    echo "  brew install git-lfs          # then: git lfs install"
    exit 1
  fi
fi

echo ""
echo "OmniDocBench downloaded to: $TARGET_DIR"
echo "  Annotations: $(wc -c <"$TARGET_DIR/OmniDocBench.json" | tr -d ' ') bytes"
[ -d "$TARGET_DIR/images" ] && echo "  Images: $(find "$TARGET_DIR/images" -maxdepth 1 -type f | wc -l | tr -d ' ') files"
[ -d "$TARGET_DIR/ori_pdfs" ] && echo "  PDFs: $(find "$TARGET_DIR/ori_pdfs" -maxdepth 1 -type f | wc -l | tr -d ' ') files"
