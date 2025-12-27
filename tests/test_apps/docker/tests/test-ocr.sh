#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "OCR Tests"
echo "================================${NC}"

CORE_API="http://localhost:8000"
FULL_API="http://localhost:8001"

echo ""
log_info "Test 1: Core can OCR image file (default)"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/ocr_image.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core OCR image extraction returns response"; then
	:
else
	log_warn "Core OCR response: $response"
fi

echo ""
log_info "Test 2: Full can OCR image file (default)"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/ocr_image.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full OCR image extraction returns response"; then
	:
else
	log_warn "Full OCR response: $response"
fi

echo ""
log_info "Test 3: Core can force OCR on image-only PDF"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/image_only_german_pdf.pdf","force_ocr":true}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core force OCR on PDF returns response"; then
	:
else
	log_warn "Core force OCR response: $response"
fi

echo ""
log_info "Test 4: Full can force OCR on image-only PDF"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/image_only_german_pdf.pdf","force_ocr":true}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full force OCR on PDF returns response"; then
	:
else
	log_warn "Full force OCR response: $response"
fi

echo ""
log_info "Test 5: Core OCR with PNG image"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.png"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core OCR PNG returns response"; then
	:
else
	log_warn "Core OCR PNG response: $response"
fi

echo ""
log_info "Test 6: Full OCR with PNG image"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.png"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full OCR PNG returns response"; then
	:
else
	log_warn "Full OCR PNG response: $response"
fi

echo ""
log_info "Test 7: Core OCR with large PDF"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/large.pdf"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core large PDF extraction returns response"; then
	:
else
	log_warn "Core large PDF response: $response"
fi

echo ""
log_info "Test 8: Full OCR with large PDF"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/large.pdf"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full large PDF extraction returns response"; then
	:
else
	log_warn "Full large PDF response: $response"
fi

echo ""
log_info "Test 9: Core normal PDF extraction without OCR"
response=$(curl -s -X POST "$CORE_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Core PDF extraction without OCR returns response"; then
	:
else
	log_warn "Core PDF no-OCR response: $response"
fi

echo ""
log_info "Test 10: Full normal PDF extraction without OCR"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/tiny.pdf"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full PDF extraction without OCR returns response"; then
	:
else
	log_warn "Full PDF no-OCR response: $response"
fi

echo ""
print_summary
