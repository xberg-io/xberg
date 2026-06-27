```typescript title="Document Structure Config (TypeScript)"
import { extractSync, ExtractionConfig } from "@xberg-io/xberg";

const config: ExtractionConfig = {
  includeDocumentStructure: true,
};

const result = extractSync("document.pdf", undefined, config);

if (result.document) {
  for (const node of result.document.nodes) {
    console.log(`[${node.content.nodeType}] ${node.content.text ?? ""}`);
  }
}
```
