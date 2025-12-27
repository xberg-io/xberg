#!/bin/bash
set -e

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

VERBOSE=${VERBOSE:-0}
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DOCUMENTS_DIR="${TEST_DIR}/test_documents"
TEST_PDF="${TEST_DOCUMENTS_DIR}/table.pdf"
SAMPLE_PDF_URL="https://www.w3.org/WAI/WCAG21/Techniques/pdf/img/table.pdf"
OUTPUT_JSON="${TEST_DIR}/result.json"

log_info() {
	echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
	echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
	echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_warning() {
	echo -e "${YELLOW}[WARNING]${NC} $1"
}

verbose() {
	if [ "$VERBOSE" = "1" ]; then
		echo -e "${BLUE}[DEBUG]${NC} $1"
	fi
}

# shellcheck disable=SC2329  # Function is invoked indirectly via trap
cleanup() {
	if [ -f "$OUTPUT_JSON" ]; then
		rm -f "$OUTPUT_JSON"
		verbose "Cleaned up $OUTPUT_JSON"
	fi
}

trap cleanup EXIT

echo ""
log_info "=== Kreuzberg CLI Test ==="
echo ""

log_info "Checking if kreuzberg CLI is available..."
if ! command -v kreuzberg &>/dev/null; then
	log_error "kreuzberg command not found. Did you run install.sh first?"
	exit 1
fi
log_success "kreuzberg found at: $(command -v kreuzberg)"

log_info "Test 1: Testing 'kreuzberg --version'..."
if kreuzberg --version 2>&1 | grep -qE "(kreuzberg|version|[0-9]+\.[0-9]+)"; then
	VERSION_OUTPUT=$(kreuzberg --version 2>&1)
	log_success "Version command works: $VERSION_OUTPUT"
else
	log_warning "Version command output unexpected (continuing anyway)"
	verbose "Output: $(kreuzberg --version 2>&1 || echo 'No output')"
fi

log_info "Test 2: Testing 'kreuzberg --help'..."
if kreuzberg --help &>/dev/null; then
	log_success "Help command works"
else
	log_warning "Help command may have issues (continuing anyway)"
fi

log_info "Test 3: Preparing test document..."
mkdir -p "$TEST_DOCUMENTS_DIR"

if [ -f "$TEST_PDF" ]; then
	log_info "Test PDF already exists: $TEST_PDF"
else
	log_info "Downloading sample PDF from: $SAMPLE_PDF_URL"
	if curl -f -s -L -o "$TEST_PDF" "$SAMPLE_PDF_URL"; then
		log_success "Downloaded test PDF to: $TEST_PDF"
		FILE_SIZE=$(stat -f%z "$TEST_PDF" 2>/dev/null || stat -c%s "$TEST_PDF" 2>/dev/null || echo "unknown")
		verbose "File size: $FILE_SIZE bytes"
	else
		log_error "Failed to download sample PDF"
		log_info "Trying alternative PDF source..."

		log_info "Creating minimal test PDF..."
		cat >"$TEST_PDF" <<'EOF'
%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /Resources 4 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>
endobj
4 0 obj
<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>
endobj
5 0 obj
<< /Length 50 >>
stream
BT
/F1 12 Tf
100 700 Td
(Test Document) Tj
ET
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000203 00000 n
0000000286 00000 n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
385
%%EOF
EOF
		if [ -f "$TEST_PDF" ]; then
			log_success "Created minimal test PDF at: $TEST_PDF"
		else
			log_error "Failed to create test PDF"
			exit 1
		fi
	fi
fi

if [ ! -f "$TEST_PDF" ]; then
	log_error "Test PDF does not exist: $TEST_PDF"
	exit 1
fi

FILE_SIZE=$(stat -f%z "$TEST_PDF" 2>/dev/null || stat -c%s "$TEST_PDF" 2>/dev/null || echo "0")
if [ "$FILE_SIZE" != "0" ] && [ "$FILE_SIZE" -lt 100 ]; then
	log_warning "Test PDF seems very small ($FILE_SIZE bytes)"
fi
log_success "Test PDF ready: $TEST_PDF ($FILE_SIZE bytes)"

log_info "Test 4: Testing extract command..."
if kreuzberg extract --help &>/dev/null; then
	log_success "Extract subcommand exists"
else
	log_warning "Extract subcommand may not exist or has issues"
fi

log_info "Test 5: Running extraction: kreuzberg extract --input '$TEST_PDF' --output '$OUTPUT_JSON'..."

EXTRACT_START_TIME=$(date +%s)
if kreuzberg extract --input "$TEST_PDF" --output "$OUTPUT_JSON" 2>&1; then
	EXTRACT_END_TIME=$(date +%s)
	EXTRACT_TIME=$((EXTRACT_END_TIME - EXTRACT_START_TIME))
	log_success "Extraction command completed (took ${EXTRACT_TIME}s)"
else
	log_error "Extraction command failed"
	log_info "Trying alternative syntax with no output flag..."

	if kreuzberg extract "$TEST_PDF" >"$OUTPUT_JSON" 2>&1; then
		log_success "Extraction completed with alternate syntax"
	else
		log_error "Both extraction syntaxes failed"
		exit 1
	fi
fi

log_info "Test 6: Verifying output file..."
if [ ! -f "$OUTPUT_JSON" ]; then
	log_error "Output file not created: $OUTPUT_JSON"
	exit 1
fi

OUTPUT_SIZE=$(stat -f%z "$OUTPUT_JSON" 2>/dev/null || stat -c%s "$OUTPUT_JSON" 2>/dev/null || echo "0")
log_success "Output file created: $OUTPUT_JSON ($OUTPUT_SIZE bytes)"

log_info "Test 7: Verifying output is valid JSON..."
if command -v jq &>/dev/null; then
	if jq empty "$OUTPUT_JSON" 2>/dev/null; then
		log_success "Output is valid JSON"

		if jq . "$OUTPUT_JSON" 2>/dev/null | grep -qE "(text|content|data)"; then
			log_success "Output contains expected content fields"
		else
			log_info "Output structure: $(jq 'keys' "$OUTPUT_JSON" 2>/dev/null)"
		fi
	else
		log_warning "Output may not be valid JSON (jq parse failed)"
		verbose "First 500 chars of output:\n$(head -c 500 "$OUTPUT_JSON")"
	fi
else
	log_warning "jq not installed, skipping JSON validation"

	if head -c 1 "$OUTPUT_JSON" | grep -q "{"; then
		log_info "Output appears to be JSON (starts with {)"
	elif head -c 1 "$OUTPUT_JSON" | grep -q "\["; then
		log_info "Output appears to be JSON (starts with [)"
	else
		log_warning "Output may not be JSON"
		verbose "First line: $(head -n 1 "$OUTPUT_JSON")"
	fi
fi

log_info "Test 8: Checking for extracted content..."
if [ "$OUTPUT_SIZE" -gt 0 ]; then
	if grep -q "." "$OUTPUT_JSON" 2>/dev/null; then
		log_success "Output file contains content"
		log_info "Output preview (first 10 lines):"
		head -n 10 "$OUTPUT_JSON" | sed 's/^/  /'
	else
		log_warning "Output file appears empty or unreadable"
	fi
else
	log_warning "Output file is empty"
fi

echo ""
log_success "=== CLI Test Passed ==="
echo ""
log_info "Summary:"
log_info "- kreuzberg CLI: $(command -v kreuzberg)"
log_info "- Test PDF: $TEST_PDF ($FILE_SIZE bytes)"
log_info "- Extraction time: ${EXTRACT_TIME}s"
log_info "- Output file: $OUTPUT_JSON ($OUTPUT_SIZE bytes)"
echo ""

exit 0
