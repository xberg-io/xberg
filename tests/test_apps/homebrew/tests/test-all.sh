#!/bin/bash

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

VERBOSE=${VERBOSE:-0}
TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORT_FILE="${TEST_DIR%/tests}/../test_report.txt"

TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
TOTAL_TIME=0

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

run_test() {
	local test_name=$1
	local test_script=$2
	local start_time
	local end_time
	local duration
	start_time=$(date +%s)

	echo ""
	log_info "=========================================="
	log_info "Running Test: $test_name"
	log_info "=========================================="
	echo ""

	if [ ! -f "$test_script" ]; then
		log_error "Test script not found: $test_script"
		TESTS_SKIPPED=$((TESTS_SKIPPED + 1))
		echo "$test_name - SKIPPED (script not found)" >>"$REPORT_FILE"
		return 1
	fi

	if [ ! -x "$test_script" ]; then
		log_warning "Test script not executable, making it executable: $test_script"
		chmod +x "$test_script"
	fi

	if bash "$test_script"; then
		end_time=$(date +%s)
		duration=$((end_time - start_time))
		TESTS_PASSED=$((TESTS_PASSED + 1))
		TOTAL_TIME=$((TOTAL_TIME + duration))

		log_success "$test_name passed (${duration}s)"
		echo "$test_name - PASSED (${duration}s)" >>"$REPORT_FILE"
		return 0
	else
		end_time=$(date +%s)
		duration=$((end_time - start_time))
		TESTS_FAILED=$((TESTS_FAILED + 1))
		TOTAL_TIME=$((TOTAL_TIME + duration))

		log_error "$test_name failed (${duration}s)"
		echo "$test_name - FAILED (${duration}s)" >>"$REPORT_FILE"
		return 1
	fi
}

echo ""
echo "=============================================="
log_info "Kreuzberg Homebrew Test Suite"
echo "=============================================="
echo ""

cat >"$REPORT_FILE" <<EOF
Kreuzberg Homebrew Test Report
Generated: $(date)
======================================

EOF

log_info "Report file: $REPORT_FILE"
log_info "Verbose mode: $VERBOSE"
echo ""

log_info "Phase 1: Installation"
if run_test "Install from Homebrew" "${TEST_DIR}/install.sh"; then
	:
else
	log_error "Installation test failed. Remaining tests cannot continue."
	TESTS_FAILED=$((TESTS_FAILED + 1))

	cat >>"$REPORT_FILE" <<EOF

SUMMARY
======================================
Total Tests: 1
Passed: 0
Failed: 1
Skipped: 0
Total Time: 0s

RESULT: FAILED
Installation failed, remaining tests skipped.
EOF

	cat "$REPORT_FILE"
	exit 1
fi

log_info "Phase 2: CLI Functionality"
run_test "CLI Commands and Extraction" "${TEST_DIR}/test-cli.sh" || {
	log_warning "CLI test failed, but continuing with remaining tests..."
}

log_info "Phase 3: API Server"
run_test "API Server and HTTP Endpoints" "${TEST_DIR}/test-api.sh" || {
	log_warning "API test failed, but continuing with remaining tests..."
}

log_info "Phase 4: MCP Server"
run_test "MCP Server Protocol" "${TEST_DIR}/test-mcp.sh" || {
	log_warning "MCP test failed, but continuing with remaining tests..."
}

echo ""
echo "=============================================="
log_info "Test Suite Complete"
echo "=============================================="
echo ""

TOTAL_TESTS=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))
PASS_RATE=0
if [ $TOTAL_TESTS -gt 0 ]; then
	PASS_RATE=$((TESTS_PASSED * 100 / TOTAL_TESTS))
fi

log_info "Test Summary:"
echo "  Total Tests:    $TOTAL_TESTS"
echo "  Passed:         $TESTS_PASSED"
echo "  Failed:         $TESTS_FAILED"
echo "  Skipped:        $TESTS_SKIPPED"
echo "  Pass Rate:      ${PASS_RATE}%"
echo "  Total Time:     ${TOTAL_TIME}s"
echo ""

cat >>"$REPORT_FILE" <<EOF

SUMMARY
======================================
Total Tests: $TOTAL_TESTS
Passed: $TESTS_PASSED
Failed: $TESTS_FAILED
Skipped: $TESTS_SKIPPED
Pass Rate: ${PASS_RATE}%
Total Time: ${TOTAL_TIME}s

EOF

if [ "$TESTS_FAILED" -eq 0 ]; then
	if [ "$TOTAL_TESTS" -gt 0 ]; then
		log_success "All tests passed!"
		echo "RESULT: PASSED" >>"$REPORT_FILE"
		OVERALL_RESULT=0
	else
		log_warning "No tests were run"
		echo "RESULT: SKIPPED" >>"$REPORT_FILE"
		OVERALL_RESULT=1
	fi
else
	log_error "$TESTS_FAILED test(s) failed"
	echo "RESULT: FAILED" >>"$REPORT_FILE"
	OVERALL_RESULT=1
fi

echo ""
log_info "Full report saved to: $REPORT_FILE"
log_info "Report preview:"
echo ""
tail -20 "$REPORT_FILE" | sed 's/^/  /'
echo ""

exit "$OVERALL_RESULT"
