import { extractFileSync } from "kreuzberg";

// Test 1: XLSX
console.log("=== Testing XLSX ===");
const xlsxResult = extractFileSync(
  "/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/xlsx/stanley_cups.xlsx",
);
console.log("metadata:", JSON.stringify(xlsxResult.metadata, null, 2));
console.log("format:", xlsxResult.metadata?.format);
console.log("excel:", xlsxResult.metadata?.format?.excel);

// Test 2: DOCX with document structure
console.log("\n=== Testing DOCX ===");
const docxResult = extractFileSync(
  "/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/docx/fake.docx",
  undefined,
  {
    includeDocumentStructure: true,
  },
);
console.log("document:", docxResult.document);
console.log("typeof document:", typeof docxResult.document);
