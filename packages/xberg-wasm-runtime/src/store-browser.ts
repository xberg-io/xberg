import type {
	VectorStoreInterface,
	CollectionSpec,
	CollectionStats,
	DocumentRecord,
	ChunkRecord,
	Filter,
	GraphEdge,
	RetrieveQuery,
	RetrieveOutput,
	CacheConfig,
} from "./types.js";
import type { StoreWorkerResponse, StoreWorkerRequestBase } from "./store-worker.js";

/** See `NodeVectorStore` in store-node.ts — same rationale applies here. */
export type BrowserVectorStore = VectorStoreInterface & {
	close(): Promise<void>;
	createEdge(edge: GraphEdge): Promise<void>;
	traverseGraph(startIds: string[], depth: number, edgeLabels?: string[]): Promise<string[]>;
};

export async function createBrowserVectorStore(config?: CacheConfig): Promise<BrowserVectorStore> {
	const worker = new Worker(new URL("./store-worker.js", import.meta.url), { type: "module" });
	const dbPath = config?.opfsPath ?? "/xberg/default.sqlite3";
	if (!dbPath.startsWith("/") || dbPath.includes("..")) {
		worker.terminate();
		throw new Error("[store-browser] opfsPath must be an absolute OPFS path without '..'");
	}

	let nextId = 1;
	let closed = false;
	const pending = new Map<
		number,
		{
			resolve: (r: StoreWorkerResponse) => void;
			reject: (error: Error) => void;
			timeout: ReturnType<typeof setTimeout>;
		}
	>();

	worker.onmessage = (event: MessageEvent<StoreWorkerResponse>) => {
		const entry = pending.get(event.data.id);
		if (entry) {
			pending.delete(event.data.id);
			clearTimeout(entry.timeout);
			entry.resolve(event.data);
		}
	};

	worker.onerror = (event) => {
		const error = new Error(`[store-browser] worker failed: ${event.message}`);
		for (const entry of pending.values()) {
			clearTimeout(entry.timeout);
			entry.reject(error);
		}
		pending.clear();
	};

	async function call<T>(req: StoreWorkerRequestBase): Promise<T> {
		const id = nextId++;
		const response = await new Promise<StoreWorkerResponse>((resolve, reject) => {
			const timeout = setTimeout(() => {
				pending.delete(id);
				reject(new Error(`[store-browser] ${req.op} timed out after 15 seconds`));
			}, 15_000);
			pending.set(id, { resolve, reject, timeout });
			worker.postMessage({ ...req, id });
		});
		if (!response.ok) {
			throw new Error(`[store-browser] ${req.op} failed: ${response.error}`);
		}
		return response.result as T;
	}

	try {
		await call<void>({ op: "init", dbPath });
	} catch (error) {
		worker.terminate();
		throw error;
	}

	return {
		close: async () => {
			if (closed) return;
			try {
				await call<void>({ op: "close" });
			} finally {
				closed = true;
				worker.terminate();
			}
		},
		ensureCollection: (spec: CollectionSpec) => call<string | void>({ op: "ensureCollection", spec }),
		dropCollection: (collection: string) => call<string | void>({ op: "dropCollection", collection }),
		getCollection: (collection: string) => call<CollectionSpec | null>({ op: "getCollection", collection }),
		upsertDocument: (collection: string, doc: DocumentRecord, chunks: ChunkRecord[]) =>
			call<string>({ op: "upsertDocument", collection, doc, chunks }),
		deleteDocuments: (collection: string, ids: string[]) => call<number>({ op: "deleteDocuments", collection, ids }),
		deleteByFilter: (collection: string, filter: Filter) =>
			call<number>({ op: "deleteByFilter", collection, filter }),
		retrieve: (collection: string, query: RetrieveQuery) =>
			call<RetrieveOutput>({ op: "retrieve", collection, query }),
		collectionStats: (collection: string) => call<CollectionStats>({ op: "collectionStats", collection }),
		createEdge: (edge: GraphEdge) => call<void>({ op: "createEdge", edge }),
		traverseGraph: (startIds: string[], depth: number, edgeLabels?: string[]) =>
			call<string[]>({ op: "traverseGraph", startIds, depth, edgeLabels }),
	};
}
