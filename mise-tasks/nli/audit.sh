#!/usr/bin/env bash
#MISE description="Audit the standalone NLI lockfile and optional-runtime dependencies"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/nli_qualification/Cargo.toml"
exec mise exec cargo-deny -- cargo deny --locked --manifest-path "$manifest" \
  --all-features --config "$root/deny.toml" "$@" check
