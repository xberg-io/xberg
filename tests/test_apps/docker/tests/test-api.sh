#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "API Endpoint Tests"
echo "================================${NC}"

CORE_API="http://localhost:8000"
FULL_API="http://localhost:8001"

echo ""
log_info "Test 1: Core health endpoint"
assert_http_status "$CORE_API/health" 200 "Core /health endpoint returns 200"

echo ""
log_info "Test 2: Full health endpoint"
assert_http_status "$FULL_API/health" 200 "Full /health endpoint returns 200"

echo ""
log_info "Test 3: Core API responds"
response=$(curl -s "$CORE_API/health" 2>/dev/null || echo "{}")
if assert_contains "$response" "healthy" "Core API response contains 'healthy'"; then
	:
else
	log_warn "Core health response: $response"
fi

echo ""
log_info "Test 4: Full API responds"
response=$(curl -s "$FULL_API/health" 2>/dev/null || echo "{}")
if assert_contains "$response" "healthy" "Full API response contains 'healthy'"; then
	:
else
	log_warn "Full health response: $response"
fi

echo ""
log_info "Test 5: Core extract endpoint exists"
status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt"}' 2>/dev/null)
if [ "$status" != "404" ]; then
	log_success "Core /extract endpoint exists (status: $status)"
else
	log_fail "Core /extract endpoint not found"
fi

echo ""
log_info "Test 6: Full extract endpoint exists"
status=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt"}' 2>/dev/null)
if [ "$status" != "404" ]; then
	log_success "Full /extract endpoint exists (status: $status)"
else
	log_fail "Full /extract endpoint not found"
fi

echo ""
log_info "Test 7: Core can extract text file"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt"}' 2>/dev/null)
if assert_contains "$response" "content" "Core extract text returns content"; then
	:
else
	log_warn "Core extract response: $response"
fi

echo ""
log_info "Test 8: Full can extract text file"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt"}' 2>/dev/null)
if assert_contains "$response" "content" "Full extract text returns content"; then
	:
else
	log_warn "Full extract response: $response"
fi

echo ""
log_info "Test 9: Core can extract PDF"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf"}' 2>/dev/null)
if assert_contains "$response" "content" "Core extract PDF returns content"; then
	:
else
	log_warn "Core PDF extract response: $response"
fi

echo ""
log_info "Test 10: Full can extract PDF"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf"}' 2>/dev/null)
if assert_contains "$response" "content" "Full extract PDF returns content"; then
	:
else
	log_warn "Full PDF extract response: $response"
fi

echo ""
log_info "Test 11: Core can extract Markdown"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/extraction_test.md"}' 2>/dev/null)
if assert_contains "$response" "content" "Core extract Markdown returns content"; then
	:
else
	log_warn "Core Markdown extract response: $response"
fi

echo ""
log_info "Test 12: Full can extract Markdown"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/extraction_test.md"}' 2>/dev/null)
if assert_contains "$response" "content" "Full extract Markdown returns content"; then
	:
else
	log_warn "Full Markdown extract response: $response"
fi

echo ""
log_info "Test 13: Core can extract ODT"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/simple.odt"}' 2>/dev/null)
if assert_contains "$response" "content" "Core extract ODT returns content"; then
	:
else
	log_warn "Core ODT extract response: $response"
fi

echo ""
log_info "Test 14: Full can extract ODT"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/simple.odt"}' 2>/dev/null)
if assert_contains "$response" "content" "Full extract ODT returns content"; then
	:
else
	log_warn "Full ODT extract response: $response"
fi

echo ""
log_info "Test 15: Core can extract image"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/example.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core extract image returns response"; then
	:
else
	log_warn "Core image extract response: $response"
fi

echo ""
log_info "Test 16: Full can extract image"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/example.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full extract image returns response"; then
	:
else
	log_warn "Full image extract response: $response"
fi

echo ""
print_summary
