# Stateful Post-Processor Plugin

Create a stateful post-processor that maintains state across multiple extraction calls.

```typescript title="WASM"
import init, { registerPostProcessor, extract } from "xberg-wasm";

await init();

// Create a stateful post-processor using a closure
function createStatefulProcessor() {
  const state = {
    extractionCount: 0,
    totalChars: 0,
    lastResult: null,
  };

  return {
    processingStage: () => "post-extraction",
    process: (extractionResult) => {
      // Update state
      state.extractionCount++;
      state.totalChars += extractionResult.text?.length || 0;
      state.lastResult = extractionResult;

      // Enrich result with statistics
      const enriched = {
        ...extractionResult,
        metadata: {
          ...extractionResult.metadata,
          extractionIndex: state.extractionCount,
          cumulativeChars: state.totalChars,
          averageDocLength: Math.round(state.totalChars / state.extractionCount),
        },
      };

      console.log(
        `[Extraction ${state.extractionCount}] ${enriched.text?.length || 0} chars, cumulative: ${state.totalChars}`,
      );

      return enriched;
    },

    // Optional: expose state for inspection
    getState: () => state,
  };
}

// Register the stateful processor
const statefulProcessor = createStatefulProcessor();
registerPostProcessor(statefulProcessor);

// Multiple extractions use the same state
async function processMultipleDocs() {
  const docs = [
    new Uint8Array([
      /* Doc 1 */
    ]),
    new Uint8Array([
      /* Doc 2 */
    ]),
    new Uint8Array([
      /* Doc 3 */
    ]),
  ];

  const results = [];
  for (const docBytes of docs) {
    const result = await extract(docBytes, "application/pdf", {});
    results.push(result);
  }

  return results;
}

await processMultipleDocs();
```

Stateful processors can track metrics across multiple extractions or maintain context.
