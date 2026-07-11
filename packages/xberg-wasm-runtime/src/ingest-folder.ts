/**
 * Host-agnostic folder ingest orchestrator, shared between MCP and the
 * browser app so both drive the exact same extract -> PII+NER redact ->
 * chunk -> embed -> store sequence via the wasm engine. No filesystem
 * access happens in this file — callers supply file bytes already read
 * into memory, and are responsible for any file-output side effects
 * (writing redacted copies, reports, rehydration-map files) themselves.
 */

export interface ExtractInput {
	kind: "bytes";
	bytes: number[];
	filename?: string;
}

export interface ExtractedDocumentLike {
	content?: string;
	mimeType?: string;
}

export interface IngestDoc {
	full_text: string;
	title?: string;
	mime?: string;
	source_uri?: string;
}

export interface EngineIngestOutcome {
	document_id: string;
	rehydration_map: Record<string, string>;
	pii_category_counts: Record<string, number>;
}

export interface XbergEngineLike {
	extract(input: ExtractInput, config?: unknown): Promise<{ results?: ExtractedDocumentLike[] }>;
	ingest(doc: IngestDoc, collection: string, config?: unknown): Promise<EngineIngestOutcome>;
}

export interface FolderFileSource {
	name: string;
	path: string;
	bytes: Uint8Array;
}

export interface IngestFolderFileResult {
	filename: string;
	documentId: string | null;
	piiCategoryCounts: Record<string, number>;
	rehydrationMap: Record<string, string>;
	error?: string;
}

/**
 * Files are ingested sequentially rather than concurrently. The shared wasm
 * `XbergEngineLike` instance is not verified reentrant across concurrent
 * `extract`/`ingest` calls (its internal NER/embedding state is mutated
 * in-place per call), so concurrent calls risk cross-file state corruption.
 * Revisit if the engine gains a documented concurrency contract.
 */
export async function ingestFolder(
	engine: XbergEngineLike,
	collection: string,
	files: FolderFileSource[],
): Promise<IngestFolderFileResult[]> {
	const results: IngestFolderFileResult[] = [];

	for (const file of files) {
		try {
			const extracted = await engine.extract({
				kind: "bytes",
				bytes: Array.from(file.bytes),
				filename: file.name,
			});
			const doc = extracted.results?.[0];
			if (!doc) {
				results.push({
					filename: file.name,
					documentId: null,
					piiCategoryCounts: {},
					rehydrationMap: {},
					error: "extraction produced no document",
				});
				continue;
			}

			const outcome = await engine.ingest(
				{
					full_text: doc.content ?? "",
					title: file.name,
					mime: doc.mimeType,
					source_uri: file.path,
				},
				collection,
			);

			results.push({
				filename: file.name,
				documentId: outcome.document_id,
				piiCategoryCounts: outcome.pii_category_counts,
				rehydrationMap: outcome.rehydration_map,
			});
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err);
			results.push({
				filename: file.name,
				documentId: null,
				piiCategoryCounts: {},
				rehydrationMap: {},
				error: message,
			});
		}
	}

	return results;
}
