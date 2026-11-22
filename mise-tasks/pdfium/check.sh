#!/usr/bin/env bash
#MISE description="Fail when a hardcoded pdfium build (Dockerfile ARGs, checksums, bindings) disagrees with scripts/pdfium-version"
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
exec bash "$root/scripts/pdfium/pdfium.sh" check
