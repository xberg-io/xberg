```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

interface VectorRecord {
  id: string;
  content: string;
  embedding: number[];
  metadata: Record<string, string>;
}

async function extractAndVectorize(
  documentPath: string,
  documentId: string,
): Promise<VectorRecord[]> {
  const config = {
    chunking: {
      maxChars: 512,
      maxOverlap: 50,
      embedding: {
        model: { type: "preset", name: "balanced" },
        normalize: true,
        batchSize: 32,
      },
    },
  };

  const result = await extract({ kind: "uri", uri: documentPath }, config);

  const records: VectorRecord[] = [];
  for (const [index, chunk] of (result.chunks ?? []).entries()) {
    if (!chunk.embedding) {
      continue;
    }
    records.push({
      id: `${documentId}_chunk_${index}`,
      content: chunk.content,
      embedding: chunk.embedding,
      metadata: {
        document_id: documentId,
        chunk_index: String(index),
        content_length: String(chunk.content.length),
      },
    });
  }
  return records;
}

await extractAndVectorize("document.pdf", "doc_001");
```
