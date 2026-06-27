```typescript title="WASM"
import { initWasm, extract } from "@xberg-io/xberg-wasm";

await initWasm();

const config = {
  chunking: {
    maxChars: 1000,
    chunkOverlap: 100,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

result.chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 50)}...`);
  console.log(`Tokens: ${chunk.metadata?.token_count}`);
});
```

```typescript title="WASM - Markdown with Heading Context"
import { initWasm, extract } from "@xberg-io/xberg-wasm";

await initWasm();

const config = {
  chunking: {
    chunkerType: "markdown",
    maxChars: 2000,
    // Note: Token-based sizing is not available in WASM builds.
    // Use character-based sizing instead.
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/markdown", config);

result.chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 50)}...`);

  if (chunk.metadata?.headingContext?.headings) {
    console.log("Headings:");
    chunk.metadata.headingContext.headings.forEach((h) => {
      console.log(`  Level ${h.level}: ${h.text}`);
    });
  }
});
```

```typescript title="WASM - Prepend Heading Context"
import { initWasm, extract } from "@xberg-io/xberg-wasm";

await initWasm();

const config = {
  chunking: {
    chunkerType: "markdown",
    maxChars: 2000,
    prependHeadingContext: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/markdown", config);

result.chunks?.forEach((chunk, idx) => {
  // Each chunk's content is prefixed with its heading breadcrumb
  console.log(`Chunk ${idx}: ${chunk.content.substring(0, 80)}...`);
});
```
