#!/usr/bin/env bash
set -euo pipefail

pdfium_path="${1:?pdfium path required}"
echo "KREUZBERG_PDFIUM_PREBUILT=${pdfium_path}" >>"$GITHUB_ENV"
echo "pdfium-path=${pdfium_path}" >>"$GITHUB_OUTPUT"
