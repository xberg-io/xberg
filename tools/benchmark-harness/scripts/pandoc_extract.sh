#!/usr/bin/env bash

set -euo pipefail

if [ $# -ne 1 ]; then
	echo "Usage: pandoc_extract.sh <file_path>" >&2
	exit 1
fi

FILE_PATH="$1"

if [ ! -f "$FILE_PATH" ]; then
	echo "Error: File not found: $FILE_PATH" >&2
	exit 1
fi

START=$(date +%s%N)

CONTENT=$(pandoc "$FILE_PATH" --to=plain --wrap=none --strip-comments 2>/dev/null || echo "")

END=$(date +%s%N)
DURATION_MS=$(((END - START) / 1000000))

if command -v jq &>/dev/null; then
	jq -n \
		--arg content "$CONTENT" \
		--argjson duration "$DURATION_MS" \
		'{
            content: $content,
            metadata: {framework: "pandoc"},
            _extraction_time_ms: $duration
        }'
else
	ESCAPED_CONTENT=$(echo "$CONTENT" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g' | awk '{printf "%s\\n", $0}' | sed '$ s/\\n$//')
	cat <<EOF
{"content":"$ESCAPED_CONTENT","metadata":{"framework":"pandoc"},"_extraction_time_ms":$DURATION_MS}
EOF
fi
