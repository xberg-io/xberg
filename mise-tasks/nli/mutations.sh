#!/usr/bin/env bash
#MISE description="List or replay the curated NLI source-bypass mutation controls"
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
exec python3 "$root/tools/benchmark-harness/nli_qualification/mutation_controls.py" "$@"
