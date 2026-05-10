import { extractFileSync } from "kreuzberg";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const testDocumentsDir = join(__dirname, "..", "..", "test_documents");
process.chdir(testDocumentsDir);

const result = extractFileSync("docx/fake.docx", undefined, {
  includeDocumentStructure: true,
});

console.log("result.document:", result.document);
console.log('result.document ?? "":', result.document ?? "");
const fallback = result.document ?? "";
console.log("fallback:", fallback);
console.log("fallback.length:", fallback.length);
console.log("typeof fallback.length:", typeof fallback.length);
