```typescript title="disk_cache.ts"
/* oxlint-disable */
import { extractFile, type ExtractionConfig } from "@kreuzberg/node";

const config: ExtractionConfig = { useCache: true };

(async () => {
  console.log("First extraction (will be cached)...");
  const result1 = await extractFile("document.pdf", null, config);
  const length1 = result1.content.length;
  console.log("  - Content length: " + length1);

  console.log("\nSecond extraction (from cache)...");
  const result2 = await extractFile("document.pdf", null, config);
  const length2 = result2.content.length;
  console.log("  - Content length: " + length2);

  const isIdentical = result1.content === result2.content;
  console.log("\nResults are identical: " + isIdentical);
})();
```;
