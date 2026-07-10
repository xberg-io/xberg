import type {
  VectorStoreInterface,
  CollectionSpec,
  CollectionStats,
  DistanceMetric,
  DocumentRecord,
  ChunkRecord,
  DocumentSummary,
  Filter,
  RetrieveQuery,
  RetrieveOutput,
  RetrievedChunk,
  CacheConfig,
} from "./types";

/**
 * Create a vector store backed by an in-memory JS Map + cosine similarity.
 *
 * This implements the exact JS-side contract B's Rust bridge calls
 * (`crates/xberg-wasm/src/bridge/store.rs` `JsVectorStore`, backed by
 * `xberg_rag::store::VectorStore`), mirroring the reference Rust in-memory
 * backend at `crates/xberg-rag/src/backends/memory.rs`.
 *
 * Method return conventions (see `JsVectorStore::call_method` callers):
 *  - `ensureCollection` / `dropCollection`: return `undefined` on success, or
 *    a **string** error message on failure (B treats a returned string as an
 *    error, never throws for these two).
 *  - All other methods: throw (reject) on error; the bridge propagates the
 *    rejection as a backend error. Success values are deserialized directly
 *    into the corresponding Rust type via `serde_wasm_bindgen::from_value`.
 *
 * Persistence, full-text and hybrid retrieval, and index methods other than
 * a full scan are out of scope: this backend advertises vector-only
 * capabilities (no `capabilities` property is exposed, so the bridge falls
 * back to its default capability set — see `JsVectorStore::capabilities`).
 */
export async function createVectorStore(
  _config?: CacheConfig
): Promise<VectorStoreInterface> {
  const collections = new Map<string, CollectionSpec>();
  // documentId -> { collection, document }
  const documents = new Map<string, { collection: string; record: DocumentRecord }>();
  // documentId -> chunks
  const chunksByDoc = new Map<string, StoredChunk[]>();
  // collection -> externalId -> documentId (for idempotent upserts)
  const externalIndex = new Map<string, Map<string, string>>();

  let docCounter = 0;

  interface StoredChunk {
    id: string;
    documentId: string;
    ordinal: number;
    content: string;
    embedding: number[];
    externalId?: string;
    chunkMetadata: unknown;
  }

  function nextDocId(): string {
    docCounter += 1;
    return `wasm-store-doc-${docCounter}`;
  }

  function requireCollection(collection: string): CollectionSpec {
    const spec = collections.get(collection);
    if (!spec) {
      throw new Error(`collection not found: ${collection}`);
    }
    return spec;
  }

  function collectionChunks(collection: string): StoredChunk[] {
    const out: StoredChunk[] = [];
    for (const [docId, chunks] of chunksByDoc.entries()) {
      const doc = documents.get(docId);
      if (doc && doc.collection === collection) {
        out.push(...chunks);
      }
    }
    return out;
  }

  function summarize(documentId: string, doc: DocumentRecord): DocumentSummary {
    return {
      id: documentId,
      external_id: doc.external_id,
      title: doc.title,
      mime: doc.mime,
      keywords: doc.keywords ?? [],
      labels: doc.labels ?? null,
      entities: doc.entities ?? null,
      metadata: doc.metadata ?? null,
      ingested_at: undefined,
    };
  }

  async function ensureCollection(spec: CollectionSpec): Promise<string | void> {
    try {
      const existing = collections.get(spec.name);
      if (existing && existing.embedding_dim !== spec.embedding_dim) {
        return `collection already exists: ${spec.name}`;
      }
      collections.set(spec.name, {
        name: spec.name,
        embedding_dim: spec.embedding_dim,
        distance_metric: spec.distance_metric ?? "cosine",
        index_method: spec.index_method ?? "flat",
      });
      return undefined;
    } catch (err) {
      return err instanceof Error ? err.message : String(err);
    }
  }

  async function dropCollection(collection: string): Promise<string | void> {
    if (!collections.has(collection)) {
      return `collection not found: ${collection}`;
    }
    collections.delete(collection);
    for (const [docId, doc] of Array.from(documents.entries())) {
      if (doc.collection === collection) {
        documents.delete(docId);
        chunksByDoc.delete(docId);
      }
    }
    externalIndex.delete(collection);
    return undefined;
  }

  async function getCollection(collection: string): Promise<CollectionSpec | null> {
    return collections.get(collection) ?? null;
  }

  async function upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunks: ChunkRecord[]
  ): Promise<string> {
    const spec = requireCollection(collection);
    for (const chunk of chunks) {
      if (chunk.embedding.length !== spec.embedding_dim) {
        throw new Error(
          `embedding dimension mismatch: expected ${spec.embedding_dim}, got ${chunk.embedding.length}`
        );
      }
    }

    let collExternalIndex = externalIndex.get(collection);
    if (!collExternalIndex) {
      collExternalIndex = new Map();
      externalIndex.set(collection, collExternalIndex);
    }

    let docId: string;
    const existingId = doc.external_id ? collExternalIndex.get(doc.external_id) : undefined;
    if (existingId) {
      docId = existingId;
      chunksByDoc.delete(docId);
    } else {
      docId = nextDocId();
    }

    if (doc.external_id) {
      collExternalIndex.set(doc.external_id, docId);
    }

    documents.set(docId, { collection, record: doc });
    const storedChunks: StoredChunk[] = chunks.map((c) => ({
      id: `${docId}:${c.ordinal}`,
      documentId: docId,
      ordinal: c.ordinal,
      content: c.content,
      embedding: c.embedding,
      externalId: c.external_id,
      chunkMetadata: c.chunk_metadata ?? null,
    }));
    chunksByDoc.set(docId, storedChunks);

    return docId;
  }

  async function deleteDocuments(collection: string, ids: string[]): Promise<number> {
    requireCollection(collection);
    let removed = 0;
    for (const id of ids) {
      // The MCP delete_documents tool accepts "Document IDs or external IDs".
      // Internal ids hit `documents` directly; otherwise resolve the id through
      // this collection's external-id index before deleting.
      const resolvedId = documents.has(id)
        ? id
        : externalIndex.get(collection)?.get(id);
      if (!resolvedId) continue;

      const doc = documents.get(resolvedId);
      if (doc && doc.collection === collection) {
        documents.delete(resolvedId);
        chunksByDoc.delete(resolvedId);
        if (doc.record.external_id) {
          externalIndex.get(collection)?.delete(doc.record.external_id);
        }
        removed += 1;
      }
    }
    return removed;
  }

  async function deleteByFilter(collection: string, filter: Filter): Promise<number> {
    requireCollection(collection);
    const toRemove: string[] = [];
    for (const [docId, doc] of documents.entries()) {
      if (doc.collection !== collection) continue;
      const chunks = chunksByDoc.get(docId) ?? [];
      const matches = chunks.some((c) => evalFilter(filter, doc.record, c));
      if (matches) {
        toRemove.push(docId);
      }
    }
    let removed = 0;
    for (const id of toRemove) {
      const doc = documents.get(id);
      documents.delete(id);
      chunksByDoc.delete(id);
      if (doc?.record.external_id) {
        externalIndex.get(collection)?.delete(doc.record.external_id);
      }
      removed += 1;
    }
    return removed;
  }

  async function retrieve(collection: string, query: RetrieveQuery): Promise<RetrieveOutput> {
    const mode = query.mode ?? "vector";
    if (mode !== "vector") {
      throw new Error(`retrieval mode unsupported by backend 'wasm-runtime-in-memory': ${mode}`);
    }
    const spec = requireCollection(collection);

    const topK = query.top_k;
    if (!Number.isInteger(topK) || topK < 1 || topK > 200) {
      throw new Error("invalid query: top_k must be between 1 and 200");
    }

    const queryVector = query.query_vector;
    if (!queryVector) {
      throw new Error("invalid query: in-memory backend cannot embed text; supply query_vector");
    }
    if (queryVector.length !== spec.embedding_dim) {
      throw new Error(
        `embedding dimension mismatch: expected ${spec.embedding_dim}, got ${queryVector.length}`
      );
    }

    const allChunks = collectionChunks(collection).filter((c) => {
      if (!query.filter) return true;
      const doc = documents.get(c.documentId);
      return doc ? evalFilter(query.filter, doc.record, c) : false;
    });

    let scored: RetrievedChunk[] = allChunks.map((c) => {
      const s = scoreByMetric(spec.distance_metric ?? "cosine", queryVector, c.embedding);
      const doc = documents.get(c.documentId);
      return {
        id: c.id,
        document_id: c.documentId,
        ordinal: c.ordinal,
        external_id: c.externalId,
        content: query.include_content ? c.content : undefined,
        score: s,
        // `PrimaryScore::Vector` is a struct variant `{ score }` under the
        // internally-tagged (`tag = "kind"`) enum, so the wire shape is
        // `{ kind: "vector", score }` — the fields flatten alongside the tag.
        primary_score: { kind: "vector", score: s },
        chunk_metadata: c.chunkMetadata,
        document:
          query.include_document && doc ? summarize(c.documentId, doc.record) : undefined,
      };
    });

    scored.sort((a, b) => b.score - a.score);

    if (query.group_by_document) {
      const seen = new Set<string>();
      scored = scored.filter((c) => {
        if (seen.has(c.document_id)) return false;
        seen.add(c.document_id);
        return true;
      });
    }

    scored = scored.slice(0, topK);

    return {
      mode: "vector",
      chunks: scored,
      primary_latency_ms: 0,
    };
  }

  async function collectionStats(collection: string): Promise<CollectionStats> {
    requireCollection(collection);
    let docCount = 0;
    let chunkCount = 0;
    for (const doc of documents.values()) {
      if (doc.collection === collection) docCount += 1;
    }
    for (const [docId, chunks] of chunksByDoc.entries()) {
      const doc = documents.get(docId);
      if (doc && doc.collection === collection) chunkCount += chunks.length;
    }
    return {
      documents: docCount,
      chunks: chunkCount,
      last_ingested_at: undefined,
    };
  }

  return {
    ensureCollection,
    dropCollection,
    getCollection,
    upsertDocument,
    deleteDocuments,
    deleteByFilter,
    retrieve,
    collectionStats,
  };
}

/**
 * Resolve a filter field to a value within a (document, chunk) context.
 * Mirrors `resolve_field` in `crates/xberg-rag/src/backends/memory.rs`.
 */
function resolveField(
  fieldPath: string,
  doc: DocumentRecord,
  chunk: { content: string; ordinal: number; externalId?: string; chunkMetadata: unknown }
): unknown {
  if (fieldPath.startsWith("doc.")) {
    const path = fieldPath.slice("doc.".length);
    switch (path) {
      case "full_text":
        return doc.full_text;
      case "title":
        return doc.title;
      case "mime":
        return doc.mime;
      case "external_id":
        return doc.external_id;
      case "source_uri":
        return doc.source_uri;
      case "keywords":
        return doc.keywords ?? [];
      case "labels":
        return doc.labels;
      case "entities":
        return doc.entities;
      default:
        if (path.startsWith("metadata.")) {
          return jsonPointer(doc.metadata, path.slice("metadata.".length));
        }
        return undefined;
    }
  } else if (fieldPath.startsWith("chunk.")) {
    const path = fieldPath.slice("chunk.".length);
    switch (path) {
      case "content":
        return chunk.content;
      case "ordinal":
        return chunk.ordinal;
      case "external_id":
        return chunk.externalId;
      default:
        if (path.startsWith("chunk_metadata.")) {
          return jsonPointer(chunk.chunkMetadata, path.slice("chunk_metadata.".length));
        }
        return undefined;
    }
  }
  return undefined;
}

function jsonPointer(value: unknown, dotted: string): unknown {
  let cur: unknown = value;
  for (const segment of dotted.split(".")) {
    if (cur === null || cur === undefined || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[segment];
  }
  return cur;
}

function jsonEquals(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function jsonCmp(a: unknown, b: unknown): number | undefined {
  if (typeof a === "number" && typeof b === "number") {
    return a - b;
  }
  if (typeof a === "string" && typeof b === "string") {
    return a < b ? -1 : a > b ? 1 : 0;
  }
  return undefined;
}

function evalFilter(
  filter: Filter,
  doc: DocumentRecord,
  chunk: { content: string; ordinal: number; externalId?: string; chunkMetadata: unknown }
): boolean {
  if ("eq" in filter) {
    const v = resolveField(filter.eq.field, doc, chunk);
    return v !== undefined && jsonEquals(v, filter.eq.value);
  }
  if ("in" in filter) {
    const v = resolveField(filter.in.field, doc, chunk);
    return v !== undefined && filter.in.values.some((candidate) => jsonEquals(candidate, v));
  }
  if ("array_contains" in filter) {
    const v = resolveField(filter.array_contains.field, doc, chunk);
    return Array.isArray(v) && v.some((item) => jsonEquals(item, filter.array_contains.value));
  }
  if ("range" in filter) {
    const { field, gte, gt, lte, lt } = filter.range;
    const v = resolveField(field, doc, chunk);
    if (v === undefined) return false;
    const pass = (bound: unknown, cmp: (ord: number) => boolean): boolean => {
      if (bound === undefined) return true;
      const ord = jsonCmp(v, bound);
      return ord !== undefined && cmp(ord);
    };
    return (
      pass(gte, (o) => o >= 0) &&
      pass(gt, (o) => o > 0) &&
      pass(lte, (o) => o <= 0) &&
      pass(lt, (o) => o < 0)
    );
  }
  if ("text_match" in filter) {
    const v = resolveField(filter.text_match.field, doc, chunk);
    return typeof v === "string" && v.toLowerCase().includes(filter.text_match.query.toLowerCase());
  }
  if ("and" in filter) {
    return filter.and.filters.every((f) => evalFilter(f, doc, chunk));
  }
  if ("or" in filter) {
    return filter.or.filters.some((f) => evalFilter(f, doc, chunk));
  }
  if ("not" in filter) {
    return !evalFilter(filter.not.filter, doc, chunk);
  }
  return false;
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

/**
 * Score a query vector against a candidate embedding under a collection's
 * distance metric. Mirrors `score()` in
 * `crates/xberg-rag/src/backends/memory.rs`: higher is always more relevant, so
 * the L2 branch returns the negated Euclidean distance and all three metrics
 * sort correctly under a descending `score` ordering.
 */
function scoreByMetric(metric: DistanceMetric, a: number[], b: number[]): number {
  switch (metric) {
    case "innerproduct": {
      if (a.length !== b.length) {
        throw new Error(`Vector dimension mismatch: ${a.length} vs ${b.length}`);
      }
      let dot = 0;
      for (let i = 0; i < a.length; i++) {
        dot += (a[i] as number) * (b[i] as number);
      }
      return dot;
    }
    case "l2": {
      if (a.length !== b.length) {
        throw new Error(`Vector dimension mismatch: ${a.length} vs ${b.length}`);
      }
      let d2 = 0;
      for (let i = 0; i < a.length; i++) {
        const diff = (a[i] as number) - (b[i] as number);
        d2 += diff * diff;
      }
      return -Math.sqrt(d2);
    }
    case "cosine":
    default:
      return cosineSimilarity(a, b);
  }
}
