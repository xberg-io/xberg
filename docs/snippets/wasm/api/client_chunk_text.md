```typescript title="WASM"
// HTTP client approach for chunking text via the REST API
// Useful in browsers where WASM extraction is called server-side

const text = "This is a long document that needs to be split into semantic chunks.";

const response = await fetch("http://localhost:8000/chunk", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    text,
    chunker_type: "text",
    config: {
      chunking: {
        strategy: "semantic",
        max_chunk_size: 512,
        overlap: 50
      }
    }
  })
});

const result = await response.json();
console.log(`Created ${result.chunks?.length ?? 0} chunks`);
result.chunks?.forEach((chunk) => {
  console.log(`Chunk: ${chunk.content.substring(0, 50)}...`);
});
```
