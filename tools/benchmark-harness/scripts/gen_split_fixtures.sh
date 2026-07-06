#!/usr/bin/env bash
# Generate synthesized multi-document split fixtures by concatenating single-doc
# PDFs from the `test_documents` submodule. Boundaries are known exactly from
# each source's page count, so ground truth needs no manual labeling.
#
# Output: tools/benchmark-harness/fixtures/split/<name>.pdf + <name>.split.json
#
# Reproducible: re-running regenerates byte-identical manifests (pdfunite output
# may differ trivially; commit the PDFs once). Requires `pdfunite` (poppler) and
# `pdfinfo`.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"        # tools/benchmark-harness
SRC="$HERE/../../test_documents/pdf"                            # submodule PDFs
OUT="$HERE/fixtures/split"
mkdir -p "$OUT"

command -v pdfunite >/dev/null || { echo "pdfunite (poppler) required" >&2; exit 1; }
command -v pdfinfo  >/dev/null || { echo "pdfinfo (poppler) required"  >&2; exit 1; }

pagecount() { pdfinfo "$1" | awk '/^Pages:/{print $2}'; }

# Build one fixture: name followed by source PDF basenames (without .pdf).
# Emits <name>.pdf and <name>.split.json with cumulative inclusive boundaries.
make_fixture() {
  local name="$1"; shift
  local -a srcs=("$@")
  local -a inputs=()
  local -a bounds=()
  local start=1
  for s in "${srcs[@]}"; do
    local f="$SRC/$s.pdf"
    [ -f "$f" ] || { echo "missing source: $f" >&2; exit 1; }
    inputs+=("$f")
    local n; n="$(pagecount "$f")"
    local end=$((start + n - 1))
    bounds+=("{\"start_page\":$start,\"end_page\":$end}")
    start=$((end + 1))
  done
  pdfunite "${inputs[@]}" "$OUT/$name.pdf"
  local joined; joined="$(IFS=,; echo "${bounds[*]}")"
  printf '{\n  "document": "%s.pdf",\n  "boundaries": [%s]\n}\n' "$name" "$joined" > "$OUT/$name.split.json"
  echo "  $name.pdf  (${#srcs[@]} segments, $((start - 1)) pages)"
}

echo "Generating split fixtures into $OUT"
# Short-doc seams (letterhead resets between one-page memos/forms).
make_fixture memo_form_memo        fake_memo flattened_form fake_memo
make_fixture forms_batch           flattened_form interactive_form google_doc_document
# Density shifts (memo/marketing prose vs dense paper).
make_fixture memo_paper            fake_memo nougat_002
make_fixture marketing_memo        multipage_marketing fake_memo
make_fixture memo_marketing_form   fake_memo multipage_marketing interactive_form
# Mixed: paper + short docs.
make_fixture paper_memo_form       code_and_formula fake_memo flattened_form
# Hard case: two dense papers back-to-back (weak density signal).
make_fixture two_papers            nougat_002 code_and_formula
# Negative control: a single cohesive paper — Auto must NOT split it.
make_fixture single_paper          nougat_002

echo "Done."
