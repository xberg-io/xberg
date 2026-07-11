export function sanitizeTableName(collection: string): string {
	return collection.replace(/[^a-zA-Z0-9_]/g, "_").slice(0, 48);
}

function collectionHash(collection: string): string {
	let hash = 0x811c9dc5;
	for (let index = 0; index < collection.length; index += 1) {
		hash ^= collection.charCodeAt(index);
		hash = Math.imul(hash, 0x01000193);
	}
	return (hash >>> 0).toString(16).padStart(8, "0");
}
export function vecTableName(collection: string): string {
	return `vec_${sanitizeTableName(collection)}_${collectionHash(collection)}`;
}
const MAX_VECTOR_DIMENSION = 65_536;

export function createVecTableSql(collection: string, vectorDim: number): string {
	if (!Number.isSafeInteger(vectorDim) || vectorDim < 1 || vectorDim > MAX_VECTOR_DIMENSION) {
		throw new RangeError(`vector dimension must be an integer from 1 to ${MAX_VECTOR_DIMENSION}`);
	}
	const table = vecTableName(collection);
	return `CREATE VIRTUAL TABLE IF NOT EXISTS ${table} USING vec0(chunk_id TEXT PRIMARY KEY, embedding FLOAT[${vectorDim}])`;
}

// Column layout mirrors `xberg_rag::types::{CollectionSpec, DocumentRecord,
// ChunkRecord}` (packages/xberg-wasm-runtime/src/types.ts), not the old
// pre-wire-alignment DocumentRecord/ChunkRecord shape. JSON-typed columns
// (keywords/entities/labels/metadata/chunk_metadata) store `JSON.stringify`d
// values and are parsed back out on read.
/**
 * Current persisted-schema version. Bump this and add a branch in the store
 * init paths (store-node.ts / store-worker.ts) when the SQL layout changes, so
 * existing databases are migrated in place instead of being left on a stale
 * layout that newer queries (e.g. `embedding_dim`, `full_text`, `content`)
 * would fail against.
 */
export const SCHEMA_VERSION = 1;

export const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS collections (
  name TEXT PRIMARY KEY,
  sanitized_name TEXT NOT NULL,
  embedding_dim INTEGER NOT NULL,
  distance_metric TEXT NOT NULL DEFAULT 'cosine',
  index_method TEXT NOT NULL DEFAULT 'flat'
);
CREATE TABLE IF NOT EXISTS documents (
  document_id TEXT NOT NULL,
  collection TEXT NOT NULL,
  external_id TEXT,
  title TEXT,
  mime TEXT,
  source_uri TEXT,
  full_text TEXT NOT NULL,
  keywords TEXT,
  entities TEXT,
  labels TEXT,
  metadata TEXT,
  ingested_at INTEGER NOT NULL,
  PRIMARY KEY (collection, document_id)
);
CREATE INDEX IF NOT EXISTS idx_documents_collection ON documents(collection);
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_external ON documents(collection, external_id) WHERE external_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS chunks (
  chunk_id TEXT NOT NULL,
  collection TEXT NOT NULL,
  document_id TEXT NOT NULL,
  ordinal INTEGER NOT NULL,
  external_id TEXT,
  content TEXT NOT NULL,
  chunk_metadata TEXT,
  PRIMARY KEY (collection, chunk_id)
);
CREATE INDEX IF NOT EXISTS idx_chunks_collection ON chunks(collection);
CREATE INDEX IF NOT EXISTS idx_chunks_document ON chunks(document_id);
CREATE TABLE IF NOT EXISTS graph_edges (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  target TEXT NOT NULL,
  label TEXT,
  properties TEXT
);
CREATE INDEX IF NOT EXISTS idx_edges_source ON graph_edges(source);
CREATE INDEX IF NOT EXISTS idx_edges_target ON graph_edges(target);
CREATE INDEX IF NOT EXISTS idx_edges_label ON graph_edges(label);
CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
  chunk_id UNINDEXED,
  collection UNINDEXED,
  content,
  content='chunks',
  content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN
  INSERT INTO chunks_fts(rowid, chunk_id, collection, content)
  VALUES (new.rowid, new.chunk_id, new.collection, new.content);
END;
CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, chunk_id, collection, content)
  VALUES ('delete', old.rowid, old.chunk_id, old.collection, old.content);
END;
CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE ON chunks BEGIN
  INSERT INTO chunks_fts(chunks_fts, rowid, chunk_id, collection, content)
  VALUES ('delete', old.rowid, old.chunk_id, old.collection, old.content);
  INSERT INTO chunks_fts(rowid, chunk_id, collection, content)
  VALUES (new.rowid, new.chunk_id, new.collection, new.content);
END;
`;
