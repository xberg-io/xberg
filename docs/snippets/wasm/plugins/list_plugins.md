# List Registered Plugins

List all registered plugins of each type: OCR backends, post-processors, validators, and document extractors.

```typescript title="WASM"
import init, {
  listDocumentExtractors,
  listOcrBackends,
  listPostProcessors,
  listValidators
} from "kreuzberg-wasm";

await init();

// List all document extractors
const extractors = listDocumentExtractors();
console.log("Document extractors:", extractors);

// List all OCR backends
const ocrBackends = listOcrBackends();
console.log("OCR backends:", ocrBackends);

// List all post-processors
const processors = listPostProcessors();
console.log("Post-processors:", processors);

// List all validators
const validators = listValidators();
console.log("Validators:", validators);

// Count registered plugins
console.log(`Total plugins registered:
  Extractors: ${extractors.length}
  OCR backends: ${ocrBackends.length}
  Post-processors: ${processors.length}
  Validators: ${validators.length}`);
```

Use this to verify which plugins are available before extraction or to debug plugin registration issues.
