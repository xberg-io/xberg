```typescript title="Element-Based Output (WASM)"
import { extractSync, ExtractionConfig } from "xberg-wasm";

// Configure element-based output
const config: ExtractionConfig = {
  outputFormat: "element_based",
};

// Extract document
const result = extractSync(fileBuffer, "application/pdf", config);

// Access elements
for (const element of result.elements) {
  console.log(`Type: ${element.elementType}`);
  console.log(`Text: ${element.text.slice(0, 100)}`);

  if (element.metadata.pageNumber) {
    console.log(`Page: ${element.metadata.pageNumber}`);
  }

  if (element.metadata.coordinates) {
    const coords = element.metadata.coordinates;
    console.log(`Coords: (${coords.left}, ${coords.top}) - (${coords.right}, ${coords.bottom})`);
  }

  console.log("---");
}

// Filter by element type
const titles = result.elements.filter((e) => e.elementType === "title");
for (const title of titles) {
  const level = title.metadata.additional?.level || "unknown";
  console.log(`[${level}] ${title.text}`);
}
```
