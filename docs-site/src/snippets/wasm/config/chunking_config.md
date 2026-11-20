---
language: typescript
target: wasm
---

```typescript title="WASM"
import init, { WasmChunkingConfig, WasmExtractInput, WasmExtractionConfig, extract } from "@xberg-io/xberg-wasm";

await init();

const chunking = WasmChunkingConfig.default();
chunking.maxCharacters = 1000;
chunking.overlap = 100;

const config = WasmExtractionConfig.default();
config.chunking = chunking;

const buffer = await fetch("document.pdf").then((response) => response.arrayBuffer());
const bytes = new Uint8Array(buffer);
const result = await extract(WasmExtractInput.fromBytes(bytes, "application/pdf"), config);

result.results[0].chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 50)}...`);
  console.log(`Tokens: ${chunk.metadata?.tokenCount}`);
});
```

```typescript title="WASM - Markdown with Heading Context"
import init, { WasmChunkerType, WasmChunkingConfig, WasmExtractInput, WasmExtractionConfig, extract } from "@xberg-io/xberg-wasm";

await init();

const chunking = WasmChunkingConfig.default();
chunking.chunkerType = WasmChunkerType.Markdown;
// WASM uses character-based sizing.
chunking.maxCharacters = 2000;

const config = WasmExtractionConfig.default();
config.chunking = chunking;

const buffer = await fetch("document.md").then((response) => response.arrayBuffer());
const bytes = new Uint8Array(buffer);
const result = await extract(WasmExtractInput.fromBytes(bytes, "text/markdown"), config);

result.results[0].chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 50)}...`);

  if (chunk.metadata?.headingContext?.headings) {
    console.log("Headings:");
    chunk.metadata.headingContext.headings.forEach((h: any) => {
      console.log(`  Level ${h.level}: ${h.text}`);
    });
  }
});
```
