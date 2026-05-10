```typescript title="TypeScript"
const response = await fetch("http://localhost:8000/chunk", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    text: "Your long text content here...",
    chunker_type: "text",
    config: {
      max_characters: 1000,
      overlap: 50,
      trim: true,
    },
  }),
});

const result = await response.json();

console.log(`Created ${result.chunk_count} chunks`);
result.chunks.forEach((chunk: { content: string; chunk_index: number }) => {
  const preview = chunk.content.substring(0, 50);
  console.log(`Chunk ${chunk.chunk_index}: ${preview}...`);
});
```
