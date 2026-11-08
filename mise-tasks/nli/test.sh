#!/usr/bin/env bash
#MISE description="Test the offline NLI qualification plan and result contracts"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target/codex2334}"
export CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
export CARGO_NET_OFFLINE=true ORT_OFFLINE=1 HF_HUB_OFFLINE=1 TSLP_OFFLINE=1
exec cargo test --offline --locked --manifest-path "$root/tools/benchmark-harness/nli_qualification/Cargo.toml" "$@"
