import type { VectorStoreInterface, DocumentRecord, ChunkRecord, CacheConfig } from "./types";

/**
 * Create a vector store backed by wa-sqlite (browser OPFS) or better-sqlite3 (Node).
 * Uses sqlite-vec for vector similarity if available; falls back to JS cosine distance.
 * Logs the vector search backend selection for debugging.
 */
export async function createVectorStore(
  _config?: CacheConfig
): Promise<VectorStoreInterface> {
  // For now, a simple in-memory implementation for testing.
  // Browser: wa-sqlite over OPFS in a dedicated Worker (implement via postMessage).
  // Node: better-sqlite3 or native sqlite binding.

  const collections = new Map<string, CollectionMetadata>();
  const documents = new Map<string, DocumentRecord>();
  const chunks = new Map<string, ChunkRecord[]>(); // key: sourceId

  const vectorBackend: "sqlite-vec" | "cosine" = "cosine";

  interface CollectionMetadata {
    name: string;
    vectorDim: number;
  }

  async function ensureCollection(collection: string, vectorDim: number): Promise<void> {
    if (!collections.has(collection)) {
      collections.set(collection, { name: collection, vectorDim });
    }
  }

  async function upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunkRecords: ChunkRecord[]
  ): Promise<{ documentId: string; chunksCount: number }> {
    documents.set(doc.documentId, doc);
    const key = `${collection}:${doc.sourceId}`;
    chunks.set(key, chunkRecords);
    return { documentId: doc.documentId, chunksCount: chunkRecords.length };
  }

  async function query(
    collection: string,
    queryVector: number[],
    k: number
  ): Promise<Array<{ chunkId: string; text: string; score: number }>> {
    const allChunks: Array<{
      chunkId: string;
      text: string;
      score: number;
    }> = [];

    // Iterate all chunks in the collection (simplified for in-memory)
    for (const [key, chunkList] of chunks.entries()) {
      if (key.startsWith(`${collection}:`)) {
        for (const chunk of chunkList) {
          const embeddingArr = Array.from(chunk.embedding);
          const score = cosineSimilarity(
            queryVector,
            embeddingArr
          );
          allChunks.push({
            chunkId: `${chunk.sourceId}:${chunk.chunkIndex}`,
            text: chunk.text,
            score,
          });
        }
      }
    }

    // Sort by score descending and slice to k
    return allChunks.sort((a, b) => b.score - a.score).slice(0, k);
  }

  async function deleteDocument(collection: string, documentId: string): Promise<void> {
    const doc = documents.get(documentId);
    if (doc) {
      documents.delete(documentId);
      chunks.delete(`${collection}:${doc.sourceId}`);
    }
  }

  async function listCollections(): Promise<string[]> {
    return Array.from(collections.keys());
  }

  async function dropCollection(collection: string): Promise<void> {
    collections.delete(collection);
    // Remove all chunks for this collection
    for (const key of chunks.keys()) {
      if (key.startsWith(`${collection}:`)) {
        chunks.delete(key);
      }
    }
  }

  // Log the selected vector backend
  console.debug(`[store] vector search backend: ${vectorBackend}`);

  return {
    ensureCollection,
    upsertDocument,
    query,
    delete: deleteDocument,
    listCollections,
    dropCollection,
  };
}

/**
 * Cosine similarity between two vectors.
 * Returns a score in [-1, 1]; normalized vectors return [0, 1].
 */
function cosineSimilarity(a: number[], b: number[]): number {
  if (a.length !== b.length) {
    throw new Error(`Vector dimension mismatch: ${a.length} vs ${b.length}`);
  }
  if (a.length === 0) return 0;

  let dotProduct = 0;
  let magA = 0;
  let magB = 0;

  for (let i = 0; i < a.length; i++) {
    const aVal = a[i];
    const bVal = b[i];
    if (aVal !== undefined && bVal !== undefined) {
      dotProduct += aVal * bVal;
      magA += aVal * aVal;
      magB += bVal * bVal;
    }
  }

  magA = Math.sqrt(magA);
  magB = Math.sqrt(magB);

  if (magA === 0 || magB === 0) return 0;

  return dotProduct / (magA * magB);
}
