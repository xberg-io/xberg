#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export CYAN='\033[0;36m'
export NC='\033[0m'

declare -a TEST_SUITES=()
declare -a SUITE_RESULTS=()

echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗"
echo "║     Kreuzberg Docker Image Comprehensive Test Suite      ║"
echo -e "╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

run_test_suite() {
	local suite_name=$1
	local script_path=$2

	TEST_SUITES+=("$suite_name")

	echo -e "${CYAN}Running: $suite_name${NC}"
	echo "Script: $script_path"
	echo ""

	if bash "$script_path"; then
		SUITE_RESULTS+=("PASS")
		echo ""
	else
		SUITE_RESULTS+=("FAIL")
		echo ""
	fi
}

check_prerequisites() {
	echo -e "${BLUE}Checking prerequisites...${NC}"

	if ! command -v docker &>/dev/null; then
		echo -e "${RED}Docker is not installed or not in PATH${NC}"
		exit 1
	fi

	if ! command -v docker-compose &>/dev/null && ! docker compose version &>/dev/null; then
		echo -e "${RED}Docker Compose is not installed or not in PATH${NC}"
		exit 1
	fi

	echo -e "${GREEN}Docker and Docker Compose are available${NC}"
	echo ""
}

check_containers() {
	echo -e "${BLUE}Checking containers...${NC}"

	if ! docker inspect kreuzberg-core-test >/dev/null 2>&1; then
		echo -e "${RED}Core container not running. Please run: docker-compose up${NC}"
		exit 1
	fi

	if ! docker inspect kreuzberg-full-test >/dev/null 2>&1; then
		echo -e "${RED}Full container not running. Please run: docker-compose up${NC}"
		exit 1
	fi

	echo -e "${GREEN}Both containers are running${NC}"
	echo ""
}

run_all_tests() {
	echo -e "${BLUE}Running test suites...${NC}"
	echo ""

	run_test_suite "Health Check Tests" "$SCRIPT_DIR/test-health.sh"
	run_test_suite "CLI Command Tests" "$SCRIPT_DIR/test-cli.sh"
	run_test_suite "API Endpoint Tests" "$SCRIPT_DIR/test-api.sh"
	run_test_suite "MCP Protocol Tests" "$SCRIPT_DIR/test-mcp.sh"
	run_test_suite "OCR Tests" "$SCRIPT_DIR/test-ocr.sh"
	run_test_suite "Embeddings Tests" "$SCRIPT_DIR/test-embeddings.sh"
	run_test_suite "Core Image Tests" "$SCRIPT_DIR/test-core.sh"
	run_test_suite "Full Image Tests" "$SCRIPT_DIR/test-full.sh"
}

print_final_summary() {
	echo ""
	echo -e "${CYAN}╔════════════════════════════════════════════════════════════╗"
	echo "║                    Test Suite Summary                       ║"
	echo -e "╚════════════════════════════════════════════════════════════╝${NC}"
	echo ""

	echo "Test Suites Results:"
	echo ""
	for i in "${!TEST_SUITES[@]}"; do
		suite="${TEST_SUITES[$i]}"
		result="${SUITE_RESULTS[$i]}"

		if [ "$result" = "PASS" ]; then
			echo -e "  ${GREEN}✓${NC} $suite"
		else
			echo -e "  ${RED}✗${NC} $suite"
		fi
	done

	echo ""
	echo "Summary:"
	echo "  Test Suites Run: ${#TEST_SUITES[@]}"

	passed_count=0
	for result in "${SUITE_RESULTS[@]}"; do
		if [ "$result" = "PASS" ]; then
			((passed_count++))
		fi
	done

	failed_count=$((${#TEST_SUITES[@]} - passed_count))

	echo -e "  Passed: ${GREEN}${passed_count}${NC}"
	echo -e "  Failed: ${RED}${failed_count}${NC}"
	echo ""

	if [ $failed_count -eq 0 ]; then
		echo -e "${GREEN}All test suites passed!${NC}"
		return 0
	else
		echo -e "${RED}Some test suites failed!${NC}"
		return 1
	fi
}

main() {
	check_prerequisites
	check_containers
	run_all_tests
	print_final_summary
}

main "$@"
