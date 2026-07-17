#!/usr/bin/env bash
# Report which tracked carry-patch files an upstream ref changed since the merge-base.
set -euo pipefail
REF="${1:-upstream/main}"
ROOT="$(git rev-parse --show-toplevel)"
TSV="$ROOT/docs/superpowers/upstream/carry-patches.tsv"
MB="$(git merge-base HEAD "$REF")"
echo "merge-base: $MB"
echo "upstream ref: $REF ($(git rev-parse --short "$REF"))"
echo "--- carry-patch files touched upstream since merge-base ---"
if [ ! -f "$TSV" ]; then echo "(no manifest yet: $TSV)"; exit 0; fi
touched="$(git diff --name-only "$MB" "$REF")"
n=0
while IFS=$'\t' read -r path _tier feature note; do
  [ "$path" = "path" ] && continue          # header
  [ -z "${path:-}" ] && continue
  if printf '%s\n' "$touched" | grep -qxF "$path"; then
    printf '  %-55s [%s] %s\n' "$path" "$feature" "$note"
    n=$((n+1))
  fi
done < "$TSV"
echo "--- $n carry-patch file(s) need manual review on resync ---"
