```typescript title="TypeScript"
import {
  listDocumentExtractors,
  listOcrBackends,
  listPostProcessors,
  listValidators,
} from "@kreuzberg/node";

const extractors = listDocumentExtractors();
console.log(`Registered extractors: ${extractors.join(", ")}`);

const processors = listPostProcessors();
console.log(`Registered processors: ${processors.join(", ")}`);

const backends = listOcrBackends();
console.log(`Registered OCR backends: ${backends.join(", ")}`);

const validators = listValidators();
console.log(`Registered validators: ${validators.join(", ")}`);
```
