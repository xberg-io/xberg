/**
 * Type definitions matching the wasm engine's injection contract.
 * These mirror the Rust engine's expected shapes and are validated at runtime.
 */

export interface EmbedderInterface {
  embed(texts: string[]): Promise<Float32Array[]>;
}

/**
 * Distance metric for similarity computation.
 * Mirrors `xberg_rag::types::DistanceMetric` (`#[serde(rename_all = "lowercase")]`).
 */
export type DistanceMetric = "cosine" | "l2" | "innerproduct";

/**
 * Vector index method hint.
 * Mirrors `xberg_rag::types::IndexMethod` (`#[serde(rename_all = "snake_case")]`).
 */
export type IndexMethod = "flat" | "hnsw" | "diskann";

/**
 * Collection configuration specification.
 * Mirrors `xberg_rag::types::CollectionSpec` (plain struct, snake_case fields).
 */
export interface CollectionSpec {
  name: string;
  embedding_dim: number;
  distance_metric?: DistanceMetric;
  index_method?: IndexMethod;
}

/**
 * Document record for upsertion.
 * Mirrors `xberg_rag::types::DocumentRecord` (plain struct, snake_case fields).
 */
export interface DocumentRecord {
  external_id?: string;
  title?: string;
  mime?: string;
  source_uri?: string;
  full_text: string;
  keywords?: string[];
  entities?: unknown;
  labels?: unknown;
  metadata?: unknown;
}

/**
 * Chunk record for upsertion.
 * Mirrors `xberg_rag::types::ChunkRecord` (plain struct, snake_case fields).
 */
export interface ChunkRecord {
  external_id?: string;
  ordinal: number;
  content: string;
  embedding: number[];
  chunk_metadata?: unknown;
}

/**
 * A directed graph edge, mirrored alongside vectors in the SQLite-backed
 * stores. Mirrors `crates/xberg-rag/src/backends/graphqlite.rs`'s
 * `_graph_edges(id, source, target, label, properties)` shape (fork-local —
 * not part of `xberg_rag::types`, and not on `VectorStoreInterface`: the
 * canonical, factory-validated interface has no graph capability, and
 * `validateInjectionDescriptor`'s zod schema would strip these methods if
 * they were added there. `createEdge`/`traverseGraph` are extras on the
 * concrete `NodeVectorStore`/`BrowserVectorStore` return types instead —
 * see `store-node.ts`.
 */
export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  properties?: Record<string, unknown>;
}

/**
 * Document summary attached to retrieval results.
 * Mirrors `xberg_rag::types::DocumentSummary`.
 */
export interface DocumentSummary {
  id: string;
  external_id?: string;
  title?: string;
  mime?: string;
  keywords?: string[];
  labels?: unknown;
  entities?: unknown;
  metadata?: unknown;
  ingested_at?: number;
}

/**
 * How a retrieved chunk was primarily scored.
 * Mirrors `xberg_rag::types::PrimaryScore` (internally tagged, `tag = "kind"`,
 * `#[serde(rename_all = "snake_case")]`).
 */
export type PrimaryScore =
  | { kind: "vector"; score: number }
  | { kind: "full_text"; score: number }
  | { kind: "hybrid"; vector: number; full_text: number; rrf: number };

/**
 * A chunk returned from a retrieval query.
 * Mirrors `xberg_rag::types::RetrievedChunk`.
 */
export interface RetrievedChunk {
  id: string;
  document_id: string;
  ordinal: number;
  external_id?: string;
  content?: string;
  score: number;
  primary_score: PrimaryScore;
  chunk_metadata?: unknown;
  document?: DocumentSummary;
}

/**
 * Aggregate statistics for a collection.
 * Mirrors `xberg_rag::types::CollectionStats`.
 */
export interface CollectionStats {
  documents: number;
  chunks: number;
  last_ingested_at?: number;
}

/**
 * Retrieval mode.
 * Mirrors `xberg_rag::query::RetrieveMode` (`#[serde(rename_all = "lowercase")]`,
 * with `FullText` explicitly renamed to `"full_text"`).
 */
export type RetrieveMode = "vector" | "full_text" | "hybrid" | "graph";

/**
 * A filter field identifier (`doc.title`, `chunk.content`, `doc.metadata.x`).
 * Mirrors `xberg_rag::filter::FilterField` (newtype around a plain string).
 */
export type FilterField = string;

/**
 * A filter expression for constraining retrieval and deletion.
 * Mirrors `xberg_rag::filter::Filter` (externally tagged enum,
 * `#[serde(rename_all = "snake_case")]`).
 */
export type Filter =
  | { eq: { field: FilterField; value: unknown } }
  | { in: { field: FilterField; values: unknown[] } }
  | {
      range: {
        field: FilterField;
        gte?: unknown;
        gt?: unknown;
        lte?: unknown;
        lt?: unknown;
      };
    }
  | { array_contains: { field: FilterField; value: unknown } }
  | { text_match: { field: FilterField; query: string } }
  | { and: { filters: Filter[] } }
  | { or: { filters: Filter[] } }
  | { not: { filter: Filter } };

/**
 * A retrieval query.
 * Mirrors `xberg_rag::query::RetrieveQuery`.
 */
export interface RetrieveQuery {
  mode?: RetrieveMode;
  query_text?: string;
  query_vector?: number[];
  top_k: number;
  filter?: Filter;
  candidate_multiplier?: number;
  group_by_document?: boolean;
  include_content?: boolean;
  include_document?: boolean;
  graph_depth?: number;
}

/**
 * The output of a retrieval query.
 * Mirrors `xberg_rag::query::RetrieveOutput`.
 */
export interface RetrieveOutput {
  mode: RetrieveMode;
  chunks: RetrievedChunk[];
  primary_latency_ms?: number;
}

/**
 * The JS-side vector store contract called by B's Rust bridge
 * (`crates/xberg-wasm/src/bridge/store.rs` `JsVectorStore`) via
 * `serde_wasm_bindgen`. See that file and `xberg_rag::store::VectorStore` for
 * the source-of-truth shapes. Methods that "return an error" per the Rust
 * bridge convention do so by **returning a string** (never throwing) for
 * `ensureCollection`/`dropCollection`; other methods should throw on error
 * (the bridge propagates thrown/rejected promises as backend errors).
 */
export interface VectorStoreInterface {
  /** Release any native/worker resources held by the store. Safe to call more than once. */
  close(): Promise<void>;
  /** Returns undefined on success, or an error message string on failure. */
  ensureCollection(spec: CollectionSpec): Promise<string | void>;
  /** Returns undefined on success, or an error message string on failure. */
  dropCollection(collection: string): Promise<string | void>;
  getCollection(collection: string): Promise<CollectionSpec | null | undefined>;
  /** Returns a bare document id string (deserialized into `DocumentId`). */
  upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunks: ChunkRecord[]
  ): Promise<string>;
  /** Returns the count of documents deleted. */
  deleteDocuments(collection: string, ids: string[]): Promise<number>;
  /** Returns the count of documents deleted. */
  deleteByFilter(collection: string, filter: Filter): Promise<number>;
  retrieve(collection: string, query: RetrieveQuery): Promise<RetrieveOutput>;
  collectionStats(collection: string): Promise<CollectionStats>;
}

export interface Entity {
  label: string;
  text: string;
  start: number;
  end: number;
  score?: number;
}

export interface NerInterface {
  /**
   * `categories` is a plain positional array (not an options object) because
   * this must match `crates/xberg-wasm/src/bridge/ner.rs`'s
   * `call_injected_ner`, which calls `ner(text, categories)` positionally —
   * the Rust bridge is the fixed contract this signature exists to satisfy.
   * `threshold` is accepted for callers that filter client-side; the Rust
   * bridge itself never passes it.
   */
  ner(text: string, categories?: string[], threshold?: number): Promise<Entity[]>;
}

export interface OcrOpts {
  languages?: string[];
  useCpu?: boolean;
}

export interface OcrResult {
  text: string;
  lines: Array<{
    text: string;
    confidence: number;
    bbox?: { x: number; y: number; w: number; h: number };
  }>;
}

export interface OcrInterface {
  ocr(bytes: Uint8Array, opts?: OcrOpts): Promise<OcrResult>;
}

export interface InjectionDescriptor {
  embedder: EmbedderInterface;
  store: VectorStoreInterface;
  ner?: NerInterface;
  ocr?: OcrInterface;
}

export interface CacheConfig {
  opfsPath?: string; // Browser OPFS mount point
  nodeCachePath?: string; // Node ~/.cache/xberg path
  nodeStorePath?: string; // Node SQLite file path; defaults inside nodeCachePath
  wasmPaths?: string; // ORT wasm binaries directory
  forceWasmBackend?: boolean; // Skip WebGPU detection; use the WASM-CPU backend
  models?: {
    embedder?: string; // Model identifier for transformers.js
    ner?: string;
    ocr?: string;
  };
}
