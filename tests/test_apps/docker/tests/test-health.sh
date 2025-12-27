#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "Health Check Tests"
echo "================================${NC}"

echo ""
log_info "Test 1: Core container exists"
if container_exists "kreuzberg-core-test"; then
	log_success "Core container exists"
else
	log_fail "Core container does not exist"
fi

echo ""
log_info "Test 2: Full container exists"
if container_exists "kreuzberg-full-test"; then
	log_success "Full container exists"
else
	log_fail "Full container does not exist"
fi

echo ""
log_info "Test 3: Core container is running"
if container_is_running "kreuzberg-core-test"; then
	log_success "Core container is running"
else
	log_fail "Core container is not running"
fi

echo ""
log_info "Test 4: Full container is running"
if container_is_running "kreuzberg-full-test"; then
	log_success "Full container is running"
else
	log_fail "Full container is not running"
fi

echo ""
log_info "Test 5: Core container becomes healthy"
if wait_for_container "kreuzberg-core-test" 60; then
	:
else
	log_fail "Core container failed health check"
fi

echo ""
log_info "Test 6: Full container becomes healthy"
if wait_for_container "kreuzberg-full-test" 60; then
	:
else
	log_fail "Full container failed health check"
fi

echo ""
log_info "Test 7: Core version command works"
version_output=$(docker exec kreuzberg-core-test kreuzberg --version 2>&1)
if assert_contains "$version_output" "kreuzberg" "Core version command returns kreuzberg"; then
	:
else
	log_fail "Core version command failed"
fi

echo ""
log_info "Test 8: Full version command works"
version_output=$(docker exec kreuzberg-full-test kreuzberg --version 2>&1)
if assert_contains "$version_output" "kreuzberg" "Full version command returns kreuzberg"; then
	:
else
	log_fail "Full version command failed"
fi

echo ""
print_summary
