```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 1500,
    chunkOverlap: 300,
  },
  includeDocumentStructure: true,
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Map chunks back to pages for source attribution
const chunkPageMap = new Map<number, number[]>();

result.chunks?.forEach((chunk, chunkIndex) => {
  const firstPage = chunk.metadata?.firstPage;
  const lastPage = chunk.metadata?.lastPage;

  if (firstPage !== undefined && lastPage !== undefined) {
    for (let page = firstPage; page <= lastPage; page++) {
      if (!chunkPageMap.has(page)) {
        chunkPageMap.set(page, []);
      }
      chunkPageMap.get(page)!.push(chunkIndex);
    }
  }
});

// Use the mapping for source attribution
chunkPageMap.forEach((chunkIndices, pageNum) => {
  console.log(`Page ${pageNum}: Chunks ${chunkIndices.join(", ")}`);
  chunkIndices.forEach((idx) => {
    const chunk = result.chunks![idx];
    console.log(`  Content: "${chunk.content.substring(0, 60)}..."`);
  });
});
```

**Snippet:syntax-only** - Requires document structure parsing during extraction.
