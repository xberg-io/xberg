import { extractSync } from "xberg";

// Test XLSX
const xlsxOutput = extractSync({
	kind: "uri",
	uri: "/Users/naamanhirschfeld/workspace/xberg-io/xberg/test_documents/xlsx/stanley_cups.xlsx",
});
const xlsxResult = xlsxOutput.results[0];
const fmt = xlsxResult.metadata?.format;
console.log("fmt.excel:", fmt?.excel);
console.log("fmt.docx:", fmt?.docx);
console.log("fmt.pdf:", fmt?.pdf);
console.log("Calling as function?", typeof fmt?.excel);
