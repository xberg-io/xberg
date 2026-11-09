#!/usr/bin/env bash
#MISE description="Audit leaf licenses/sources/bans offline; opt in to live advisory checks"
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
manifest="$root/tools/benchmark-harness/nli_qualification/Cargo.toml"
advisories=false
offline_requested=false
for argument in "$@"; do
  case "$argument" in
  --advisories) advisories=true ;;
  --offline) offline_requested=true ;;
  *)
    printf 'usage: nli:audit [--advisories] [--offline]\n' >&2
    exit 2
    ;;
  esac
done
checks=(bans licenses sources)
offline=(--offline)
if [ "$advisories" = true ]; then
  checks=(advisories bans licenses sources)
  if [ "$offline_requested" = false ]; then offline=(); fi
fi
exec cargo deny "${offline[@]}" --locked --manifest-path "$manifest" --all-features \
  --config "$root/tools/benchmark-harness/nli_qualification/deny.toml" check "${checks[@]}"
