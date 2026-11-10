#!/usr/bin/env bash
#MISE description="Prove the actual leaf audit rejects a planted MIT license refusal"
set -euo pipefail
root="$(cd "$(dirname "$0")/../../.." && pwd)"
exec python3 "$root/tools/benchmark-harness/nli_qualification/audit_control.py"
