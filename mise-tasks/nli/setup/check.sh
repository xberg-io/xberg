#!/usr/bin/env bash
#MISE description="Check NLI native setup versions, archive checksums and refusal controls"
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
exec python3 "$root/tools/benchmark-harness/nli_qualification/setup_checks.py"
