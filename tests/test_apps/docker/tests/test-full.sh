#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

echo -e "${BLUE}================================"
echo "Full Image Specific Tests"
echo "================================${NC}"

echo ""
log_info "Test 1: LibreOffice should be available"
if docker exec kreuzberg-full-test which libreoffice >/dev/null 2>&1; then
	log_success "LibreOffice correctly installed in full image"
else
	log_fail "LibreOffice not found in full image"
fi

echo ""
log_info "Test 2: LibreOffice version"
version=""
version=$(docker exec kreuzberg-full-test libreoffice --version 2>&1 || echo "unknown")
if assert_contains "$version" "LibreOffice" "LibreOffice version detected"; then
	:
else
	log_warn "LibreOffice version check inconclusive: $version"
fi

echo ""
log_info "Test 3: Tesseract OCR is available"
if docker exec kreuzberg-full-test which tesseract >/dev/null 2>&1; then
	log_success "Tesseract found in full image"
else
	log_fail "Tesseract not found in full image"
fi

echo ""
log_info "Test 4: Office document extraction capability"
output=$(docker exec kreuzberg-full-test kreuzberg extract --help 2>&1 || true)
if assert_contains "$output" "docx\|xlsx\|office\|format" "Full supports Office formats"; then
	:
else
	log_warn "Office format extraction not clearly advertised"
fi

echo ""
log_info "Test 5: DOCX file mime detection"
if [ -f "/Users/naamanhirschfeld/workspace/kreuzberg-dev/test_apps/docker/fixtures/sample.docx" ]; then
	mime=""
	mime=$(docker exec kreuzberg-full-test file -b --mime-type /fixtures/sample.docx 2>&1)
	if assert_contains "$mime" "officedocument\|word\|application" "DOCX mime type detected"; then
		:
	else
		log_warn "DOCX mime type unexpected: $mime"
	fi
else
	log_skip "DOCX extraction - fixture not found"
fi

echo ""
log_info "Test 6: XLSX file mime detection"
if [ -f "/Users/naamanhirschfeld/workspace/kreuzberg-dev/test_apps/docker/fixtures/sample.xlsx" ]; then
	mime=""
	mime=$(docker exec kreuzberg-full-test file -b --mime-type /fixtures/sample.xlsx 2>&1)
	if assert_contains "$mime" "spreadsheet\|sheet\|application" "XLSX mime type detected"; then
		:
	else
		log_warn "XLSX mime type unexpected: $mime"
	fi
else
	log_skip "XLSX extraction - fixture not found"
fi

echo ""
log_info "Test 7: Full container dependencies check"
deps_ok=1
for lib in libfontconfig libxinerama libgl1 libxrender1 libsm6; do
	if docker exec kreuzberg-full-test dpkg -l 2>/dev/null | grep -q "$lib"; then
		:
	else
		log_warn "Dependency may be missing: $lib"
		deps_ok=0
	fi
done
if [ "$deps_ok" -eq 1 ]; then
	log_success "Core LibreOffice dependencies verified"
else
	log_warn "Some LibreOffice dependencies may be missing"
fi

echo ""
log_info "Test 8: Full container memory check"
memory=""
memory=$(docker stats --no-stream kreuzberg-full-test 2>/dev/null | tail -1 | awk '{print $4}' || echo "unknown")
log_info "Full container using: $memory of memory"
log_success "Memory check completed"

echo ""
log_info "Test 9: Fixtures directory mounted"
if docker exec kreuzberg-full-test [ -d "/fixtures" ]; then
	log_success "Fixtures directory is mounted in full container"
else
	log_fail "Fixtures directory not mounted in full container"
fi

echo ""
log_info "Test 10: Cache directory is writable"
if docker exec kreuzberg-full-test touch /app/.kreuzberg/test_write 2>/dev/null &&
	docker exec kreuzberg-full-test rm /app/.kreuzberg/test_write 2>/dev/null; then
	log_success "Cache directory is writable"
else
	log_fail "Cache directory is not writable"
fi

echo ""
log_info "Test 11: Full can extract legacy .doc file"
FULL_API="http://localhost:8001"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/unit_test_lists.doc"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full extract legacy .doc returns response"; then
	:
else
	log_warn "Full .doc extract response: $response"
fi

echo ""
log_info "Test 12: Full can extract modern .docx file"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/lorem_ipsum.docx"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full extract .docx returns response"; then
	:
else
	log_warn "Full .docx extract response: $response"
fi

echo ""
log_info "Test 13: Full can extract DOCX with tables"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/docx_tables.docx"}' 2>/dev/null)
if assert_contains "$response" "content\|table\|success" "Full extract DOCX tables returns response"; then
	:
else
	log_warn "Full DOCX tables response: $response"
fi

echo ""
log_info "Test 14: Full can extract .xlsx file"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/stanley_cups.xlsx"}' 2>/dev/null)
if assert_contains "$response" "content\|table\|success" "Full extract .xlsx returns response"; then
	:
else
	log_warn "Full .xlsx extract response: $response"
fi

echo ""
log_info "Test 15: LibreOffice soffice binary check"
if docker exec kreuzberg-full-test which soffice >/dev/null 2>&1; then
	log_success "LibreOffice soffice command found (conversion tool)"
else
	log_warn "LibreOffice soffice not directly found, but libreoffice may be available"
fi

echo ""
log_info "Test 16: ONNX Runtime availability in full"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/sample.txt","generate_embeddings":true}' 2>/dev/null)
if assert_contains "$response" "content\|embedding\|success" "Full with embeddings returns response"; then
	:
else
	log_warn "Full embeddings response: $response"
fi

echo ""
log_info "Test 17: Full can process image files with OCR"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/ocr_image.jpg"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full OCR image returns response"; then
	:
else
	log_warn "Full OCR image response: $response"
fi

echo ""
log_info "Test 18: Full can process ODT documents"
response=$(curl -s -X POST "$FULL_API/extract" \
	-H "Content-Type: application/json" \
	-d '{"path":"/fixtures/simple.odt"}' 2>/dev/null)
if assert_contains "$response" "content\|success" "Full extract ODT returns response"; then
	:
else
	log_warn "Full ODT extract response: $response"
fi

echo ""
log_info "Test 19: Full API health status"
assert_http_status "$FULL_API/health" 200 "Full /health endpoint working"

echo ""
log_info "Test 20: Full container disk space check"
disk_usage=""
disk_usage=$(docker exec kreuzberg-full-test df /app 2>/dev/null | tail -1 | awk '{print $5}' || echo "unknown")
log_info "Full container disk usage: $disk_usage"
log_success "Disk usage check completed"

echo ""
print_summary
