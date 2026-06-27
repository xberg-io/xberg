```typescript title="TypeScript"
import { extractSync } from "xberg";

const config = {
  // OCR: Tesseract on all pages with English text
  force_ocr: false,
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
  // Chunking: semantic markdown chunks of ~800 chars, 100-char overlap
  chunking: {
    max_characters: 800,
    overlap: 100,
    chunker_type: "markdown",
    prepend_heading_context: true,
  },
  // Output: include document structure and tables
  output_format: "markdown",
  include_document_structure: true,
  // Images: extract embedded images
  images: {
    extract_images: true,
  },
  // Cache extracted results on disk
  use_cache: true,
  enable_quality_processing: true,
};

const result = extractSync("report.pdf", undefined, config);

console.log(`Content (${result.content.length} chars):`);
console.log(result.content.slice(0, 200));

if (result.chunks) {
  console.log(`\nChunks: ${result.chunks.length}`);
}
console.log(`Tables: ${result.tables?.length ?? 0}`);
if (result.detected_languages) {
  console.log(`Languages: ${result.detected_languages}`);
}
if (result.extraction_method) {
  console.log(`Extraction method: ${result.extraction_method}`);
}
```
