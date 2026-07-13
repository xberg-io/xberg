export interface IngestChunkPayload {
  ordinal: number;
  content: string;
  embedding: number[];
  chunk_metadata?: unknown;
}

/**
 * Intentionally has no page identity or page dimensions: `engine.worker.ts`'s
 * `handleOcr` gets a flat string back from `XbergEngine.ocr` (the `@xberg-io/xberg-wasm`
 * binding doesn't expose per-line geometry or multi-page structure yet) and
 * splits it into "lines" on newlines, so every result is already scoped to
 * a single page with no real bounding boxes. Modeling `page`/dimensions here
 * would just be a field nothing populates. Add it once the WASM OCR bridge
 * returns real per-page, per-line geometry (`toParsedOcrOutput` would then
 * need to group blocks by that page identity instead of hardcoding page 1).
 */
export interface OcrLine {
  text: string;
  confidence: number;
  bbox?: { x: number; y: number; w: number; h: number };
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
