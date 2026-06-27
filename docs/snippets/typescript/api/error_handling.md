```typescript title="TypeScript"
import { extractSync } from "xberg";

try {
  const result = extractSync("missing.pdf");
  console.log(result.content);
} catch (error: unknown) {
  if (error instanceof Error) {
    console.error(`Extraction failed: ${error.message}`);
  }
  throw error;
}
```
