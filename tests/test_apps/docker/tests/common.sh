#!/bin/bash

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

log_info() {
	echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
	echo -e "${GREEN}[PASS]${NC} $1"
	((TESTS_PASSED++))
}

log_fail() {
	echo -e "${RED}[FAIL]${NC} $1"
	((TESTS_FAILED++))
}

log_skip() {
	echo -e "${YELLOW}[SKIP]${NC} $1"
	((TESTS_SKIPPED++))
}

log_warn() {
	echo -e "${YELLOW}[WARN]${NC} $1"
}

assert_exit_code() {
	local actual=$1
	local expected=${2:-0}
	local message=$3

	if [ "$actual" -eq "$expected" ]; then
		log_success "$message (exit code: $actual)"
		return 0
	else
		log_fail "$message (expected: $expected, got: $actual)"
		return 1
	fi
}

assert_contains() {
	local haystack=$1
	local needle=$2
	local message=$3

	if echo "$haystack" | grep -q "$needle"; then
		log_success "$message"
		return 0
	else
		log_fail "$message (needle: '$needle' not found)"
		return 1
	fi
}

assert_equals() {
	local actual=$1
	local expected=$2
	local message=$3

	if [ "$actual" = "$expected" ]; then
		log_success "$message"
		return 0
	else
		log_fail "$message (expected: '$expected', got: '$actual')"
		return 1
	fi
}

assert_http_status() {
	local url=$1
	local expected_code=${2:-200}
	local message=$3
	local response
	response=$(curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null)

	if [ "$response" = "$expected_code" ]; then
		log_success "$message (status: $response)"
		return 0
	else
		log_fail "$message (expected: $expected_code, got: $response)"
		return 1
	fi
}

wait_for_container() {
	local container=$1
	local timeout=${2:-60}
	local elapsed=0
	local health

	log_info "Waiting for container '$container' to be healthy..."

	while [ "$elapsed" -lt "$timeout" ]; do
		health=$(docker inspect --format='{{.State.Health.Status}}' "$container" 2>/dev/null)

		if [ "$health" = "healthy" ]; then
			log_success "Container '$container' is healthy"
			return 0
		fi

		sleep 1
		((elapsed++))
	done

	log_fail "Container '$container' failed to become healthy within ${timeout}s"
	return 1
}

print_summary() {
	local total=$((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))

	echo ""
	echo "========================================"
	echo "Test Summary"
	echo "========================================"
	echo -e "Total:   $total"
	echo -e "Passed:  ${GREEN}$TESTS_PASSED${NC}"
	echo -e "Failed:  ${RED}$TESTS_FAILED${NC}"
	echo -e "Skipped: ${YELLOW}$TESTS_SKIPPED${NC}"
	echo "========================================"

	if [ "$TESTS_FAILED" -eq 0 ]; then
		echo -e "${GREEN}All tests passed!${NC}"
		return 0
	else
		echo -e "${RED}Some tests failed!${NC}"
		return 1
	fi
}

container_exists() {
	local container=$1
	docker inspect "$container" >/dev/null 2>&1
}

container_is_running() {
	local container=$1
	[ "$(docker inspect -f '{{.State.Running}}' "$container" 2>/dev/null)" = "true" ]
}

call_api() {
	local endpoint=$1
	local base_url=${2:-"http://localhost:8000"}
	local method=${3:-"GET"}
	local data=${4:-""}

	if [ -z "$data" ]; then
		curl -s -X "$method" "$base_url$endpoint"
	else
		curl -s -X "$method" \
			-H "Content-Type: application/json" \
			-d "$data" \
			"$base_url$endpoint"
	fi
}

get_container_ip() {
	local container=$1
	docker inspect --format='{{.NetworkSettings.IPAddress}}' "$container" 2>/dev/null
}

export -f log_info log_success log_fail log_skip log_warn
export -f assert_exit_code assert_contains assert_equals assert_http_status
export -f wait_for_container print_summary container_exists container_is_running
export -f call_api get_container_ip
