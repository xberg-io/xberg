#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "===== Kreuzberg CLI MCP Server Test ====="
echo

if ! command -v kreuzberg &>/dev/null; then
  echo -e "${RED}✗ kreuzberg not found. Run ./tests/install.sh first.${NC}"
  exit 1
fi

# Check if 'mcp' subcommand is available (requires --features mcp)
if ! kreuzberg mcp --help >/dev/null 2>&1; then
  echo -e "${YELLOW}⚠ 'mcp' subcommand not available (requires --features mcp). Skipping.${NC}"
  exit 0
fi

PASSED=0
FAILED=0

# MCP uses JSON-RPC over stdio. Send an initialize request and check for a response.
echo "Testing MCP server via stdio..."

# Build a valid JSON-RPC initialize request
INIT_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'

# Send the request to the MCP server via stdin and capture stdout
# The MCP server reads from stdin and writes to stdout
RESPONSE=$(echo "$INIT_REQUEST" | timeout 10 kreuzberg mcp 2>/dev/null || true)

if echo "$RESPONSE" | grep -q '"result"'; then
  echo -e "${GREEN}✓ MCP server responded to initialize request${NC}"
  ((PASSED++))
else
  echo -e "${RED}✗ MCP server did not respond to initialize request${NC}"
  ((FAILED++))
fi

if echo "$RESPONSE" | grep -q '"serverInfo"'; then
  echo -e "${GREEN}✓ MCP server returned serverInfo${NC}"
  ((PASSED++))
else
  echo -e "${RED}✗ MCP server missing serverInfo in response${NC}"
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
