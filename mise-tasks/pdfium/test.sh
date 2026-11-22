#!/usr/bin/env bash
#MISE description="Provision the pinned libpdfium, then run the real-pdfium tests (fork crate and the xberg pdfium engine)"
#MISE depends=["pdfium:provision"]
#USAGE flag "--log <file>" help="Also write the combined test output to this file"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
cd "$root"
# XBERG_REQUIRE_PDFIUM=1 turns a missing library or fixture into a failure instead of a skip.
env_exports="$(bash scripts/pdfium/pdfium.sh env)"
eval "$env_exports"
echo "pdfium: PDFIUM_DYNAMIC_LIB_PATH=$PDFIUM_DYNAMIC_LIB_PATH" >&2

run() {
  RUST_TEST_THREADS=1 cargo test --locked "$@"
}

{
  run -p xberg-pdfium-render
  run -p xberg --features pdf,pdf-pdfium --lib extractors::pdf::
} 2>&1 | if [ -n "${usage_log:-}" ]; then tee "$usage_log"; else cat; fi
