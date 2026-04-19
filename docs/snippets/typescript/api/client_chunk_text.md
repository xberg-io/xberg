```typescript title="TypeScript"
interface ChunkRequest {
  text: string;
  chunker_type?: "text" | "markdown" | "yaml" | "semantic";
  config?: {
    max_characters?: number;
    overlap?: number;
    trim?: boolean;
  };
}

interface ChunkItem {
  content: string;
  byte_start: number;
  byte_end: number;
  chunk_index: number;
  total_chunks: number;
  first_page: number | null;
  last_page: number | null;
}

interface ChunkResponse {
  chunks: ChunkItem[];
  chunk_count: number;
  config: {
    max_characters: number;
    overlap: number;
    trim: boolean;
    chunker_type: string;
  };
  input_size_bytes: number;
  chunker_type: string;
}

// Basic chunking
const response = await fetch("http://localhost:8000/chunk", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ text: "Your long text content here..." }),
});

const result: ChunkResponse = await response.json();
console.log(`Created ${result.chunk_count} chunks`);

// Chunking with custom configuration
const customResponse = await fetch("http://localhost:8000/chunk", {
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
  } satisfies ChunkRequest),
});

const customResult: ChunkResponse = await customResponse.json();
for (const chunk of customResult.chunks) {
  console.log(`Chunk ${chunk.chunk_index}: ${chunk.content.slice(0, 50)}...`);
}
```
