/**
 * `xberg-wasm-runtime`'s default embedder is `Xenova/bge-m3` (1024-dim,
 * L2-normalized). Collections are created with this fixed dimension; the
 * worker asserts the actual embed output matches it (Task 6) so a model
 * swap fails loudly instead of deep inside `engine.ingest()`.
 */
export const EMBEDDING_DIM = 1024;

/** Must match `mcp-server/src/http/map-route.ts`'s `DOCUMENT_ID_PATTERN`. */
export const DOCUMENT_ID_PATTERN = /^[A-Za-z0-9_.-]+$/;

export const INGEST_MAX_BODY_BYTES = 10 * 1024 * 1024;
export const MAP_MAX_BODY_BYTES = 16 * 1024 * 1024;
