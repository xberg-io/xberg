# Plugin Logging and Debugging

Log plugin registration and execution for debugging purposes.

```typescript title="WASM"
import init, {
  registerPostProcessor,
  registerValidator,
  registerOcrBackend,
  listPostProcessors,
  listValidators,
  listOcrBackends
} from "kreuzberg-wasm";

await init();

// Track plugin registrations
const pluginLog = {
  processors: [],
  validators: [],
  ocrBackends: []
};

// Register a logging post-processor
const loggingProcessor = {
  processingStage: () => "post-extraction",
  process: (result) => {
    console.log("[POST-PROCESSOR] Processing extraction result", {
      textLength: result.text?.length,
      hasMetadata: !!result.metadata
    });
    return result;
  }
};

registerPostProcessor(loggingProcessor);
pluginLog.processors.push("loggingProcessor");

// Register a logging validator
const loggingValidator = {
  validate: (result) => {
    console.log("[VALIDATOR] Validating extraction result", {
      textLength: result.text?.length,
      isValid: true
    });
    return { valid: true, error: null };
  }
};

registerValidator(loggingValidator);
pluginLog.validators.push("loggingValidator");

// Log registered plugins
function logPluginStatus() {
  const processors = listPostProcessors();
  const validators = listValidators();
  const backends = listOcrBackends();
  
  console.log("Plugin Registration Status:", {
    postProcessors: processors,
    validators: validators,
    ocrBackends: backends,
    total: processors.length + validators.length + backends.length
  });
}

logPluginStatus();
```

Use this pattern to monitor and debug plugin lifecycle and execution.
