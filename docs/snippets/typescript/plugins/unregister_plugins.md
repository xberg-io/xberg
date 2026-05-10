<!-- snippet:syntax-only -->
```typescript title="TypeScript"
import {
  unregisterOcrBackend,
  unregisterPostProcessor,
  unregisterValidator,
} from "@kreuzberg/node";

// Remove plugins by their registered name.
unregisterPostProcessor("metadata-enrichment-processor");
unregisterValidator("min-length-validator");
unregisterOcrBackend("custom-ocr-backend");
```
