```typescript title="TypeScript"
import { extract, ExtractionConfig } from "@xberg-io/xberg";

const config = ExtractionConfig.discover();
if (config) {
  const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
  console.log(result.content);
} else {
  console.log("No configuration file found");
}
```
