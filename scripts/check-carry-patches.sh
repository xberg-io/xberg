#!/usr/bin/env bash
# Fail if a co-modified crates/xberg* file drifted from merge-base but is absent from the TSV.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
TSV="$ROOT/docs/superpowers/upstream/carry-patches.tsv"
MB="$(git merge-base HEAD upstream/main 2>/dev/null || true)"
if [ -z "$MB" ]; then echo "no upstream merge-base; skipping"; exit 0; fi
if [ ! -f "$TSV" ]; then echo "no carry-patch manifest: $TSV"; exit 0; fi
tracked="$(cut -f1 "$TSV" | tail -n +2 | sort -u)"
missing=0
while IFS= read -r f; do
  case "$f" in
    crates/xberg/src/api/rag/*) continue ;;                 # Tier-1 extracted module
    crates/xberg/*|crates/xberg-rag/*|crates/xberg-ffi/*|crates/xberg-wasm/src/lib.rs|crates/xberg-wasm/Cargo.toml|crates/xberg-node/src/lib.rs|crates/xberg-node/index.d.ts)
      if ! printf '%s\n' "$tracked" | grep -qxF "$f"; then
        echo "UNTRACKED co-modified file: $f"; missing=1
      fi ;;
  esac
done < <(git diff --name-only "$MB" HEAD)
if [ "$missing" -eq 0 ]; then
  echo "carry-patch manifest OK"
else
  echo "add the above to carry-patches.tsv"
  exit 1
fi
