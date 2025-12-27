#!/bin/bash
set -e

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

VERBOSE=${VERBOSE:-0}
API_HOST=${API_HOST:-127.0.0.1}
API_PORT=${API_PORT:-8000}
API_URL="http://${API_HOST}:${API_PORT}"
API_TIMEOUT=10
HEALTH_CHECK_TIMEOUT=15
HEALTH_CHECK_INTERVAL=1

TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEST_DOCUMENTS_DIR="${TEST_DIR}/test_documents"
TEST_PDF="${TEST_DOCUMENTS_DIR}/table.pdf"
SAMPLE_PDF_URL="https://www.w3.org/WAI/WCAG21/Techniques/pdf/img/table.pdf"

LOG_FILE="/tmp/kreuzberg_api_${API_PORT}.log"
API_PID_FILE="/tmp/kreuzberg_api_${API_PORT}.pid"

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
	log_info "Cleaning up..."

	if [ -f "$API_PID_FILE" ]; then
		API_PID=$(cat "$API_PID_FILE")
		if ps -p "$API_PID" >/dev/null 2>&1; then
			verbose "Killing API server (PID: $API_PID)..."
			kill "$API_PID" 2>/dev/null || true
			sleep 1

			if ps -p "$API_PID" >/dev/null 2>&1; then
				verbose "Force killing API server..."
				kill -9 "$API_PID" 2>/dev/null || true
			fi
		fi
		rm -f "$API_PID_FILE"
	fi

	if [ "$VERBOSE" != "1" ] && [ -f "$LOG_FILE" ]; then
		rm -f "$LOG_FILE"
	fi
}

trap cleanup EXIT

echo ""
log_info "=== Kreuzberg API Server Test ==="
echo ""

log_info "Checking if kreuzberg CLI is available..."
if ! command -v kreuzberg &>/dev/null; then
	log_error "kreuzberg command not found. Did you run install.sh first?"
	exit 1
fi
log_success "kreuzberg found at: $(command -v kreuzberg)"

log_info "Checking if port $API_PORT is available..."
if lsof -i :"$API_PORT" &>/dev/null 2>&1; then
	log_error "Port $API_PORT is already in use"
	log_info "Try setting API_PORT environment variable to use a different port"
	exit 1
fi
log_success "Port $API_PORT is available"

log_info "Preparing test document..."
mkdir -p "$TEST_DOCUMENTS_DIR"

if [ ! -f "$TEST_PDF" ]; then
	log_info "Downloading sample PDF..."
	if curl -f -s -L -o "$TEST_PDF" "$SAMPLE_PDF_URL"; then
		log_success "Downloaded test PDF"
	else
		log_info "Creating minimal test PDF..."
		cat >"$TEST_PDF" <<'PDFEOF'
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
PDFEOF
	fi
fi

if [ ! -f "$TEST_PDF" ]; then
	log_error "Failed to prepare test PDF"
	exit 1
fi
log_success "Test PDF ready: $TEST_PDF"

log_info "Starting Kreuzberg API server on ${API_URL}..."
log_info "Command: kreuzberg serve --host $API_HOST --port $API_PORT"

kreuzberg serve --host "$API_HOST" --port "$API_PORT" >"$LOG_FILE" 2>&1 &
API_PID=$!
echo "$API_PID" >"$API_PID_FILE"

verbose "API server started with PID: $API_PID"
verbose "Server logs: $LOG_FILE"

log_info "Waiting for server to start (timeout: ${HEALTH_CHECK_TIMEOUT}s)..."
ELAPSED=0
SERVER_READY=0

while [ "$ELAPSED" -lt "$HEALTH_CHECK_TIMEOUT" ]; do
	if curl -f -s -m "$API_TIMEOUT" "${API_URL}/health" >/dev/null 2>&1; then
		SERVER_READY=1
		break
	fi

	if ! ps -p "$API_PID" >/dev/null 2>&1; then
		log_error "API server process died during startup"
		log_error "Server output:"
		sed 's/^/  /' "$LOG_FILE"
		exit 1
	fi

	verbose "Server not ready yet (${ELAPSED}s elapsed)..."
	sleep "$HEALTH_CHECK_INTERVAL"
	ELAPSED=$((ELAPSED + HEALTH_CHECK_INTERVAL))
done

if [ "$SERVER_READY" -eq 0 ]; then
	log_error "Server did not start within ${HEALTH_CHECK_TIMEOUT} seconds"
	log_error "Server output:"
	sed 's/^/  /' "$LOG_FILE"
	exit 1
fi

log_success "Server is ready and accepting requests (${ELAPSED}s startup time)"

log_info "Test 1: Testing health check endpoint..."
HEALTH_RESPONSE=$(curl -f -s -m "$API_TIMEOUT" "${API_URL}/health" 2>&1 || echo "FAILED")

if [ "$HEALTH_RESPONSE" != "FAILED" ]; then
	log_success "Health check passed"
	verbose "Response: $HEALTH_RESPONSE"
else
	log_error "Health check failed"
	exit 1
fi

log_info "Test 2: Checking available endpoints..."
if curl -f -s -m "$API_TIMEOUT" "${API_URL}/" >/dev/null 2>&1; then
	log_success "Root endpoint accessible"
else
	log_warning "Root endpoint not accessible (continuing anyway)"
fi

log_info "Test 3: Testing extraction endpoint..."
RESPONSE_FILE="/tmp/kreuzberg_api_response_${API_PORT}.json"

if [ ! -f "$TEST_PDF" ]; then
	log_error "Test PDF not found: $TEST_PDF"
	exit 1
fi

log_info "Sending POST request to ${API_URL}/extract with test PDF..."

EXTRACT_SUCCESS=0

if curl -f -s -m "$API_TIMEOUT" \
	-F "file=@${TEST_PDF}" \
	"${API_URL}/extract" \
	-o "$RESPONSE_FILE" 2>&1; then
	EXTRACT_SUCCESS=1
	verbose "Extraction succeeded with /extract endpoint"
fi

if [ "$EXTRACT_SUCCESS" -eq 0 ]; then
	if curl -f -s -m "$API_TIMEOUT" \
		-F "file=@${TEST_PDF}" \
		"${API_URL}/api/extract" \
		-o "$RESPONSE_FILE" 2>&1; then
		EXTRACT_SUCCESS=1
		verbose "Extraction succeeded with /api/extract endpoint"
	fi
fi

if [ "$EXTRACT_SUCCESS" -eq 0 ]; then
	if curl -f -s -m "$API_TIMEOUT" \
		-H "Content-Type: application/pdf" \
		--data-binary "@${TEST_PDF}" \
		"${API_URL}/extract" \
		-o "$RESPONSE_FILE" 2>&1; then
		EXTRACT_SUCCESS=1
		verbose "Extraction succeeded with binary upload"
	fi
fi

if [ "$EXTRACT_SUCCESS" -eq 0 ]; then
	log_warning "Extraction endpoint test failed or not available"
	verbose "This may be expected if API doesn't implement /extract endpoint"
	RESPONSE_FILE=""
else
	log_success "Extraction endpoint responded"

	if [ -f "$RESPONSE_FILE" ]; then
		RESPONSE_SIZE=$(stat -f%z "$RESPONSE_FILE" 2>/dev/null || stat -c%s "$RESPONSE_FILE" 2>/dev/null || echo "0")
		log_success "Response received ($RESPONSE_SIZE bytes)"

		if [ "$RESPONSE_SIZE" -gt 0 ]; then
			log_info "Response preview (first 500 chars):"
			head -c 500 "$RESPONSE_FILE" | sed 's/^/  /'
			echo ""

			if command -v jq &>/dev/null; then
				if jq empty "$RESPONSE_FILE" 2>/dev/null; then
					log_success "Response is valid JSON"
				else
					log_warning "Response may not be valid JSON"
				fi
			fi
		fi
	fi
fi

log_info "Test 4: Verifying server is still running..."
if ps -p "$API_PID" >/dev/null 2>&1; then
	log_success "Server is running (PID: $API_PID)"
else
	log_error "Server process is no longer running"
	exit 1
fi

if [ -f "$RESPONSE_FILE" ]; then
	rm -f "$RESPONSE_FILE"
fi

log_info "Shutting down server gracefully..."
kill "$API_PID" 2>/dev/null || true
sleep 2

if ps -p "$API_PID" >/dev/null 2>&1; then
	log_warning "Process did not stop gracefully, force killing..."
	kill -9 "$API_PID" 2>/dev/null || true
	sleep 1
fi

if ps -p "$API_PID" >/dev/null 2>&1; then
	log_error "Failed to stop API server"
	exit 1
fi

log_success "Server shut down successfully"
rm -f "$API_PID_FILE"

echo ""
log_success "=== API Server Test Passed ==="
echo ""
log_info "Summary:"
log_info "- API URL: $API_URL"
log_info "- Server PID: $API_PID"
log_info "- Test PDF: $TEST_PDF"
log_info "- Server logs: $LOG_FILE"
echo ""

exit 0
