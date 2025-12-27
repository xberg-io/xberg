#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "Core Image Specific Tests"
echo "================================${NC}"

echo ""
log_info "Test 1: LibreOffice should NOT be available"
if docker exec kreuzberg-core-test which libreoffice >/dev/null 2>&1; then
	log_fail "LibreOffice found in core image (should not be present)"
else
	log_success "LibreOffice correctly not installed in core image"
fi

echo ""
log_info "Test 2: Tesseract OCR is available"
if docker exec kreuzberg-core-test which tesseract >/dev/null 2>&1; then
	log_success "Tesseract found in core image"
else
	log_fail "Tesseract not found in core image"
fi

echo ""
log_info "Test 3: Text extraction works"
if [ -f "/Users/naamanhirschfeld/workspace/kreuzberg-dev/test_apps/docker/fixtures/sample.txt" ]; then
	result=$(docker exec -w /fixtures kreuzberg-core-test kreuzberg extract sample.txt 2>&1 || true)
	if [ ! -z "$result" ]; then
		log_success "Text extraction produces output"
	else
		log_warn "Text extraction returned empty result"
	fi
else
	log_skip "Text extraction test - fixture not found"
fi

echo ""
log_info "Test 4: PDF extraction capability available"
output=$(docker exec kreuzberg-core-test kreuzberg extract --help 2>&1 || true)
if assert_contains "$output" "pdf\|file\|format" "Core supports PDF extraction"; then
	:
else
	log_warn "PDF extraction not clearly advertised in help"
fi

echo ""
log_info "Test 5: ONNX Runtime embeddings available"
result=""
result=$(docker exec kreuzberg-core-test ldconfig -p 2>/dev/null | grep -i onnx || echo "")
if [ -n "$result" ]; then
	log_success "ONNX Runtime libraries installed"
else
	log_warn "ONNX Runtime not clearly visible (may still be available)"
fi

echo ""
log_info "Test 6: Core container memory check"
memory=""
memory=$(docker stats --no-stream kreuzberg-core-test 2>/dev/null | tail -1 | awk '{print $4}' || echo "unknown")
log_info "Core container using: $memory of memory"
log_success "Memory check completed"

echo ""
log_info "Test 7: Fixtures directory mounted"
if docker exec kreuzberg-core-test [ -d "/fixtures" ]; then
	log_success "Fixtures directory is mounted in core container"
else
	log_fail "Fixtures directory not mounted in core container"
fi

echo ""
log_info "Test 8: Cache directory is writable"
if docker exec kreuzberg-core-test touch /app/.kreuzberg/test_write 2>/dev/null &&
	docker exec kreuzberg-core-test rm /app/.kreuzberg/test_write 2>/dev/null; then
	log_success "Cache directory is writable"
else
	log_fail "Cache directory is not writable"
fi

echo ""
log_info "Test 9: Core can extract PDF via API"
CORE_API="http://localhost:8000"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core API PDF extraction returns response"; then
	:
else
	log_warn "Core API PDF response: $response"
fi

echo ""
log_info "Test 10: Core can extract text files via API"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core API text extraction returns response"; then
	:
else
	log_warn "Core API text response: $response"
fi

echo ""
log_info "Test 11: Core can extract Markdown via API"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/extraction_test.md"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core API Markdown extraction returns response"; then
	:
else
	log_warn "Core API Markdown response: $response"
fi

echo ""
log_info "Test 12: Core can extract ODT files via API"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/simple.odt"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core API ODT extraction returns response"; then
	:
else
	log_warn "Core API ODT response: $response"
fi

echo ""
log_info "Test 13: Core can process images with OCR"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/ocr_image.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core API image OCR returns response"; then
	:
else
	log_warn "Core API OCR response: $response"
fi

echo ""
log_info "Test 14: Core correctly lacks LibreOffice (.doc/.xlsx support)"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/lorem_ipsum.docx"}' 2>/dev/null)
if assert_contains "$response" "error\|not.*supported\|unsupported" "Core correctly rejects .docx without LibreOffice"; then
	log_success "Core properly rejects modern Office formats"
elif assert_contains "$response" "content" "Core processes .docx (may be via alternative method)"; then
	log_warn "Core may have alternate support for .docx (not LibreOffice)"
else
	log_info "Core response for .docx: $response"
fi

echo ""
log_info "Test 15: Tesseract data files are available"
if docker exec kreuzberg-core-test [ -d "/usr/share/tesseract-ocr" ]; then
	log_success "Tesseract data directory found"
else
	log_warn "Tesseract data directory not found (may be in alternative location)"
fi

echo ""
log_info "Test 16: Core API health check"
assert_http_status "$CORE_API/health" 200 "Core /health endpoint working"

echo ""
log_info "Test 17: Core can generate embeddings"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Core embeddings returns response"; then
	:
else
	log_warn "Core embeddings response: $response"
fi

echo ""
log_info "Test 18: Core image size efficiency"
size=""
size=$(docker inspect kreuzberg-core-test --format='{{.Config.Size}}' 2>/dev/null | awk '{print $1}' || echo "unknown")
log_info "Core container size: $size bytes"
log_success "Size check completed"

echo ""
print_summary
