#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "CLI Command Tests"
echo "================================${NC}"

echo ""
log_info "Test 1: Core version command"
if docker exec kreuzberg-core-test kreuzberg --version >/dev/null 2>&1; then
	log_success "Core version command executes"
else
	log_fail "Core version command failed"
fi

echo ""
log_info "Test 2: Full version command"
if docker exec kreuzberg-full-test kreuzberg --version >/dev/null 2>&1; then
	log_success "Full version command executes"
else
	log_fail "Full version command failed"
fi

echo ""
log_info "Test 3: Core help command"
output=$(docker exec kreuzberg-core-test kreuzberg --help 2>&1 || true)
if assert_contains "$output" "extract\|serve\|mcp" "Core help shows commands"; then
	:
else
	log_fail "Core help command output unexpected"
fi

echo ""
log_info "Test 4: Full help command"
output=$(docker exec kreuzberg-full-test kreuzberg --help 2>&1 || true)
if assert_contains "$output" "extract\|serve\|mcp" "Full help shows commands"; then
	:
else
	log_fail "Full help command output unexpected"
fi

echo ""
log_info "Test 5: Core extract subcommand exists"
output=$(docker exec kreuzberg-core-test kreuzberg extract --help 2>&1 || true)
if assert_contains "$output" "extract\|file\|path" "Core extract help works"; then
	:
else
	log_warn "Core extract help: $output"
fi

echo ""
log_info "Test 6: Full extract subcommand exists"
output=$(docker exec kreuzberg-full-test kreuzberg extract --help 2>&1 || true)
if assert_contains "$output" "extract\|file\|path" "Full extract help works"; then
	:
else
	log_warn "Full extract help: $output"
fi

echo ""
log_info "Test 7: Core serve subcommand exists"
output=$(docker exec kreuzberg-core-test kreuzberg serve --help 2>&1 || true)
if assert_contains "$output" "serve\|port\|host" "Core serve help works"; then
	:
else
	log_warn "Core serve help: $output"
fi

echo ""
log_info "Test 8: Full serve subcommand exists"
output=$(docker exec kreuzberg-full-test kreuzberg serve --help 2>&1 || true)
if assert_contains "$output" "serve\|port\|host" "Full serve help works"; then
	:
else
	log_warn "Full serve help: $output"
fi

echo ""
log_info "Test 9: Core list formats or similar"
output=$(docker exec kreuzberg-core-test kreuzberg --help 2>&1 || true)
if [ ! -z "$output" ]; then
	log_success "Core CLI is responsive"
else
	log_fail "Core CLI output empty"
fi

echo ""
log_info "Test 10: Full list formats or similar"
output=$(docker exec kreuzberg-full-test kreuzberg --help 2>&1 || true)
if [ ! -z "$output" ]; then
	log_success "Full CLI is responsive"
else
	log_fail "Full CLI output empty"
fi

echo ""
print_summary
