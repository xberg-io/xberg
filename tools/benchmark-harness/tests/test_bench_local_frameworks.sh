#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../scripts/bench_local_frameworks.sh"

assert_equals() {
  local expected="$1"
  local actual="$2"
  if [ "$actual" != "$expected" ]; then
    echo "expected '$expected', got '$actual'" >&2
    exit 1
  fi
}

default_selection="$(native_batch_frameworks 'markitdown,xberg-markdown-layout,docling,unstructured,liteparse,xberg-markdown-baseline,xberg-markdown-layout')"
assert_equals 'xberg-markdown-layout,docling,liteparse,xberg-markdown-baseline' "$default_selection"
assert_equals '' "$(native_batch_frameworks '')"

explicit_selection="$(validate_native_batch_frameworks 'docling,xberg-markdown-paddle-ocr-batch,liteparse,docling')"
assert_equals 'docling,xberg-markdown-paddle-ocr-batch,liteparse' "$explicit_selection"

if validate_native_batch_frameworks 'xberg-private,unstructured' >/dev/null 2>&1; then
  echo "invalid explicit native-batch frameworks were accepted" >&2
  exit 1
fi

for malformed in ',docling' 'docling,' 'docling,,liteparse' ','; do
  if validate_native_batch_frameworks "$malformed" >/dev/null 2>&1; then
    echo "malformed explicit native-batch framework list was accepted: $malformed" >&2
    exit 1
  fi
done

framework_list_contains 'xberg-markdown-baseline,docling' docling
if framework_list_contains 'xberg-markdown-baseline,liteparse' docling; then
  echo "docling membership check produced a false positive" >&2
  exit 1
fi

docling_is_explicitly_requested 'xberg-markdown-baseline' 'docling,liteparse'
if docling_is_explicitly_requested 'xberg-markdown-baseline' 'liteparse'; then
  echo "batch-only Docling preflight check produced a false positive" >&2
  exit 1
fi
