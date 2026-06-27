# Unregister Plugins

Remove registered plugins from the WASM runtime using individual unregister or bulk clear operations.

```typescript title="WASM"
import init, {
  registerDocumentExtractor,
  unregisterDocumentExtractor,
  listDocumentExtractors,
  clearDocumentExtractors,
  registerOcrBackend,
  unregisterOcrBackend,
  listOcrBackends,
  clearOcrBackends,
  registerPostProcessor,
  unregisterPostProcessor,
  listPostProcessors,
  clearPostProcessors,
  registerRenderer,
  unregisterRenderer,
  listRenderers,
  clearRenderers,
  registerValidator,
  unregisterValidator,
  listValidators,
  clearValidators,
} from "xberg-wasm";

await init();

// Example: register a custom document extractor
const extractor = {
  extract: async (bytes, mimeType, config) => {
    return JSON.stringify({ text: "test", page_count: 1 });
  },
  supportedMimeTypes: () => JSON.stringify(["application/x-test"]),
};

registerDocumentExtractor(extractor);
console.log("Registered extractors:", listDocumentExtractors());

// Individual unregistration by plugin name
try {
  unregisterDocumentExtractor("wasm_bridge");
  console.log("Extractor unregistered");
} catch (error) {
  console.error("Unregister failed:", error);
}

// Clear all plugins of a type
clearPostProcessors();
console.log("After clearPostProcessors:", listPostProcessors());

clearOcrBackends();
console.log("After clearOcrBackends:", listOcrBackends());

clearRenderers();
console.log("After clearRenderers:", listRenderers());

clearValidators();
console.log("After clearValidators:", listValidators());

// Selective re-registration: clear and register only desired plugins
clearPostProcessors();
const myProcessor = {
  processingStage: () => "post-extraction",
  process: (result) => result, // Pass-through
};
registerPostProcessor(myProcessor);
console.log("After selective re-register:", listPostProcessors());

// Unregister specific plugin by name
unregisterPostProcessor("wasm_bridge");
console.log("After selective unregister:", listPostProcessors());
```

Use `unregister*` to remove individual plugins by name, or `clear*` for bulk removal of all plugins of a type. All custom plugins are registered with the default name `"wasm_bridge"` managed by the bridge.
