```typescript title="TypeScript"
import { extract } from "xberg";

try {
  const result = extract("missing.pdf");
  console.log(result.content);
} catch (error: unknown) {
  if (error instanceof Error) {
    console.error(`Extraction failed: ${error.message}`);
  }
  throw error;
}
```
