```typescript title="TypeScript"
import { extract, type ExtractionConfig } from "@xberg-io/xberg";

/**
 * Note: Custom cache backends are not supported in TypeScript v4.0.
 * Caching is handled internally by the Rust core.
 *
 * Use the built-in cache with the useCache configuration flag.
 */

// Enable built-in caching
const config: ExtractionConfig = {
  useCache: true,
};

const result = await extract("document.pdf", null, config);
```
