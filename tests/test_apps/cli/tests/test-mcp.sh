#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "===== Kreuzberg CLI MCP Server Test ====="
echo

if ! command -v kreuzberg &>/dev/null; then
	echo -e "${RED}✗ kreuzberg not found. Run ./tests/install.sh first.${NC}"
	exit 1
fi

PASSED=0
FAILED=0

# shellcheck disable=SC2329  # Function is invoked indirectly via trap
cleanup() {
	if [ -n "${SERVER_PID:-}" ]; then
		echo "Stopping MCP server (PID: $SERVER_PID)..."
		kill "$SERVER_PID" 2>/dev/null || true
		wait "$SERVER_PID" 2>/dev/null || true
	fi
}

trap cleanup EXIT

echo "Starting MCP server..."
kreuzberg mcp >/tmp/kreuzberg-mcp.log 2>&1 &
SERVER_PID=$!

echo "Waiting for MCP server to start..."
sleep 2

if ! kill -0 "$SERVER_PID" 2>/dev/null; then
	echo -e "${RED}✗ MCP server failed to start${NC}"
	cat /tmp/kreuzberg-mcp.log
	exit 1
fi

echo -e "${GREEN}✓ MCP server started successfully (PID: $SERVER_PID)${NC}"
((PASSED++))

echo "Checking if MCP server is responsive..."
if ps -p "$SERVER_PID" >/dev/null; then
	echo -e "${GREEN}✓ MCP server is running${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ MCP server stopped unexpectedly${NC}"
	((FAILED++))
fi

echo
echo "===== Test Results ====="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
	echo -e "${GREEN}===== CLI MCP Server Test PASSED =====${NC}"
	exit 0
else
	echo -e "${RED}===== CLI MCP Server Test FAILED =====${NC}"
	exit 1
fi
