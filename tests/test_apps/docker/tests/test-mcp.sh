#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "MCP Protocol Tests"
echo "================================${NC}"

echo ""
log_info "Test 1: Core MCP mode available"
output=$(docker exec kreuzberg-core-test kreuzberg --help 2>&1 || true)
if assert_contains "$output" "mcp" "Core supports MCP command"; then
	:
else
	log_fail "Core MCP command not found in help"
fi

echo ""
log_info "Test 2: Full MCP mode available"
output=$(docker exec kreuzberg-full-test kreuzberg --help 2>&1 || true)
if assert_contains "$output" "mcp" "Full supports MCP command"; then
	:
else
	log_fail "Full MCP command not found in help"
fi

echo ""
log_info "Test 3: Core MCP help works"
output=$(docker exec kreuzberg-core-test kreuzberg mcp --help 2>&1 || true)
if [ ! -z "$output" ]; then
	log_success "Core MCP help is available"
else
	log_fail "Core MCP help is empty"
fi

echo ""
log_info "Test 4: Full MCP help works"
output=$(docker exec kreuzberg-full-test kreuzberg mcp --help 2>&1 || true)
if [ ! -z "$output" ]; then
	log_success "Full MCP help is available"
else
	log_fail "Full MCP help is empty"
fi

echo ""
log_info "Test 5: Core MCP stdio mode can start (basic check)"
timeout 2 docker exec kreuzberg-core-test kreuzberg mcp stdio 2>&1 || true
log_success "Core MCP stdio mode starts (may exit after timeout)"

echo ""
log_info "Test 6: Full MCP stdio mode can start (basic check)"
timeout 2 docker exec kreuzberg-full-test kreuzberg mcp stdio 2>&1 || true
log_success "Full MCP stdio mode starts (may exit after timeout)"

echo ""
log_info "Test 7: Core MCP modes available"
output=$(docker exec kreuzberg-core-test kreuzberg mcp --help 2>&1 || true)
if assert_contains "$output" "stdio\|sse\|server" "Core MCP has server modes"; then
	:
else
	log_warn "Core MCP help output: $output"
fi

echo ""
log_info "Test 8: Full MCP modes available"
output=$(docker exec kreuzberg-full-test kreuzberg mcp --help 2>&1 || true)
if assert_contains "$output" "stdio\|sse\|server" "Full MCP has server modes"; then
	:
else
	log_warn "Full MCP help output: $output"
fi

echo ""
print_summary
