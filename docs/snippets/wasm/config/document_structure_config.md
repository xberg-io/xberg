```typescript title="Document Structure Config (WASM)"
import { extract } from "xberg-wasm";

const config = {
  includeDocumentStructure: true,
};

const result = extract(fileBuffer, "application/pdf", config);

if (result.document) {
  for (const node of result.document.nodes) {
    console.log(`[${node.content.nodeType}]`);
  }
}
```
