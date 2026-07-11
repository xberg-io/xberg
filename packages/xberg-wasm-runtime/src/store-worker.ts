import { SCHEMA_SQL, SCHEMA_VERSION, createVecTableSql, sanitizeTableName, vecTableName } from "./store-schema.js";
import { evalFilter } from "./filter-eval.js";
import type {
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
} from "./types.js";

export type StoreWorkerRequest =
	| { op: "init"; dbPath: string; id: number }
	| { op: "close"; id: number }
	| { op: "ensureCollection"; spec: CollectionSpec; id: number }
	| { op: "dropCollection"; collection: string; id: number }
	| { op: "getCollection"; collection: string; id: number }
	| { op: "upsertDocument"; collection: string; doc: DocumentRecord; chunks: ChunkRecord[]; id: number }
	| { op: "deleteDocuments"; collection: string; ids: string[]; id: number }
	| { op: "deleteByFilter"; collection: string; filter: Filter; id: number }
	| { op: "retrieve"; collection: string; query: RetrieveQuery; id: number }
	| { op: "collectionStats"; collection: string; id: number }
	| { op: "createEdge"; edge: GraphEdge; id: number }
	| { op: "traverseGraph"; startIds: string[]; depth: number; edgeLabels?: string[]; id: number };

export interface StoreWorkerResponse {
	id: number;
	ok: boolean;
	result?: unknown;
	error?: string;
}

export type StoreWorkerRequestBase = DistributiveOmit<StoreWorkerRequest, "id">;
type DistributiveOmit<T, K extends PropertyKey> = T extends unknown ? Omit<T, K> : never;

interface SqliteDb {
	exec(options: string | { sql: string; bind?: unknown[]; rowMode?: "object"; returnValue?: "resultRows" }): unknown;
	selectValue(sql: string, bind?: unknown[]): unknown;
	close(): void;
}

interface SqliteModule {
	oo1: {
		DB: new (filename: string, flags?: string) => SqliteDb;
		OpfsDb: new (filename: string) => SqliteDb;
	};
}

let database: SqliteDb | undefined;

const HYBRID_CANDIDATE_MULTIPLIER = 4;
const RRF_K = 60;

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

async function ensureOpfsDirectory(dbPath: string): Promise<void> {
	const directoryNames = dbPath.split("/").filter(Boolean).slice(0, -1);
	let directory = await navigator.storage.getDirectory();
	for (const name of directoryNames) {
		// oxlint-disable-next-line no-await-in-loop -- each handle is relative to its parent
		directory = await directory.getDirectoryHandle(name, { create: true });
	}
}

function requireDatabase(): SqliteDb {
	if (!database) throw new Error("store is not initialized");
	return database;
}

function rows<T>(db: SqliteDb, sql: string, bind: unknown[] = []): T[] {
	return db.exec({ sql, bind, rowMode: "object", returnValue: "resultRows" }) as T[];
}

function transaction(db: SqliteDb, operation: () => void): void {
	db.exec("BEGIN IMMEDIATE");
	try {
		operation();
		db.exec("COMMIT");
	} catch (error) {
		db.exec("ROLLBACK");
		throw error;
	}
}

async function initialize(dbPath: string): Promise<void> {
	if (!globalThis.crossOriginIsolated) {
		throw new Error("OPFS SQLite requires cross-origin isolation (COOP/COEP headers)");
	}
	const module = (await import("../wasm/sqlite-vec/sqlite3.mjs")) as {
		default: (options?: { locateFile?: (filename: string) => string }) => Promise<SqliteModule>;
	};
	let sqlite3: SqliteModule;
	try {
		sqlite3 = await module.default({
			locateFile: (filename) => new URL(`../wasm/sqlite-vec/${filename}`, import.meta.url).href,
		});
	} catch (error) {
		throw new Error(`loading sqlite-vec WASM: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	if (!sqlite3.oo1.OpfsDb) throw new Error("OPFS is unavailable in this browser context");
	try {
		await ensureOpfsDirectory(dbPath);
		database = new sqlite3.oo1.OpfsDb(dbPath);
	} catch (error) {
		throw new Error(`opening OPFS database ${dbPath}: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	try {
		// Versioned migration: only (re)apply the schema when the persisted
		// database is on an older layout, then record the current version.
		// `SCHEMA_SQL` uses `IF NOT EXISTS`, so re-applying is idempotent and
		// never drops existing data.
		const version = Number(database.selectValue("PRAGMA user_version") ?? 0);
		if (version < SCHEMA_VERSION) {
			database.exec(SCHEMA_SQL);
			database.exec(`PRAGMA user_version = ${SCHEMA_VERSION}`);
		}
	} catch (error) {
		database.close();
		database = undefined;
		throw new Error(`creating store schema: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	let version: unknown;
	try {
		version = database.selectValue("SELECT vec_version()");
	} catch (error) {
		database.close();
		database = undefined;
		throw new Error(`checking sqlite-vec: ${error instanceof Error ? error.message : String(error)}`, {
			cause: error,
		});
	}
	if (typeof version !== "string") {
		database.close();
		database = undefined;
		throw new Error("sqlite-vec failed to initialize");
	}
}

function closeDatabase(): void {
	database?.close();
	database = undefined;
}

function getCollectionRow(collection: string): CollectionRow | undefined {
	const result = rows<CollectionRow>(
		requireDatabase(),
		"SELECT embedding_dim, distance_metric, index_method FROM collections WHERE name = ?",
		[collection],
	);
	return result[0];
}

function requireCollectionRow(collection: string): CollectionRow {
	const row = getCollectionRow(collection);
	if (!row) throw new Error(`collection not found: ${collection}`);
	return row;
}

function ensureCollection(spec: CollectionSpec): string | void {
	if (!spec.name.trim()) return "collection name must not be empty";
	const db = requireDatabase();
	const existing = getCollectionRow(spec.name);
	if (existing) {
		if (existing.embedding_dim !== spec.embedding_dim) {
			return `collection already exists: ${spec.name}`;
		}
		return undefined;
	}
	db.exec({
		sql: "INSERT INTO collections (name, sanitized_name, embedding_dim, distance_metric, index_method) VALUES (?, ?, ?, ?, ?)",
		bind: [
			spec.name,
			sanitizeTableName(spec.name),
			spec.embedding_dim,
			spec.distance_metric ?? "cosine",
			spec.index_method ?? "flat",
		],
	});
	db.exec(createVecTableSql(spec.name, spec.embedding_dim));
	return undefined;
}

function dropCollection(collection: string): string | void {
	if (!getCollectionRow(collection)) return `collection not found: ${collection}`;
	const db = requireDatabase();
	transaction(db, () => {
		db.exec(`DROP TABLE IF EXISTS ${vecTableName(collection)}`);
		db.exec({ sql: "DELETE FROM chunks WHERE collection = ?", bind: [collection] });
		db.exec({ sql: "DELETE FROM documents WHERE collection = ?", bind: [collection] });
		db.exec({ sql: "DELETE FROM collections WHERE name = ?", bind: [collection] });
	});
	return undefined;
}

function getCollection(collection: string): CollectionSpec | null {
	const row = getCollectionRow(collection);
	if (!row) return null;
	return {
		name: collection,
		embedding_dim: row.embedding_dim,
		distance_metric: row.distance_metric,
		index_method: row.index_method,
	};
}

function upsertDocument(collection: string, doc: DocumentRecord, chunks: ChunkRecord[]): string {
	const spec = requireCollectionRow(collection);
	for (const chunk of chunks) {
		if (chunk.embedding.length !== spec.embedding_dim) {
			throw new Error(`embedding dimension mismatch: expected ${spec.embedding_dim}, got ${chunk.embedding.length}`);
		}
	}
	const db = requireDatabase();
	const table = vecTableName(collection);
	const existingRows = doc.external_id
		? rows<{ document_id: string }>(
				db,
				"SELECT document_id FROM documents WHERE collection = ? AND external_id = ?",
				[collection, doc.external_id],
			)
		: [];
	const documentId = existingRows[0]?.document_id ?? crypto.randomUUID();

	transaction(db, () => {
		db.exec({
			sql: `INSERT OR REPLACE INTO documents
				(document_id, collection, external_id, title, mime, source_uri, full_text, keywords, entities, labels, metadata, ingested_at)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
			bind: [
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
			],
		});
		const oldChunkIds = rows<{ chunk_id: string }>(
			db,
			"SELECT chunk_id FROM chunks WHERE collection = ? AND document_id = ?",
			[collection, documentId],
		);
		for (const { chunk_id } of oldChunkIds) {
			db.exec({ sql: `DELETE FROM ${table} WHERE chunk_id = ?`, bind: [chunk_id] });
		}
		db.exec({ sql: "DELETE FROM chunks WHERE collection = ? AND document_id = ?", bind: [collection, documentId] });
		for (const chunk of chunks) {
			const chunkId = `${documentId}:${chunk.ordinal}`;
			db.exec({
				sql: `INSERT INTO chunks (chunk_id, collection, document_id, ordinal, external_id, content, chunk_metadata)
					VALUES (?, ?, ?, ?, ?, ?, ?)`,
				bind: [
					chunkId,
					collection,
					documentId,
					chunk.ordinal,
					chunk.external_id ?? null,
					chunk.content,
					chunk.chunk_metadata !== undefined ? JSON.stringify(chunk.chunk_metadata) : null,
				],
			});
			const embedding = new Float32Array(chunk.embedding);
			db.exec({
				sql: `INSERT INTO ${table} (chunk_id, embedding) VALUES (?, ?)`,
				bind: [chunkId, new Uint8Array(embedding.buffer, embedding.byteOffset, embedding.byteLength)],
			});
		}
	});
	return documentId;
}

function deleteDocumentById(collection: string, documentId: string): number {
	const db = requireDatabase();
	const table = vecTableName(collection);
	let changed = 0;
	transaction(db, () => {
		const chunkIds = rows<{ chunk_id: string }>(
			db,
			"SELECT chunk_id FROM chunks WHERE collection = ? AND document_id = ?",
			[collection, documentId],
		);
		for (const { chunk_id } of chunkIds) db.exec({ sql: `DELETE FROM ${table} WHERE chunk_id = ?`, bind: [chunk_id] });
		db.exec({ sql: "DELETE FROM chunks WHERE collection = ? AND document_id = ?", bind: [collection, documentId] });
		const before = db.selectValue("SELECT COUNT(*) FROM documents WHERE collection = ? AND document_id = ?", [
			collection,
			documentId,
		]);
		db.exec({
			sql: "DELETE FROM documents WHERE collection = ? AND document_id = ?",
			bind: [collection, documentId],
		});
		changed = typeof before === "number" ? before : 0;
	});
	return changed;
}

function deleteDocuments(collection: string, ids: string[]): number {
	requireCollectionRow(collection);
	const db = requireDatabase();
	let removed = 0;
	for (const id of ids) {
		const resolved = rows<{ document_id: string }>(
			db,
			"SELECT document_id FROM documents WHERE collection = ? AND (document_id = ? OR external_id = ?)",
			[collection, id, id],
		)[0];
		if (!resolved) continue;
		removed += deleteDocumentById(collection, resolved.document_id);
	}
	return removed;
}

function deleteByFilter(collection: string, filter: Filter): number {
	requireCollectionRow(collection);
	const db = requireDatabase();
	const docRows = rows<DocumentRow>(db, "SELECT * FROM documents WHERE collection = ?", [collection]);
	const toRemove: string[] = [];
	for (const docRow of docRows) {
		const record = toDocumentRecord(docRow);
		const chunkRows = rows<ChunkRow>(db, "SELECT * FROM chunks WHERE collection = ? AND document_id = ?", [
			collection,
			docRow.document_id,
		]);
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
	return deleteDocuments(collection, toRemove);
}

function vectorCandidates(collection: string, queryVector: number[], k: number): Map<string, number> {
	const table = vecTableName(collection);
	const result = rows<{ chunkId: string; distance: number }>(
		requireDatabase(),
		`SELECT chunk_id AS chunkId, distance FROM ${table} WHERE embedding MATCH ? AND k = ? ORDER BY distance`,
		[new Uint8Array(new Float32Array(queryVector).buffer), k],
	);
	return new Map(result.map((r) => [r.chunkId, r.distance]));
}

function fullTextCandidates(collection: string, queryText: string, k: number): Map<string, number> {
	const result = rows<{ chunkId: string; rank: number }>(
		requireDatabase(),
		`SELECT f.chunk_id AS chunkId, bm25(chunks_fts) AS rank
		 FROM chunks_fts f WHERE chunks_fts MATCH ? AND f.collection = ? ORDER BY rank LIMIT ?`,
		[queryText, collection, k],
	);
	return new Map(result.map((r) => [r.chunkId, -r.rank]));
}

function loadChunkContext(collection: string, chunkIds: string[]): Map<string, { row: ChunkRow; doc: DocumentRow }> {
	if (chunkIds.length === 0) return new Map();
	const db = requireDatabase();
	const placeholders = chunkIds.map(() => "?").join(",");
	const chunkRows = rows<ChunkRow>(db, `SELECT * FROM chunks WHERE collection = ? AND chunk_id IN (${placeholders})`, [
		collection,
		...chunkIds,
	]);
	const docIds = Array.from(new Set(chunkRows.map((c) => c.document_id)));
	const docPlaceholders = docIds.map(() => "?").join(",");
	const docRows = docIds.length
		? rows<DocumentRow>(db, `SELECT * FROM documents WHERE collection = ? AND document_id IN (${docPlaceholders})`, [
				collection,
				...docIds,
			])
		: [];
	const docsById = new Map(docRows.map((d) => [d.document_id, d]));
	const out = new Map<string, { row: ChunkRow; doc: DocumentRow }>();
	for (const row of chunkRows) {
		const doc = docsById.get(row.document_id);
		if (doc) out.set(row.chunk_id, { row, doc });
	}
	return out;
}

function retrieve(collection: string, query: RetrieveQuery): RetrieveOutput {
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

	let fused: Array<{ chunkId: string; score: number; primaryScore: RetrievedChunk["primary_score"] }>;
	if (mode === "vector") {
		fused = Array.from(vectorScores.entries()).map(([chunkId, distance]) => {
			const s = -distance;
			return { chunkId, score: s, primaryScore: { kind: "vector" as const, score: s } };
		});
	} else if (mode === "full_text") {
		fused = Array.from(ftsScores.entries()).map(([chunkId, s]) => ({
			chunkId,
			score: s,
			primaryScore: { kind: "full_text" as const, score: s },
		}));
	} else {
		const vecRanked = Array.from(vectorScores.entries()).sort((a, b) => a[1] - b[1]);
		const ftsRanked = Array.from(ftsScores.entries()).sort((a, b) => b[1] - a[1]);
		const rrfContribution = new Map<string, number>();
		vecRanked.forEach(([chunkId], rank) => {
			rrfContribution.set(chunkId, (rrfContribution.get(chunkId) ?? 0) + 1 / (RRF_K + rank + 1));
		});
		ftsRanked.forEach(([chunkId], rank) => {
			rrfContribution.set(chunkId, (rrfContribution.get(chunkId) ?? 0) + 1 / (RRF_K + rank + 1));
		});
		fused = Array.from(rrfContribution.entries()).map(([chunkId, rrf]) => ({
			chunkId,
			score: rrf,
			primaryScore: {
				kind: "hybrid" as const,
				vector: -(vectorScores.get(chunkId) ?? 0),
				full_text: ftsScores.get(chunkId) ?? 0,
				rrf,
			},
		}));
	}

	fused.sort((a, b) => b.score - a.score);
	const context = loadChunkContext(
		collection,
		fused.map((f) => f.chunkId),
	);

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

function collectionStats(collection: string): CollectionStats {
	requireCollectionRow(collection);
	const db = requireDatabase();
	const documentsCount = db.selectValue("SELECT COUNT(*) FROM documents WHERE collection = ?", [collection]);
	const chunksCount = db.selectValue("SELECT COUNT(*) FROM chunks WHERE collection = ?", [collection]);
	const lastIngested = db.selectValue("SELECT MAX(ingested_at) FROM documents WHERE collection = ?", [collection]);
	return {
		documents: typeof documentsCount === "number" ? documentsCount : 0,
		chunks: typeof chunksCount === "number" ? chunksCount : 0,
		last_ingested_at: typeof lastIngested === "number" ? lastIngested : undefined,
	};
}

function createEdge(edge: GraphEdge): void {
	requireDatabase().exec({
		sql: "INSERT OR REPLACE INTO graph_edges (id, source, target, label, properties) VALUES (?, ?, ?, ?, ?)",
		bind: [
			edge.id,
			edge.source,
			edge.target,
			edge.label ?? null,
			edge.properties ? JSON.stringify(edge.properties) : null,
		],
	});
}

function traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): string[] {
	if (startIds.length === 0) return [];
	if (!Number.isInteger(depth) || depth < 0) throw new Error("depth must be a non-negative integer");
	const labels = edgeLabels ?? [];
	const labelFilter = labels.length ? `AND e.label IN (${labels.map(() => "?").join(",")})` : "";
	const result = rows<{ node_id: string }>(
		requireDatabase(),
		`
    WITH RECURSIVE traversal(node_id, depth) AS (
      SELECT value, 0 FROM json_each(?)
      UNION
      SELECT e.target, traversal.depth + 1 FROM traversal
      JOIN graph_edges e ON e.source = traversal.node_id
      WHERE traversal.depth < ? ${labelFilter}
    ) SELECT DISTINCT node_id FROM traversal`,
		[JSON.stringify(startIds), depth, ...labels],
	);
	return result.map(({ node_id }) => node_id);
}

async function dispatch(request: StoreWorkerRequest): Promise<unknown> {
	switch (request.op) {
		case "init":
			return initialize(request.dbPath);
		case "close":
			return closeDatabase();
		case "ensureCollection":
			return ensureCollection(request.spec);
		case "dropCollection":
			return dropCollection(request.collection);
		case "getCollection":
			return getCollection(request.collection);
		case "upsertDocument":
			return upsertDocument(request.collection, request.doc, request.chunks);
		case "deleteDocuments":
			return deleteDocuments(request.collection, request.ids);
		case "deleteByFilter":
			return deleteByFilter(request.collection, request.filter);
		case "retrieve":
			return retrieve(request.collection, request.query);
		case "collectionStats":
			return collectionStats(request.collection);
		case "createEdge":
			return createEdge(request.edge);
		case "traverseGraph":
			return traverseGraph(request.startIds, request.depth, request.edgeLabels);
	}
}

globalThis.onmessage = async (event: MessageEvent<StoreWorkerRequest>) => {
	const { id } = event.data;
	try {
		const result = await dispatch(event.data);
		globalThis.postMessage({ id, ok: true, result } satisfies StoreWorkerResponse);
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		globalThis.postMessage({ id, ok: false, error: message } satisfies StoreWorkerResponse);
	}
};
