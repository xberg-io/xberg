# Unregister Plugins

The WASM binding provides `clear*` functions to remove all plugins of a specific type, but does not expose individual unregistration.

<!-- snippet:skip -->

WASM binding does not expose selective plugin unregistration; only bulk clearing is available via `clearOcrBackends()`, `clearPostProcessors()`, and `clearValidators()`.

```typescript title="WASM"
import init, {
  clearOcrBackends,
  clearPostProcessors,
  clearValidators,
  listPostProcessors
} from "kreuzberg-wasm";

await init();

// List current plugins before clearing
console.log("Before clearing:");
console.log("Post-processors:", listPostProcessors());

// Remove all post-processors at once
clearPostProcessors();
console.log("After clearPostProcessors():");
console.log("Post-processors:", listPostProcessors());

// If you need selective removal, re-register only the plugins you want to keep
import { registerPostProcessor } from "kreuzberg-wasm";

// Clear all
clearPostProcessors();

// Re-register only the ones you want
const processor1 = { processingStage: () => "post-extraction", process: (r) => r };
registerPostProcessor(processor1);

console.log("After selective re-registration:");
console.log("Post-processors:", listPostProcessors());
```

To remove a specific plugin, clear all and re-register only the ones you need.
