import { extractSync } from "xberg";

// Test 1: XLSX
console.log("=== Testing XLSX ===");
const xlsxOutput = extractSync({
	kind: "uri",
	uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/xlsx/stanley_cups.xlsx",
});
const xlsxResult = xlsxOutput.results[0];
console.log("metadata:", JSON.stringify(xlsxResult.metadata, null, 2));
console.log("format:", xlsxResult.metadata?.format);
console.log("excel:", xlsxResult.metadata?.format?.excel);

// Test 2: DOCX with document structure
console.log("\n=== Testing DOCX ===");
const docxOutput = extractSync(
	{
		kind: "uri",
		uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/docx/fake.docx",
	},
	{ includeDocumentStructure: true },
);
const docxResult = docxOutput.results[0];
console.log("document:", docxResult.document);
console.log("typeof document:", typeof docxResult.document);
