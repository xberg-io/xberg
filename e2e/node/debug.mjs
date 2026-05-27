import { extractFileSync } from "kreuzberg";

// Test DOCX
const docxResult = extractFileSync(
	"/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/docx/fake.docx",
	undefined,
	{
		includeDocumentStructure: true,
	},
);
console.log("=== DOCX Result ===");
console.log("document:", docxResult.document);
console.log("typeof document:", typeof docxResult.document);
console.log(
	"keys:",
	Object.keys(docxResult).filter((k) => k.includes("doc")),
);

// Test XLSX
const xlsxResult = extractFileSync(
	"/Users/naamanhirschfeld/workspace/kreuzberg-dev/kreuzberg/test_documents/xlsx/stanley_cups.xlsx",
);
console.log("\n=== XLSX Result ===");
console.log("metadata.format:", xlsxResult.metadata?.format);
console.log("typeof format:", typeof xlsxResult.metadata?.format);
console.log("format keys:", Object.keys(xlsxResult.metadata?.format || {}));
console.log("format.format_type:", xlsxResult.metadata?.format?.format_type);
console.log("format['0']:", xlsxResult.metadata?.format?.["0"]);
