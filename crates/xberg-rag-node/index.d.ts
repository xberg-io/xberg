export interface EmbeddingConfig {
  preset?: string;
  dimensions?: number;
  normalize?: boolean;
}

export interface RerankerConfig {
  preset?: string;
  model?: string;
}

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

export interface ChunkRecord {
  external_id?: string;
  ordinal: number;
  content: string;
  embedding: number[];
  chunk_metadata?: unknown;
}

export interface CollectionSpec {
  name: string;
  embedding_dim: number;
  distance_metric?: string;
  index_method?: string;
}

export interface CollectionStats {
  documents: number;
  chunks: number;
  last_ingested_at?: number;
}

export interface RetrievedChunk {
  id: { 0: string };
  document_id: { 0: string };
  ordinal: number;
  external_id?: string;
  content?: string;
  score: number;
  primary_score: unknown;
  chunk_metadata?: unknown;
  document?: DocumentSummary;
}

export interface DocumentSummary {
  id: { 0: string };
  external_id?: string;
  title?: string;
  mime?: string;
  keywords: string[];
  labels: unknown;
  entities: unknown;
  metadata: unknown;
  ingested_at?: number;
}

export interface RetrieveOutput {
  mode: string;
  chunks: RetrievedChunk[];
  primary_latency_ms: number;
}

export interface RetrieveQuery {
  mode: string;
  query_text?: string;
  query_vector?: number[];
  top_k: number;
  filter?: unknown;
  candidate_multiplier?: number;
  group_by_document?: boolean;
  include_content?: boolean;
  include_document?: boolean;
  graph_depth?: number;
}

export type Filter =
  | { eq: { field: string; value: unknown } }
  | { in: { field: string; values: unknown[] } }
  | { range: { field: string; gte?: unknown; gt?: unknown; lte?: unknown; lt?: unknown } }
  | { array_contains: { field: string; value: unknown } }
  | { text_match: { field: string; query: string } }
  | { and: { filters: Filter[] } }
  | { or: { filters: Filter[] } }
  | { not: { filter: Filter } };

export class RagStore {
  static openSqlite(name: string, dbPath: string): Promise<RagStore>;
  ensureCollection(specJson: string): Promise<void>;
  dropCollection(name: string): Promise<void>;
  getCollection(name: string): Promise<string | null>;
  upsertDocument(
    collection: string,
    documentJson: string,
    chunksJson: string
  ): Promise<string>;
  retrieve(collection: string, queryJson: string): Promise<string>;
  deleteDocuments(collection: string, idsJson: string): Promise<number>;
  deleteByFilter(collection: string, filterJson: string): Promise<number>;
  collectionStats(collection: string): Promise<string>;
}

export declare function embedTexts(textsJson: string, configJson: string): Promise<string>;
export declare function rerank(
  query: string,
  documentsJson: string,
  configJson: string
): Promise<string>;
export declare function openSqlite(name: string, dbPath: string): Promise<RagStore>;