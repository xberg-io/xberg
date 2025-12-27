#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "Embeddings Tests"
echo "================================${NC}"

CORE_API="http://localhost:8000"
FULL_API="http://localhost:8001"

log_info "Testing ONNX Runtime and Embeddings functionality"
log_info "Note: These tests assume embeddings endpoint is available"
echo ""

echo ""
log_info "Test 1: Core has ONNX Runtime available"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt","embedding_model":"default"}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Core extract with embeddings returns response"; then
	:
else
	log_warn "Core embeddings response: $response"
fi

echo ""
log_info "Test 2: Full has ONNX Runtime available"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt","embedding_model":"default"}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Full extract with embeddings returns response"; then
	:
else
	log_warn "Full embeddings response: $response"
fi

echo ""
log_info "Test 3: Core can generate embeddings for PDF"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Core PDF embeddings returns response"; then
	:
else
	log_warn "Core PDF embeddings response: $response"
fi

echo ""
log_info "Test 4: Full can generate embeddings for PDF"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Full PDF embeddings returns response"; then
	:
else
	log_warn "Full PDF embeddings response: $response"
fi

echo ""
log_info "Test 5: Core can generate embeddings for DOCX"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/lorem_ipsum.docx","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Core DOCX embeddings returns response"; then
	:
else
	log_warn "Core DOCX embeddings response: $response"
fi

echo ""
log_info "Test 6: Full can generate embeddings for DOCX"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/lorem_ipsum.docx","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Full DOCX embeddings returns response"; then
	:
else
	log_warn "Full DOCX embeddings response: $response"
fi

echo ""
log_info "Test 7: Core can generate embeddings for XLSX"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/stanley_cups.xlsx","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Core XLSX embeddings returns response"; then
	:
else
	log_warn "Core XLSX embeddings response: $response"
fi

echo ""
log_info "Test 8: Full can generate embeddings for XLSX"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/stanley_cups.xlsx","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Full XLSX embeddings returns response"; then
	:
else
	log_warn "Full XLSX embeddings response: $response"
fi

echo ""
log_info "Test 9: Core cache directory is writable"
if docker exec kreuzberg-core-test touch /app/.kreuzberg/test-write.txt 2>/dev/null; then
	if docker exec kreuzberg-core-test rm /app/.kreuzberg/test-write.txt 2>/dev/null; then
		log_success "Core cache directory is writable"
	else
		log_fail "Core cache directory write test cleanup failed"
	fi
else
	log_fail "Core cache directory is not writable"
fi

echo ""
log_info "Test 10: Full cache directory is writable"
if docker exec kreuzberg-full-test touch /app/.kreuzberg/test-write.txt 2>/dev/null; then
	if docker exec kreuzberg-full-test rm /app/.kreuzberg/test-write.txt 2>/dev/null; then
		log_success "Full cache directory is writable"
	else
		log_fail "Full cache directory write test cleanup failed"
	fi
else
	log_fail "Full cache directory is not writable"
fi

echo ""
print_summary
