#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_DOCS_DIR="$SCRIPT_DIR/../test_documents"

echo "===== Kreuzberg CLI HTTP API Server Test ====="
echo

if ! command -v kreuzberg &>/dev/null; then
	echo -e "${RED}✗ kreuzberg not found. Run ./tests/install.sh first.${NC}"
	exit 1
fi

PASSED=0
FAILED=0
PORT=38765

# shellcheck disable=SC2329  # Function is invoked indirectly via trap
cleanup() {
	if [ -n "${SERVER_PID:-}" ]; then
		echo "Stopping server (PID: $SERVER_PID)..."
		kill "$SERVER_PID" 2>/dev/null || true
		wait "$SERVER_PID" 2>/dev/null || true
	fi
}

trap cleanup EXIT

echo "Starting HTTP API server on port $PORT..."
kreuzberg serve --port "$PORT" >/tmp/kreuzberg-serve.log 2>&1 &
SERVER_PID=$!

echo "Waiting for server to start..."
sleep 3

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
	echo -e "${RED}✗ Server failed to start${NC}"
	cat /tmp/kreuzberg-serve.log
	exit 1
fi

echo -e "${GREEN}✓ Server started successfully (PID: $SERVER_PID)${NC}"

echo "Testing health endpoint..."
if curl -s -f "http://localhost:$PORT/health" >/dev/null; then
	echo -e "${GREEN}✓ Health endpoint working${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ Health endpoint failed${NC}"
	((FAILED++))
fi

echo "Testing extraction endpoint with PDF..."
if curl -s -f -X POST "http://localhost:$PORT/extract" \
	-F "file=@$TEST_DOCS_DIR/tiny.pdf" |
	grep -q "text"; then
	echo -e "${GREEN}✓ PDF extraction via API successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ PDF extraction via API failed${NC}"
	((FAILED++))
fi

echo "Testing extraction endpoint with DOCX..."
if curl -s -f -X POST "http://localhost:$PORT/extract" \
	-F "file=@$TEST_DOCS_DIR/lorem_ipsum.docx" |
	grep -q "text"; then
	echo -e "${GREEN}✓ DOCX extraction via API successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ DOCX extraction via API failed${NC}"
	((FAILED++))
fi

echo
echo "===== Test Results ====="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
	echo -e "${GREEN}===== CLI HTTP API Server Test PASSED =====${NC}"
	exit 0
else
	echo -e "${RED}===== CLI HTTP API Server Test FAILED =====${NC}"
	exit 1
fi
