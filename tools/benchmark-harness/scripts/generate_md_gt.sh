#!/usr/bin/env bash
# Generate markdown and text ground truth for docbook, typst, and fictionbook formats
# using pandoc + sanitize_pandoc_gt.py, then create benchmark fixture JSON files.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SANITIZE="$REPO_ROOT/tools/benchmark-harness/scripts/sanitize_pandoc_gt.py"
FIXTURES_DIR="$REPO_ROOT/tools/benchmark-harness/fixtures"

cd "$REPO_ROOT"

echo "=== Step 1: Generate MD ground truth via pandoc + sanitize ==="

# --- DocBook ---
echo "--- DocBook ---"
for f in test_documents/docbook/*.dbk test_documents/docbook/*.docbook test_documents/docbook/*.docbook4 test_documents/docbook/*.docbook5; do
  [ -f "$f" ] || continue
  name=$(basename "$f" | sed 's/\.[^.]*$//')
  mkdir -p test_documents/ground_truth/docbook
  pandoc -f docbook -t gfm --wrap=none "$f" 2>/dev/null | python3 "$SANITIZE" >"test_documents/ground_truth/docbook/${name}.md"
  echo "docbook: $name ($(wc -c <"test_documents/ground_truth/docbook/${name}.md") bytes)"
done

# --- Typst ---
echo "--- Typst ---"
for f in test_documents/typst/*.typ; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .typ)
  # Typst GT goes in both typ/ (matching existing convention) and typst/
  for gtdir in test_documents/ground_truth/typ test_documents/ground_truth/typst; do
    mkdir -p "$gtdir"
    pandoc -f typst -t gfm --wrap=none "$f" 2>/dev/null | python3 "$SANITIZE" >"${gtdir}/${name}.md"
  done
  echo "typst: $name ($(wc -c <"test_documents/ground_truth/typ/${name}.md") bytes)"
done

# --- FictionBook (fb2) ---
echo "--- FictionBook ---"
for f in test_documents/fictionbook/*.fb2; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .fb2)
  mkdir -p test_documents/ground_truth/fb2
  existing="test_documents/ground_truth/fb2/${name}.md"
  if [ ! -f "$existing" ]; then
    pandoc -f fb2 -t gfm --wrap=none "$f" 2>/dev/null | python3 "$SANITIZE" >"$existing"
    echo "fb2: $name (new, $(wc -c <"$existing") bytes)"
  else
    echo "fb2: $name (exists, $(wc -c <"$existing") bytes)"
  fi
done

echo ""
echo "=== Step 2: Generate text GT from MD GT ==="

# For each .md GT file, generate .txt if missing
for md_file in test_documents/ground_truth/docbook/*.md test_documents/ground_truth/typ/*.md test_documents/ground_truth/fb2/*.md; do
  [ -f "$md_file" ] || continue
  txt_file="${md_file%.md}.txt"
  if [ ! -f "$txt_file" ]; then
    pandoc -f gfm -t plain --wrap=none "$md_file" >"$txt_file"
    echo "text: $(basename "$txt_file") (new, $(wc -c <"$txt_file") bytes)"
  fi
done

echo ""
echo "=== Step 3: Create fixture JSON files ==="

# Helper to create fixture JSON
create_fixture() {
  local doc_path="$1"
  local file_type="$2"
  local gt_text="$3"
  local gt_md="$4"
  local fixture_out="$5"
  local description="$6"
  local category="$7"

  local file_size
  file_size=$(stat -f %z "$doc_path" 2>/dev/null || wc -c <"$doc_path" | tr -d ' ')

  local name
  name=$(basename "$doc_path" | sed 's/\.[^.]*$//')

  # Compute relative paths from fixtures dir
  local rel_doc="../../../${doc_path}"
  local rel_text="../../../${gt_text}"
  local rel_md="../../../${gt_md}"

  local json
  if [ -f "$gt_md" ] && [ -f "$gt_text" ]; then
    json=$(
      cat <<EOJSON
{
	"document": "${rel_doc}",
	"file_type": "${file_type}",
	"file_size": ${file_size},
	"expected_frameworks": ["kreuzberg"],
	"metadata": {
		"description": "${description}",
		"category": "${category}"
	},
	"ground_truth": {
		"text_file": "${rel_text}",
		"markdown_file": "${rel_md}",
		"source": "pandoc"
	}
}
EOJSON
    )
  elif [ -f "$gt_text" ]; then
    json=$(
      cat <<EOJSON
{
	"document": "${rel_doc}",
	"file_type": "${file_type}",
	"file_size": ${file_size},
	"expected_frameworks": ["kreuzberg"],
	"metadata": {
		"description": "${description}",
		"category": "${category}"
	},
	"ground_truth": {
		"text_file": "${rel_text}",
		"source": "pandoc"
	}
}
EOJSON
    )
  fi

  echo "$json" >"$fixture_out"
  echo "fixture: $(basename "$fixture_out")"
}

# --- DocBook fixtures ---
echo "--- DocBook fixtures ---"
for f in test_documents/docbook/*.dbk test_documents/docbook/*.docbook test_documents/docbook/*.docbook4 test_documents/docbook/*.docbook5; do
  [ -f "$f" ] || continue
  name=$(basename "$f" | sed 's/\.[^.]*$//')
  ext=$(basename "$f" | sed 's/.*\.//')
  gt_md="test_documents/ground_truth/docbook/${name}.md"
  gt_txt="test_documents/ground_truth/docbook/${name}.txt"

  # Determine file_type based on extension
  case "$ext" in
  dbk) ft="dbk" ;;
  docbook | docbook4 | docbook5) ft="docbook" ;;
  *) ft="docbook" ;;
  esac

  fixture_name="docbook_$(echo "$name" | tr '-' '_').json"
  create_fixture "$f" "$ft" "$gt_txt" "$gt_md" "${FIXTURES_DIR}/${fixture_name}" "DocBook document: ${name}" "docbook"
done

# --- Typst fixtures (update existing to add markdown_file) ---
echo "--- Typst fixtures ---"
for f in test_documents/typst/*.typ; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .typ)
  gt_md="test_documents/ground_truth/typ/${name}.md"
  gt_txt="test_documents/ground_truth/typ/typst_${name}.txt"
  # Some txt files use name directly, some use typst_ prefix - check both
  if [ ! -f "$gt_txt" ]; then
    gt_txt="test_documents/ground_truth/typ/${name}.txt"
  fi

  fixture_name="typst_${name}.json"
  create_fixture "$f" "typ" "$gt_txt" "$gt_md" "${FIXTURES_DIR}/${fixture_name}" "Typst document: ${name}" "typst"
done

# --- FictionBook fixtures (update existing to add markdown_file) ---
echo "--- FictionBook fixtures ---"
for f in test_documents/fictionbook/*.fb2; do
  [ -f "$f" ] || continue
  name=$(basename "$f" .fb2)
  gt_md="test_documents/ground_truth/fb2/${name}.md"
  gt_txt="test_documents/ground_truth/fb2/${name}.txt"
  # Some txt files use fb2_ prefix
  if [ ! -f "$gt_txt" ]; then
    gt_txt="test_documents/ground_truth/fb2/fb2_${name}.txt"
  fi

  fixture_name="fb2_${name}.json"
  create_fixture "$f" "fb2" "$gt_txt" "$gt_md" "${FIXTURES_DIR}/${fixture_name}" "FictionBook document: ${name}" "fictionbook"
done

echo ""
echo "=== Step 4: Validate ==="

echo "--- Verifying GT files are non-empty ---"
empty_count=0
for f in test_documents/ground_truth/docbook/*.md test_documents/ground_truth/typ/*.md test_documents/ground_truth/fb2/*.md; do
  [ -f "$f" ] || continue
  size=$(wc -c <"$f" | tr -d ' ')
  if [ "$size" -le 1 ]; then
    echo "WARNING: $f is empty/near-empty ($size bytes)"
    empty_count=$((empty_count + 1))
  fi
done
echo "Empty/near-empty GT files: $empty_count"

echo ""
echo "=== Summary ==="
echo "DocBook MD GT files: $(find test_documents/ground_truth/docbook/*.md -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo "DocBook TXT GT files: $(find test_documents/ground_truth/docbook/*.txt -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo "Typst MD GT files: $(find test_documents/ground_truth/typ/*.md -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo "Typst TXT GT files: $(find test_documents/ground_truth/typ/*.txt -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo "FB2 MD GT files: $(find test_documents/ground_truth/fb2/*.md -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo "FB2 TXT GT files: $(find test_documents/ground_truth/fb2/*.txt -maxdepth 1 2>/dev/null | wc -l | tr -d ' ')"
echo ""
echo "Fixture files created/updated:"
ls -1 "${FIXTURES_DIR}"/docbook_*.json "${FIXTURES_DIR}"/typst_*.json "${FIXTURES_DIR}"/fb2_*.json "${FIXTURES_DIR}"/dbk_*.json 2>/dev/null
