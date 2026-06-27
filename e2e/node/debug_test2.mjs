import { extractSync } from "xberg";

// Test XLSX
const xlsxOutput = extractSync({
	kind: "uri",
	uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/xlsx/stanley_cups.xlsx",
});
const xlsxResult = xlsxOutput.results[0];
const fmt = xlsxResult.metadata?.format;
console.log("format:", fmt);
console.log("format.format_type:", fmt?.format_type);
console.log("typeof format:", typeof fmt);
console.log("Object.keys(format):", Object.keys(fmt || {}));
