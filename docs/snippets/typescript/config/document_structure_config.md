```typescript title="Document Structure Config (TypeScript)"
import { extract, ExtractionConfig } from "@xberg-io/xberg";

const config: ExtractionConfig = {
  includeDocumentStructure: true,
};

const result = extract({ kind: "uri", uri: "document.pdf" }, config);

if (result.document) {
  for (const node of result.document.nodes) {
    console.log(`[${node.content.nodeType}] ${node.content.text ?? ""}`);
  }
}
```
