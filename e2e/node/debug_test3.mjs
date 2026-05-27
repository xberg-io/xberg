import { extractFileSync } from "kreuzberg";

// Test XLSX
const xlsxResult = extractFileSync(
	"/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/xlsx/stanley_cups.xlsx",
);
const fmt = xlsxResult.metadata?.format;
console.log("fmt.excel:", fmt?.excel);
console.log("fmt.docx:", fmt?.docx);
console.log("fmt.pdf:", fmt?.pdf);
console.log("Calling as function?", typeof fmt?.excel);
