```typescript title="WASM"
import init, { extract } from "xberg-wasm";
import { ExtractionConfig } from "xberg-wasm";

await init();

const fileBuffer = new Uint8Array(/* your file bytes */);
const mimeType = "application/pdf";

const config = new ExtractionConfig({});

const result = await extract(fileBuffer, mimeType, config);

if (result.tables && result.tables.length > 0) {
  console.log(`Found ${result.tables.length} tables`);

  result.tables.forEach((table, index) => {
    console.log(`\nTable ${index + 1}:`);
    console.log(`  Page: ${table.pageNumber}`);
    console.log(`  Markdown representation:`);
    console.log(table.markdown);

    // Access cell data
    const cells = table.cells;
    if (cells) {
      console.log(`  Total cells: ${Object.keys(cells).length}`);

      // Iterate through cells (structure depends on how cells are serialized)
      for (const rowKey of Object.keys(cells)) {
        const row = cells[rowKey];
        console.log(`  Row ${rowKey}: ${JSON.stringify(row)}`);
      }
    }

    // Access bounding box if available
    if (table.boundingBox) {
      console.log(`  Bounding box: ${table.boundingBox}`);
    }
  });
} else {
  console.log("No tables found in document");
}
```
