#!/usr/bin/env bash
# Package the license-restricted "reference" benchmark corpus slice
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "${REPO_ROOT}/scripts/lib/common.sh"
validate_repo_root "$REPO_ROOT" || exit 1

BUCKET="${GCP_BENCHMARK_BUCKET:-xberg-benchmark-corpus}"
TEST_DOCS="${REPO_ROOT}/test_documents"
CACHE="${TEST_DOCS}/.corpus-cache"

SHA="$(git -C "$TEST_DOCS" rev-parse HEAD)"
OBJECT="gs://${BUCKET}/corpus-cache/${SHA}.tar.zst"

for sub in "pdf" "ground_truth/pdf"; do
  if [ ! -d "${CACHE}/${sub}" ] || [ -z "$(ls -A "${CACHE}/${sub}" 2>/dev/null)" ]; then
    echo "::error::${CACHE}/${sub} is missing or empty." >&2
    echo "::error::Materialize it first: python tools/benchmark-harness/scripts/build_corpus.py --stage materialize" >&2
    exit 1
  fi
done

TARBALL="$(mktemp -t corpus-cache-XXXXXX).tar.zst"
trap 'rm -f "$TARBALL"' EXIT

echo "Packaging reference corpus (.corpus-cache/{pdf,ground_truth/pdf}) for test_documents ${SHA:0:12}..."
tar -C "$TEST_DOCS" -cf - .corpus-cache/pdf .corpus-cache/ground_truth/pdf | zstd -19 -T0 -f -o "$TARBALL"

echo "Uploading $(du -h "$TARBALL" | cut -f1) → ${OBJECT}"
gcloud storage cp "$TARBALL" "$OBJECT"
echo "✓ Published reference corpus for test_documents ${SHA:0:12}."
