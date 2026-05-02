```typescript title="TypeScript"
import { extractFile, ExtractionConfig } from "@kreuzberg/node";

const config = ExtractionConfig.discover();
if (config) {
  const result = await extractFile("document.pdf", null, config);
  console.log(result.content);
} else {
  console.log("No configuration file found");
}
```
