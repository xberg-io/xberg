#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Kreuzberg CLI Test Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo

TOTAL_PASSED=0
TOTAL_FAILED=0

run_test() {
	local test_name=$1
	local test_script=$2

	echo -e "${BLUE}>>> Running: $test_name${NC}"
	if bash "$SCRIPT_DIR/$test_script"; then
		echo -e "${GREEN}✓ $test_name PASSED${NC}"
		((TOTAL_PASSED++))
	else
		echo -e "${RED}✗ $test_name FAILED${NC}"
		((TOTAL_FAILED++))
	fi
	echo
}

run_test "Installation Test" "install.sh"
run_test "Extraction Test" "test-extract.sh"
run_test "HTTP API Server Test" "test-serve.sh"
run_test "MCP Server Test" "test-mcp.sh"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Test Suite Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total Passed: ${GREEN}$TOTAL_PASSED${NC}"
echo -e "Total Failed: ${RED}$TOTAL_FAILED${NC}"
echo

if [ $TOTAL_FAILED -eq 0 ]; then
	echo -e "${GREEN}===== ALL TESTS PASSED =====${NC}"
	exit 0
else
	echo -e "${RED}===== SOME TESTS FAILED =====${NC}"
	exit 1
fi
