#!/usr/bin/env bash
#MISE description="Test the offline NLI qualification plan and result contracts"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
export CARGO_NET_OFFLINE=true ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
exec cargo test --offline --locked --manifest-path "$root/tools/benchmark-harness/nli_qualification/Cargo.toml" "$@"
