/**
 * Type definitions matching the wasm engine's injection contract.
 * These mirror the Rust engine's expected shapes and are validated at runtime.
 */

export interface EmbedderInterface {
  embed(texts: string[]): Promise<Float32Array[]>;
}

export interface DocumentRecord {
  documentId: string;
  sourceId: string;
  collectionId: string;
  metadata?: Record<string, unknown>;
  text?: string;
}

export interface ChunkRecord {
  sourceId: string;
  chunkIndex: number;
  text: string;
  startOffset: number;
  endOffset: number;
  embedding: Float32Array;
}

export interface VectorStoreInterface {
  upsertDocument(
    collection: string,
    doc: DocumentRecord,
    chunks: ChunkRecord[]
  ): Promise<{ documentId: string; chunksCount: number }>;
  query(
    collection: string,
    queryVector: number[],
    k: number
  ): Promise<Array<{ chunkId: string; text: string; score: number }>>;
  delete(collection: string, documentId: string): Promise<void>;
  listCollections(): Promise<string[]>;
  dropCollection(collection: string): Promise<void>;
  ensureCollection(collection: string, vectorDim: number): Promise<void>;
}

export interface Entity {
  label: string;
  text: string;
  start: number;
  end: number;
  score?: number;
}

export interface NerOpts {
  categories?: string[];
  threshold?: number;
}

export interface NerInterface {
  ner(text: string, opts?: NerOpts): Promise<Entity[]>;
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
  wasmPaths?: string; // ORT wasm binaries directory
  models?: {
    embedder?: string; // Model identifier for transformers.js
    ner?: string;
    ocr?: string;
  };
}
