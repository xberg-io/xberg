export interface IngestChunkPayload {
  ordinal: number;
  content: string;
  embedding: number[];
  chunk_metadata?: unknown;
}

/**
 * `bbox`/`confidence` are real per-line OCR geometry (see the WASM OCR
 * bridge in `crates/xberg-wasm/src/bridge/ocr.rs`), not derived from a
 * flat-string split. `page` is optional and caller-supplied: nothing in
 * this codebase currently splits a multi-page document into per-page
 * images before calling OCR (`handleOcr` still OCRs one whole file's
 * bytes per call), so `page` stays undefined until that rasterization
 * step exists. `toParsedOcrOutput` uses `page.number`/`width`/`height`
 * per block when present and defaults to page 1 otherwise.
 */
export interface OcrLine {
  text: string;
  confidence: number;
  bbox?: { x: number; y: number; w: number; h: number };
  page?: { number: number; width: number; height: number };
}

export interface PiiDetection {
  token: string;
  category: string;
  confidence?: number;
}

/** Mirrors `mcp-server/src/http/ingest-route.ts`'s `IngestPayloadSchema`. */
export interface IngestPayload {
  collection: string;
  external_id: string;
  title?: string;
  mime?: string;
  source_uri?: string;
  full_text: string;
  keywords?: string[];
  metadata?: Record<string, unknown>;
  chunks: IngestChunkPayload[];
}

export interface CollectionPayload {
  name: string;
  embedding_dim: number;
  distance_metric?: "cosine" | "l2" | "innerproduct";
  index_method?: "flat" | "hnsw" | "diskann";
}

export type SyncStatus = "pending" | "syncing" | "synced" | "error";

/**
 * Local (IndexedDB) record of an ingested document. Never contains the
 * plaintext rehydration map — only redacted text and counts.
 */
export interface IngestHistoryEntry {
  collection: string;
  externalId: string;
  filename: string;
  mime: string;
  redactedText: string;
  piiCategoryCounts: Record<string, number>;
  documentId: string | null;
  status: SyncStatus;
  error?: string;
  ingestedAt: number;
}
