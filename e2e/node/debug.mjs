import { extractSync } from "xberg";

// Test DOCX
const docxOutput = extractSync(
	{
		kind: "uri",
		uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/docx/fake.docx",
	},
	{ includeDocumentStructure: true },
);
const docxResult = docxOutput.results[0];
console.log("=== DOCX Result ===");
console.log("document:", docxResult.document);
console.log("typeof document:", typeof docxResult.document);
console.log(
	"keys:",
	Object.keys(docxResult).filter((k) => k.includes("doc")),
);

// Test XLSX
const xlsxOutput = extractSync({
	kind: "uri",
	uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/xlsx/stanley_cups.xlsx",
});
const xlsxResult = xlsxOutput.results[0];
console.log("\n=== XLSX Result ===");
console.log("metadata.format:", xlsxResult.metadata?.format);
console.log("typeof format:", typeof xlsxResult.metadata?.format);
console.log("format keys:", Object.keys(xlsxResult.metadata?.format || {}));
console.log("format.format_type:", xlsxResult.metadata?.format?.format_type);
console.log("format['0']:", xlsxResult.metadata?.format?.["0"]);
