```typescript title="WASM - Fixed-Size Chunks"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 2000,
    chunkOverlap: 400,
    trim: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

result.chunks?.forEach((chunk, idx) => {
  console.log(`Chunk ${chunk.metadata?.chunkIndex}/${chunk.metadata?.totalChunks}`);
  console.log(`  Position: ${chunk.metadata?.byteStart}-${chunk.metadata?.byteEnd}`);
  console.log(`  Content: "${chunk.content.substring(0, 50)}..."`);
});
```

```typescript title="WASM - Markdown-Aware Chunking"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    chunkerType: "markdown",
    maxChars: 1500,
    prependHeadingContext: true,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/markdown", config);

result.chunks?.forEach((chunk) => {
  // Content already includes heading context prepended
  console.log(chunk.content.substring(0, 80));
  console.log(
    `  Heading path: ${chunk.metadata?.headingContext?.headings?.map((h) => `${"#".repeat(h.level)} ${h.text}`).join(" > ")}`,
  );
});
```

```typescript title="WASM - Semantic Chunking with Topic Threshold"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    chunkerType: "semantic",
    maxChars: 1000,
    topicThreshold: 0.5, // Boundary detection at 50% topic change
    chunkOverlap: 100,
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/markdown", config);

console.log(`Generated ${result.chunks?.length} semantic chunks`);
result.chunks?.forEach((chunk) => {
  console.log(`Chunk ${chunk.metadata?.chunkIndex}: ${chunk.content.length} chars`);
});
```
