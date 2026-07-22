#!/usr/bin/env bash
# Restore the license-restricted "reference" benchmark corpus slice into
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "${REPO_ROOT}/scripts/lib/common.sh"
validate_repo_root "$REPO_ROOT" || exit 1

BUCKET="${GCP_BENCHMARK_BUCKET:-xberg-benchmark-corpus}"
TEST_DOCS="${REPO_ROOT}/test_documents"

SHA="$(git -C "$TEST_DOCS" rev-parse HEAD)"
OBJECT="gs://${BUCKET}/corpus-cache/${SHA}.tar.zst"

echo "Restoring reference corpus for test_documents ${SHA:0:12} from ${OBJECT}..."
if ! gcloud storage objects describe "$OBJECT" >/dev/null 2>&1; then
  echo "::error::${OBJECT} not found — no reference corpus cache published for this test_documents SHA." >&2
  echo "::error::A maintainer must run 'task benchmark:corpus:cache:publish' (after build_corpus.py --stage materialize) for ${SHA:0:12}." >&2
  exit 1
fi

TARBALL="$(mktemp -t corpus-cache-XXXXXX).tar.zst"
trap 'rm -f "$TARBALL"' EXIT

gcloud storage cp "$OBJECT" "$TARBALL"
mkdir -p "${TEST_DOCS}/.corpus-cache"
zstd -dc "$TARBALL" | tar -C "$TEST_DOCS" -xf -

pdf_count="$(find "${TEST_DOCS}/.corpus-cache/pdf" -type f 2>/dev/null | wc -l | tr -d ' ')"
gt_count="$(find "${TEST_DOCS}/.corpus-cache/ground_truth/pdf" -type f 2>/dev/null | wc -l | tr -d ' ')"
echo "✓ Restored ${pdf_count} reference PDFs + ${gt_count} ground-truth files."
