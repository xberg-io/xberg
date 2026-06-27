```typescript title="WASM - Chunking for RAG Pipeline"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 512, // Smaller chunks for vector DB efficiency
    chunkOverlap: 100,
    trim: true,
  },
  includeDocumentStructure: true,
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "application/pdf", config);

// Prepare chunks for vector database ingestion
const ragChunks =
  result.chunks?.map((chunk, idx) => ({
    id: `${result.metadata?.filename || "doc"}_chunk_${idx}`,
    text: chunk.content,
    metadata: {
      source: result.metadata?.filename,
      chunk_index: chunk.metadata?.chunkIndex,
      total_chunks: chunk.metadata?.totalChunks,
      first_page: chunk.metadata?.firstPage,
      last_page: chunk.metadata?.lastPage,
      byte_position: `${chunk.metadata?.byteStart}-${chunk.metadata?.byteEnd}`,
    },
    // Embedding would be added by vector DB embedding model
  })) || [];

console.log(`Prepared ${ragChunks.length} chunks for RAG ingestion`);
ragChunks.slice(0, 3).forEach((c) => {
  console.log(`Chunk ${c.id}: ${c.text.substring(0, 50)}...`);
});

// Example: Send to vector database (e.g., Pinecone, Weaviate, Milvus)
// const vectorResults = await vectorDb.upsert(ragChunks.map(c => ({
//   id: c.id,
//   values: await embedModel.embed(c.text),
//   metadata: c.metadata,
// })));
```

```typescript title="WASM - RAG with Retrieval Context"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 768,
    chunkOverlap: 200,
    prependHeadingContext: true, // For markdown/docs
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract(bytes, "text/markdown", config);

// Build retrieval augmented context
interface RagDocument {
  id: string;
  query_text: string;
  context: string;
  page: number | null;
}

const ragDocs: RagDocument[] =
  result.chunks?.map((chunk, idx) => ({
    id: `chunk_${idx}`,
    query_text: chunk.content, // Text to embed and search
    context: chunk.content, // Full context (includes heading)
    page: chunk.metadata?.firstPage ?? null,
  })) || [];

console.log(`Built RAG documents: ${ragDocs.length}`);
```
