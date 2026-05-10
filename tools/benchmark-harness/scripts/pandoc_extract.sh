#!/usr/bin/env bash

set -euo pipefail

FORMAT="markdown"
FILE_PATH=""
for arg in "$@"; do
  case "$arg" in
    --format=*)
      FORMAT="${arg#--format=}"
      ;;
    *)
      FILE_PATH="$arg"
      ;;
  esac
done

if [ -z "$FILE_PATH" ]; then
  echo "Usage: pandoc_extract.sh [--format=markdown|plaintext] <file_path>" >&2
  exit 1
fi

if [ "$FORMAT" != "markdown" ] && [ "$FORMAT" != "plaintext" ]; then
  echo "Error: --format must be 'markdown' or 'plaintext'; got '$FORMAT'" >&2
  exit 64
fi

if [ ! -f "$FILE_PATH" ]; then
  echo "Error: File not found: $FILE_PATH" >&2
  exit 1
fi

if [ "$FORMAT" = "markdown" ]; then
  PANDOC_TO="gfm"
else
  PANDOC_TO="plain"
fi

START=$(date +%s%N)

if command -v timeout &>/dev/null; then
  CONTENT=$(timeout 60s pandoc "$FILE_PATH" "--to=$PANDOC_TO" --wrap=none --strip-comments 2>/dev/null || echo "")
elif command -v gtimeout &>/dev/null; then
  CONTENT=$(gtimeout 60s pandoc "$FILE_PATH" "--to=$PANDOC_TO" --wrap=none --strip-comments 2>/dev/null || echo "")
else
  CONTENT=$(pandoc "$FILE_PATH" "--to=$PANDOC_TO" --wrap=none --strip-comments 2>/dev/null || echo "")
fi

END=$(date +%s%N)
DURATION_MS=$(((END - START) / 1000000))

if command -v jq &>/dev/null; then
  jq -n \
    --arg content "$CONTENT" \
    --arg fmt "$FORMAT" \
    --argjson duration "$DURATION_MS" \
    '{
            content: $content,
            metadata: {framework: "pandoc", output_format: $fmt},
            _extraction_time_ms: $duration
        }'
else
  ESCAPED_CONTENT=$(echo "$CONTENT" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}' | sed '$ s/\\n$//')
  cat <<EOF
{"content":"$ESCAPED_CONTENT","metadata":{"framework":"pandoc","output_format":"$FORMAT"},"_extraction_time_ms":$DURATION_MS}
EOF
fi
