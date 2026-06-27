```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  forceOcr: true,
  ocr: {
    backend: "vlm",
    vlmConfig: {
      model: "openai/gpt-4o-mini",
    },
  },
};

const result = extract({ kind: "uri", uri: "scan.pdf" }, config);
console.log(result.content);
```
