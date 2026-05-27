import { extractFileSync } from "kreuzberg";

// Test XLSX
const xlsxResult = extractFileSync(
	"/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/xlsx/stanley_cups.xlsx",
);
const fmt = xlsxResult.metadata?.format;
console.log("format:", fmt);
console.log("format.format_type:", fmt?.format_type);
console.log("typeof format:", typeof fmt);
console.log("Object.keys(format):", Object.keys(fmt || {}));
