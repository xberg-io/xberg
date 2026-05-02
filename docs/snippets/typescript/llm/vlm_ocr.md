```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  forceOcr: true,
  ocr: {
    backend: "vlm",
    vlmConfig: {
      model: "openai/gpt-4o-mini",
    },
  },
};

const result = extractFileSync("scan.pdf", null, config);
console.log(result.content);
```
