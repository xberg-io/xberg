```typescript title="WASM"
import init, { extract } from "xberg-wasm";
import { PageConfig, ExtractionConfig } from "xberg-wasm";

await init();

const fileBuffer = new Uint8Array(/* your file bytes */);
const mimeType = "application/pdf";

const config = new ExtractionConfig({
  pages: new PageConfig({
    extract_pages: true,
  }),
});

const result = await extract(fileBuffer, mimeType, config);

if (result.metadata && result.metadata.pages) {
  const pageStructure = result.metadata.pages;
  console.log(`Total pages: ${pageStructure.total_count}`);

  if (pageStructure.boundaries) {
    // Iterate through page boundaries to map content to pages
    pageStructure.boundaries.forEach((boundary) => {
      const pageText = result.content.substring(
        boundary.byte_start,
        Math.min(boundary.byte_end, boundary.byte_start + 100),
      );

      console.log(`Page ${boundary.page_number}:`);
      console.log(`  Byte range: ${boundary.byte_start}-${boundary.byte_end}`);
      console.log(`  Preview: ${pageText}...`);
    });
  }
}
```
