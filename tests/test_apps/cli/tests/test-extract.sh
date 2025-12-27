#!/usr/bin/env bash
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_DOCS_DIR="$SCRIPT_DIR/../test_documents"

echo "===== Kreuzberg CLI Extraction Test ====="
echo

if ! command -v kreuzberg &>/dev/null; then
	echo -e "${RED}✗ kreuzberg not found. Run ./tests/install.sh first.${NC}"
	exit 1
fi

PASSED=0
FAILED=0

echo "Testing PDF extraction..."
if kreuzberg extract "$TEST_DOCS_DIR/tiny.pdf" >/dev/null 2>&1; then
	echo -e "${GREEN}✓ PDF extraction successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ PDF extraction failed${NC}"
	((FAILED++))
fi

echo "Testing DOCX extraction..."
if kreuzberg extract "$TEST_DOCS_DIR/lorem_ipsum.docx" >/dev/null 2>&1; then
	echo -e "${GREEN}✓ DOCX extraction successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ DOCX extraction failed${NC}"
	((FAILED++))
fi

echo "Testing XLSX extraction..."
if kreuzberg extract "$TEST_DOCS_DIR/stanley_cups.xlsx" >/dev/null 2>&1; then
	echo -e "${GREEN}✓ XLSX extraction successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ XLSX extraction failed${NC}"
	((FAILED++))
fi

echo "Testing JSON output format..."
if kreuzberg extract "$TEST_DOCS_DIR/tiny.pdf" --format json >/dev/null 2>&1; then
	echo -e "${GREEN}✓ JSON format successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ JSON format failed${NC}"
	((FAILED++))
fi

echo "Testing markdown output format..."
if kreuzberg extract "$TEST_DOCS_DIR/tiny.pdf" --format markdown >/dev/null 2>&1; then
	echo -e "${GREEN}✓ Markdown format successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ Markdown format failed${NC}"
	((FAILED++))
fi

echo "Testing error handling (non-existent file)..."
if ! kreuzberg extract "/nonexistent/file.pdf" >/dev/null 2>&1; then
	echo -e "${GREEN}✓ Error handling successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ Error handling failed (should have errored)${NC}"
	((FAILED++))
fi

echo "Testing --version flag..."
if kreuzberg --version >/dev/null 2>&1; then
	echo -e "${GREEN}✓ Version flag successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ Version flag failed${NC}"
	((FAILED++))
fi

echo "Testing --help flag..."
if kreuzberg --help >/dev/null 2>&1; then
	echo -e "${GREEN}✓ Help flag successful${NC}"
	((PASSED++))
else
	echo -e "${RED}✗ Help flag failed${NC}"
	((FAILED++))
fi

echo
echo "===== Test Results ====="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"

if [ $FAILED -eq 0 ]; then
	echo -e "${GREEN}===== CLI Extraction Test PASSED =====${NC}"
	exit 0
else
	echo -e "${RED}===== CLI Extraction Test FAILED =====${NC}"
	exit 1
fi
