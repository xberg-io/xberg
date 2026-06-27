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

if (result.pages) {
  console.log(`Total pages extracted: ${result.pages.length}`);

  result.pages.forEach((page) => {
    console.log(`Page ${page.pageNumber}:`);
    console.log(`  Content length: ${page.content.length} chars`);
    console.log(`  Tables: ${page.tables.length}`);
    console.log(`  Images: ${page.images.length}`);

    // Check if page is blank
    if (page.isBlank) {
      console.log("  This page is blank");
    }

    // Access page hierarchy if available
    if (page.hierarchy) {
      console.log(`  Hierarchy level: ${page.hierarchy}`);
    }
  });
}
```
