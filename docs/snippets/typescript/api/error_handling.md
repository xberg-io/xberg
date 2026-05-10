```typescript title="TypeScript"
import { extractFileSync } from "kreuzberg";

try {
  const result = extractFileSync("missing.pdf");
  console.log(result.content);
} catch (error: unknown) {
  if (error instanceof Error) {
    console.error(`Extraction failed: ${error.message}`);
  }
  throw error;
}
```
