```typescript title="Document Structure Config (TypeScript)"
import { extractFileSync, ExtractionConfig } from "@kreuzberg/node";

const config: ExtractionConfig = {
  includeDocumentStructure: true,
};

const result = extractFileSync("document.pdf", undefined, config);

if (result.document) {
  for (const node of result.document.nodes) {
    console.log(`[${node.content.nodeType}] ${node.content.text ?? ""}`);
  }
}
```
