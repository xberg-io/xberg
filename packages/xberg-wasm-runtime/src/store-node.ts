import Database from "better-sqlite3";
import * as sqliteVec from "sqlite-vec";
import { randomUUID } from "node:crypto";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { SCHEMA_SQL, SCHEMA_VERSION, createVecTableSql, vecTableName, sanitizeTableName } from "./store-schema.js";
import { evalFilter } from "./filter-eval.js";
import type {
	VectorStoreInterface,
	CollectionSpec,
	CollectionStats,
	DistanceMetric,
	DocumentRecord,
	ChunkRecord,
	DocumentSummary,
	Filter,
	GraphEdge,
	RetrieveQuery,
	RetrieveOutput,
	RetrievedChunk,
	CacheConfig,
} from "./types.js";

/**
 * `VectorStoreInterface` (the canonical, factory-validated contract) has no
 * `close()`, `createEdge`, or `traverseGraph` — the in-memory reference
 * backend has no state to release and no graph capability, and
 * `validateInjectionDescriptor`'s zod schema strips unknown keys, so adding
 * them there would be silently dropped anyway. This SQLite-backed store
 * genuinely owns a native handle and mirrors
 * `crates/xberg-rag/src/backends/graphqlite.rs`'s `_graph_edges` table
 * alongside vectors (see `docs/superpowers/plans/2026-07-07-xberg-wasm-sqlite-vec-store-and-perf.md`),
 * so callers that construct it directly (tests, lifecycle-aware hosts) get
 * these as extras.
 */
export type NodeVectorStore = VectorStoreInterface & {
	close(): Promise<void>;
	createEdge(edge: GraphEdge): Promise<void>;
	traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]>;
};

const HYBRID_CANDIDATE_MULTIPLIER = 4;

interface CollectionRow {
	embedding_dim: number;
	distance_metric: DistanceMetric;
	index_method: CollectionSpec["index_method"];
}

interface DocumentRow {
	document_id: string;
	external_id: string | null;
	title: string | null;
	mime: string | null;
	source_uri: string | null;
	full_text: string;
	keywords: string | null;
	entities: string | null;
	labels: string | null;
	metadata: string | null;
	ingested_at: number;
}

interface ChunkRow {
	chunk_id: string;
	document_id: string;
	ordinal: number;
	external_id: string | null;
	content: string;
	chunk_metadata: string | null;
}

function toDocumentRecord(row: DocumentRow): DocumentRecord {
	return {
		external_id: row.external_id ?? undefined,
		title: row.title ?? undefined,
		mime: row.mime ?? undefined,
		source_uri: row.source_uri ?? undefined,
		full_text: row.full_text,
		keywords: row.keywords ? JSON.parse(row.keywords) : undefined,
		entities: row.entities ? JSON.parse(row.entities) : undefined,
		labels: row.labels ? JSON.parse(row.labels) : undefined,
		metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
	};
}

function toDocumentSummary(row: DocumentRow): DocumentSummary {
	return {
		id: row.document_id,
		external_id: row.external_id ?? undefined,
		title: row.title ?? undefined,
		mime: row.mime ?? undefined,
		keywords: row.keywords ? JSON.parse(row.keywords) : [],
		labels: row.labels ? JSON.parse(row.labels) : undefined,
		entities: row.entities ? JSON.parse(row.entities) : undefined,
		metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
		ingested_at: row.ingested_at,
	};
}

export async function createNodeVectorStore(config?: CacheConfig): Promise<NodeVectorStore> {
	const dbPath =
		config?.nodeStorePath ?? (config?.nodeCachePath ? join(config.nodeCachePath, "store.sqlite3") : ":memory:");
	if (dbPath !== ":memory:") {
		mkdirSync(dirname(dbPath), { recursive: true });
	}
	const db = new Database(dbPath);
	try {
		sqliteVec.load(db);
		db.pragma("journal_mode = WAL");
		// Versioned migration: only (re)apply the schema when the persisted
		// database is on an older layout, then record the current version.
		// `SCHEMA_SQL` uses `IF NOT EXISTS`, so re-applying is idempotent and
		// never drops existing data.
		if (Number(db.pragma("user_version")) < SCHEMA_VERSION) {
			db.exec(SCHEMA_SQL);
			db.pragma(`user_version = ${SCHEMA_VERSION}`);
		}
	} catch (error) {
		db.close();
		throw error;
	}

	function getCollectionRow(collection: string): CollectionRow | undefined {
		return db
			.prepare("SELECT embedding_dim, distance_metric, index_method FROM collections WHERE name = ?")
			.get(collection) as CollectionRow | undefined;
	}

	function requireCollectionRow(collection: string): CollectionRow {
		const row = getCollectionRow(collection);
		if (!row) throw new Error(`collection not found: ${collection}`);
		return row;
	}

	async function ensureCollection(spec: CollectionSpec): Promise<string | void> {
		try {
			if (!spec.name.trim()) return "collection name must not be empty";
			const existing = getCollectionRow(spec.name);
			if (existing) {
				if (existing.embedding_dim !== spec.embedding_dim) {
					return `collection already exists: ${spec.name}`;
				}
				return undefined;
			}
			db.prepare(
				"INSERT INTO collections (name, sanitized_name, embedding_dim, distance_metric, index_method) VALUES (?, ?, ?, ?, ?)",
			).run(
				spec.name,
				sanitizeTableName(spec.name),
				spec.embedding_dim,
				spec.distance_metric ?? "cosine",
				spec.index_method ?? "flat",
			);
			db.exec(createVecTableSql(spec.name, spec.embedding_dim));
			return undefined;
		} catch (err) {
			return err instanceof Error ? err.message : String(err);
		}
	}

	async function dropCollection(collection: string): Promise<string | void> {
		if (!getCollectionRow(collection)) return `collection not found: ${collection}`;
		const table = vecTableName(collection);
		const tx = db.transaction(() => {
			db.exec(`DROP TABLE IF EXISTS ${table}`);
			db.prepare("DELETE FROM chunks WHERE collection = ?").run(collection);
			db.prepare("DELETE FROM documents WHERE collection = ?").run(collection);
			db.prepare("DELETE FROM collections WHERE name = ?").run(collection);
		});
		tx();
		return undefined;
	}

	async function getCollection(collection: string): Promise<CollectionSpec | null> {
		const row = getCollectionRow(collection);
		if (!row) return null;
		return {
			name: collection,
			embedding_dim: row.embedding_dim,
			distance_metric: row.distance_metric,
			index_method: row.index_method,
		};
	}

	async function upsertDocument(collection: string, doc: DocumentRecord, chunks: ChunkRecord[]): Promise<string> {
		const spec = requireCollectionRow(collection);
		for (const chunk of chunks) {
			if (chunk.embedding.length !== spec.embedding_dim) {
				throw new Error(`embedding dimension mismatch: expected ${spec.embedding_dim}, got ${chunk.embedding.length}`);
			}
		}

		const table = vecTableName(collection);
		const existing = doc.external_id
			? (db
					.prepare("SELECT document_id FROM documents WHERE collection = ? AND external_id = ?")
					.get(collection, doc.external_id) as { document_id: string } | undefined)
			: undefined;
		const documentId = existing?.document_id ?? randomUUID();

		const insertDoc = db.prepare(
			`INSERT OR REPLACE INTO documents
			 (document_id, collection, external_id, title, mime, source_uri, full_text, keywords, entities, labels, metadata, ingested_at)
			 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		);
		const deleteOldChunks = db.prepare("SELECT chunk_id FROM chunks WHERE collection = ? AND document_id = ?");
		const deleteVec = db.prepare(`DELETE FROM ${table} WHERE chunk_id = ?`);
		const deleteChunks = db.prepare("DELETE FROM chunks WHERE collection = ? AND document_id = ?");
		const insertChunk = db.prepare(
			`INSERT INTO chunks (chunk_id, collection, document_id, ordinal, external_id, content, chunk_metadata)
			 VALUES (?, ?, ?, ?, ?, ?, ?)`,
		);
		const insertVec = db.prepare(`INSERT INTO ${table} (chunk_id, embedding) VALUES (?, ?)`);

		const tx = db.transaction(() => {
			insertDoc.run(
				documentId,
				collection,
				doc.external_id ?? null,
				doc.title ?? null,
				doc.mime ?? null,
				doc.source_uri ?? null,
				doc.full_text,
				doc.keywords ? JSON.stringify(doc.keywords) : null,
				doc.entities !== undefined ? JSON.stringify(doc.entities) : null,
				doc.labels !== undefined ? JSON.stringify(doc.labels) : null,
				doc.metadata !== undefined ? JSON.stringify(doc.metadata) : null,
				Date.now(),
			);
			const oldChunkIds = deleteOldChunks.all(collection, documentId) as Array<{ chunk_id: string }>;
			for (const { chunk_id } of oldChunkIds) deleteVec.run(chunk_id);
			deleteChunks.run(collection, documentId);
			for (const chunk of chunks) {
				const chunkId = `${documentId}:${chunk.ordinal}`;
				insertChunk.run(
					chunkId,
					collection,
					documentId,
					chunk.ordinal,
					chunk.external_id ?? null,
					chunk.content,
					chunk.chunk_metadata !== undefined ? JSON.stringify(chunk.chunk_metadata) : null,
				);
				const embedding = new Float32Array(chunk.embedding);
				insertVec.run(chunkId, Buffer.from(embedding.buffer, embedding.byteOffset, embedding.byteLength));
			}
		});
		tx();
		return documentId;
	}

	async function deleteDocuments(collection: string, ids: string[]): Promise<number> {
		requireCollectionRow(collection);
		const table = vecTableName(collection);
		const resolveId = db.prepare(
			"SELECT document_id FROM documents WHERE collection = ? AND (document_id = ? OR external_id = ?)",
		);
		const deleteOne = db.transaction((documentId: string) => {
			const chunkIds = db
				.prepare("SELECT chunk_id FROM chunks WHERE collection = ? AND document_id = ?")
				.all(collection, documentId) as Array<{ chunk_id: string }>;
			const deleteVec = db.prepare(`DELETE FROM ${table} WHERE chunk_id = ?`);
			for (const { chunk_id } of chunkIds) deleteVec.run(chunk_id);
			db.prepare("DELETE FROM chunks WHERE collection = ? AND document_id = ?").run(collection, documentId);
			const info = db
				.prepare("DELETE FROM documents WHERE collection = ? AND document_id = ?")
				.run(collection, documentId);
			return info.changes;
		});

		let removed = 0;
		for (const id of ids) {
			const row = resolveId.get(collection, id, id) as { document_id: string } | undefined;
			if (!row) continue;
			removed += deleteOne(row.document_id);
		}
		return removed;
	}

	async function deleteByFilter(collection: string, filter: Filter): Promise<number> {
		requireCollectionRow(collection);
		const docRows = db.prepare("SELECT * FROM documents WHERE collection = ?").all(collection) as DocumentRow[];
		const toRemove: string[] = [];
		for (const docRow of docRows) {
			const record = toDocumentRecord(docRow);
			const chunkRows = db
				.prepare("SELECT * FROM chunks WHERE collection = ? AND document_id = ?")
				.all(collection, docRow.document_id) as ChunkRow[];
			const matches = chunkRows.some((c) =>
				evalFilter(filter, record, {
					content: c.content,
					ordinal: c.ordinal,
					externalId: c.external_id ?? undefined,
					chunkMetadata: c.chunk_metadata ? JSON.parse(c.chunk_metadata) : undefined,
				}),
			);
			if (matches) toRemove.push(docRow.document_id);
		}
		if (toRemove.length === 0) return 0;
		return deleteDocuments(collection, toRemove);
	}

	function vectorCandidates(collection: string, queryVector: number[], k: number): Map<string, number> {
		const table = vecTableName(collection);
		const queryBuf = Buffer.from(new Float32Array(queryVector).buffer);
		const rows = db
			.prepare(
				`SELECT chunk_id AS chunkId, distance FROM ${table} WHERE embedding MATCH ? AND k = ? ORDER BY distance`,
			)
			.all(queryBuf, k) as Array<{ chunkId: string; distance: number }>;
		return new Map(rows.map((r) => [r.chunkId, r.distance]));
	}

	function fullTextCandidates(collection: string, queryText: string, k: number): Map<string, number> {
		const rows = db
			.prepare(
				`SELECT f.chunk_id AS chunkId, bm25(chunks_fts) AS rank
				 FROM chunks_fts f WHERE chunks_fts MATCH ? AND f.collection = ? ORDER BY rank LIMIT ?`,
			)
			.all(queryText, collection, k) as Array<{ chunkId: string; rank: number }>;
		// bm25() is smaller-is-better; negate so larger is always more relevant.
		return new Map(rows.map((r) => [r.chunkId, -r.rank]));
	}

	function loadChunkContext(
		collection: string,
		chunkIds: string[],
	): Map<string, { row: ChunkRow; doc: DocumentRow }> {
		if (chunkIds.length === 0) return new Map();
		const placeholders = chunkIds.map(() => "?").join(",");
		const chunkRows = db
			.prepare(`SELECT * FROM chunks WHERE collection = ? AND chunk_id IN (${placeholders})`)
			.all(collection, ...chunkIds) as ChunkRow[];
		const docIds = Array.from(new Set(chunkRows.map((c) => c.document_id)));
		const docPlaceholders = docIds.map(() => "?").join(",");
		const docRows = docIds.length
			? (db
					.prepare(`SELECT * FROM documents WHERE collection = ? AND document_id IN (${docPlaceholders})`)
					.all(collection, ...docIds) as DocumentRow[])
			: [];
		const docsById = new Map(docRows.map((d) => [d.document_id, d]));
		const out = new Map<string, { row: ChunkRow; doc: DocumentRow }>();
		for (const row of chunkRows) {
			const doc = docsById.get(row.document_id);
			if (doc) out.set(row.chunk_id, { row, doc });
		}
		return out;
	}

	async function retrieve(collection: string, query: RetrieveQuery): Promise<RetrieveOutput> {
		const mode = query.mode ?? "vector";
		if (mode !== "vector" && mode !== "full_text" && mode !== "hybrid") {
			throw new Error(`retrieval mode unsupported by backend 'wasm-runtime-sqlite': ${mode}`);
		}
		const spec = requireCollectionRow(collection);
		const topK = query.top_k;
		if (!Number.isInteger(topK) || topK < 1 || topK > 200) {
			throw new Error("invalid query: top_k must be between 1 and 200");
		}
		const candidateK = topK * (query.candidate_multiplier ?? HYBRID_CANDIDATE_MULTIPLIER);

		let vectorScores: Map<string, number> = new Map();
		let ftsScores: Map<string, number> = new Map();

		if (mode === "vector" || mode === "hybrid") {
			if (!query.query_vector) throw new Error(`retrieve: query_vector is required for mode '${mode}'`);
			if (query.query_vector.length !== spec.embedding_dim) {
				throw new Error(`embedding dimension mismatch: expected ${spec.embedding_dim}, got ${query.query_vector.length}`);
			}
			vectorScores = vectorCandidates(collection, query.query_vector, mode === "hybrid" ? candidateK : topK);
		}
		if (mode === "full_text" || mode === "hybrid") {
			if (!query.query_text) throw new Error(`retrieve: query_text is required for mode '${mode}'`);
			ftsScores = fullTextCandidates(collection, query.query_text, mode === "hybrid" ? candidateK : topK);
		}

		const rrfK = 60;
		let fused: Array<{ chunkId: string; score: number; primaryScore: RetrievedChunk["primary_score"] }>;
		if (mode === "vector") {
			// vec0's MATCH returns distance (smaller = closer); negate so score
			// follows this store's larger-is-more-relevant convention. The vec0
			// index computes one physical distance function per table (fixed at
			// creation, not re-derived from `distance_metric` per query), so this
			// does not yet vary by the collection's configured `distance_metric`.
			fused = Array.from(vectorScores.entries()).map(([chunkId, distance]) => {
				const s = -distance;
				return { chunkId, score: s, primaryScore: { kind: "vector", score: s } };
			});
		} else if (mode === "full_text") {
			fused = Array.from(ftsScores.entries()).map(([chunkId, s]) => ({
				chunkId,
				score: s,
				primaryScore: { kind: "full_text", score: s },
			}));
		} else {
			const vecRanked = Array.from(vectorScores.entries()).sort((a, b) => a[1] - b[1]);
			const ftsRanked = Array.from(ftsScores.entries()).sort((a, b) => b[1] - a[1]);
			const rrfContribution = new Map<string, number>();
			vecRanked.forEach(([chunkId], rank) => {
				rrfContribution.set(chunkId, (rrfContribution.get(chunkId) ?? 0) + 1 / (rrfK + rank + 1));
			});
			ftsRanked.forEach(([chunkId], rank) => {
				rrfContribution.set(chunkId, (rrfContribution.get(chunkId) ?? 0) + 1 / (rrfK + rank + 1));
			});
			fused = Array.from(rrfContribution.entries()).map(([chunkId, rrf]) => ({
				chunkId,
				score: rrf,
				primaryScore: {
					kind: "hybrid",
					vector: -(vectorScores.get(chunkId) ?? 0),
					full_text: ftsScores.get(chunkId) ?? 0,
					rrf,
				},
			}));
		}

		fused.sort((a, b) => b.score - a.score);
		const context = loadChunkContext(collection, fused.map((f) => f.chunkId));

		let scored: RetrievedChunk[] = [];
		for (const f of fused) {
			const ctx = context.get(f.chunkId);
			if (!ctx) continue;
			const record = toDocumentRecord(ctx.doc);
			const chunkMetadata = ctx.row.chunk_metadata ? JSON.parse(ctx.row.chunk_metadata) : undefined;
			if (
				query.filter &&
				!evalFilter(query.filter, record, {
					content: ctx.row.content,
					ordinal: ctx.row.ordinal,
					externalId: ctx.row.external_id ?? undefined,
					chunkMetadata,
				})
			) {
				continue;
			}
			scored.push({
				id: f.chunkId,
				document_id: ctx.row.document_id,
				ordinal: ctx.row.ordinal,
				external_id: ctx.row.external_id ?? undefined,
				content: query.include_content ? ctx.row.content : undefined,
				score: f.score,
				primary_score: f.primaryScore,
				chunk_metadata: chunkMetadata,
				document: query.include_document ? toDocumentSummary(ctx.doc) : undefined,
			});
		}

		if (query.group_by_document) {
			const seen = new Set<string>();
			scored = scored.filter((c) => {
				if (seen.has(c.document_id)) return false;
				seen.add(c.document_id);
				return true;
			});
		}
		scored = scored.slice(0, topK);

		return { mode, chunks: scored, primary_latency_ms: 0 };
	}

	async function collectionStats(collection: string): Promise<CollectionStats> {
		requireCollectionRow(collection);
		const docCount = db.prepare("SELECT COUNT(*) AS n FROM documents WHERE collection = ?").get(collection) as {
			n: number;
		};
		const chunkCount = db.prepare("SELECT COUNT(*) AS n FROM chunks WHERE collection = ?").get(collection) as {
			n: number;
		};
		const lastIngested = db
			.prepare("SELECT MAX(ingested_at) AS t FROM documents WHERE collection = ?")
			.get(collection) as { t: number | null };
		return {
			documents: docCount.n,
			chunks: chunkCount.n,
			last_ingested_at: lastIngested.t ?? undefined,
		};
	}

	async function createEdge(edge: GraphEdge): Promise<void> {
		db.prepare(
			`INSERT OR REPLACE INTO graph_edges (id, source, target, label, properties) VALUES (?, ?, ?, ?, ?)`,
		).run(
			edge.id,
			edge.source,
			edge.target,
			edge.label ?? null,
			edge.properties ? JSON.stringify(edge.properties) : null,
		);
	}

	async function traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]> {
		if (startIds.length === 0) return [];
		if (!Number.isInteger(depth) || depth < 0) throw new Error("depth must be a non-negative integer");
		const labels = edgeLabels ?? [];
		const labelFilter = labels.length ? `AND e.label IN (${labels.map(() => "?").join(",")})` : "";
		const sql = `
      WITH RECURSIVE traversal(node_id, depth) AS (
        SELECT value, 0 FROM json_each(?)
        UNION
        SELECT e.target, traversal.depth + 1
        FROM traversal JOIN graph_edges e ON e.source = traversal.node_id
        WHERE traversal.depth < ? ${labelFilter}
      )
      SELECT DISTINCT node_id FROM traversal`;
		const params: unknown[] = [JSON.stringify(startIds), depth, ...labels];
		return (db.prepare(sql).all(...params) as Array<{ node_id: string }>).map((r) => r.node_id);
	}

	let closed = false;

	return {
		close: async () => {
			if (!closed) {
				db.close();
				closed = true;
			}
		},
		ensureCollection,
		dropCollection,
		getCollection,
		upsertDocument,
		deleteDocuments,
		deleteByFilter,
		retrieve,
		collectionStats,
		createEdge,
		traverseGraph,
	};
}
