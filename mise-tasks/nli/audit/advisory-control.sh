#!/usr/bin/env bash
#MISE description="Prove advisory policy rejects a real unmaintained dependency as an error"
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
exec python3 "$root/tools/benchmark-harness/nli_qualification/audit_control.py" --advisories
