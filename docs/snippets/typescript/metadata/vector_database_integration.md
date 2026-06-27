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
      max_chars: 512,
      max_overlap: 50,
      embedding: {
        model: { type: "preset", name: "balanced" },
        normalize: true,
        batchSize: 32,
      },
    },
  };

  const result = await extract(documentPath, null, config);

  const records: VectorRecord[] = [];
  if (result.chunks) {
    result.chunks.forEach((chunk, index) => {
      if (chunk.embedding) {
        records.push({
          id: `${documentId}_chunk_${index}`,
          content: chunk.content,
          embedding: chunk.embedding,
          metadata: {
            document_id: documentId,
            chunk_index: index.toString(),
            content_length: chunk.content.length.toString(),
          },
        });
      }
    });
  }

  return records;
}
```
