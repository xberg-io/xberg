import type { DocumentRecord, Filter } from "./types.js";

/** Minimal chunk shape `evalFilter` needs — enough to resolve `chunk.*` field paths. */
export interface FilterableChunk {
	content: string;
	ordinal: number;
	externalId?: string | undefined;
	chunkMetadata: unknown;
}

/**
 * Resolve a filter field to a value within a (document, chunk) context.
 * Mirrors `resolve_field` in `crates/xberg-rag/src/backends/memory.rs`.
 */
export function resolveField(fieldPath: string, doc: DocumentRecord, chunk: FilterableChunk): unknown {
	if (fieldPath.startsWith("doc.")) {
		const path = fieldPath.slice("doc.".length);
		switch (path) {
			case "full_text":
				return doc.full_text;
			case "title":
				return doc.title;
			case "mime":
				return doc.mime;
			case "external_id":
				return doc.external_id;
			case "source_uri":
				return doc.source_uri;
			case "keywords":
				return doc.keywords ?? [];
			case "labels":
				return doc.labels;
			case "entities":
				return doc.entities;
			default:
				if (path.startsWith("metadata.")) {
					return jsonPointer(doc.metadata, path.slice("metadata.".length));
				}
				return undefined;
		}
	} else if (fieldPath.startsWith("chunk.")) {
		const path = fieldPath.slice("chunk.".length);
		switch (path) {
			case "content":
				return chunk.content;
			case "ordinal":
				return chunk.ordinal;
			case "external_id":
				return chunk.externalId;
			default:
				if (path.startsWith("chunk_metadata.")) {
					return jsonPointer(chunk.chunkMetadata, path.slice("chunk_metadata.".length));
				}
				return undefined;
		}
	}
	return undefined;
}

function jsonPointer(value: unknown, dotted: string): unknown {
	let cur: unknown = value;
	for (const segment of dotted.split(".")) {
		if (cur === null || cur === undefined || typeof cur !== "object") return undefined;
		cur = (cur as Record<string, unknown>)[segment];
	}
	return cur;
}

function jsonEquals(a: unknown, b: unknown): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

function jsonCmp(a: unknown, b: unknown): number | undefined {
	if (typeof a === "number" && typeof b === "number") {
		return a - b;
	}
	if (typeof a === "string" && typeof b === "string") {
		return a < b ? -1 : a > b ? 1 : 0;
	}
	return undefined;
}

/** Mirrors `eval_filter` in `crates/xberg-rag/src/backends/memory.rs`. */
export function evalFilter(filter: Filter, doc: DocumentRecord, chunk: FilterableChunk): boolean {
	if ("eq" in filter) {
		const v = resolveField(filter.eq.field, doc, chunk);
		return v !== undefined && jsonEquals(v, filter.eq.value);
	}
	if ("in" in filter) {
		const v = resolveField(filter.in.field, doc, chunk);
		return v !== undefined && filter.in.values.some((candidate) => jsonEquals(candidate, v));
	}
	if ("array_contains" in filter) {
		const v = resolveField(filter.array_contains.field, doc, chunk);
		return Array.isArray(v) && v.some((item) => jsonEquals(item, filter.array_contains.value));
	}
	if ("range" in filter) {
		const { field, gte, gt, lte, lt } = filter.range;
		const v = resolveField(field, doc, chunk);
		if (v === undefined) return false;
		const pass = (bound: unknown, cmp: (ord: number) => boolean): boolean => {
			if (bound === undefined) return true;
			const ord = jsonCmp(v, bound);
			return ord !== undefined && cmp(ord);
		};
		return pass(gte, (o) => o >= 0) && pass(gt, (o) => o > 0) && pass(lte, (o) => o <= 0) && pass(lt, (o) => o < 0);
	}
	if ("text_match" in filter) {
		const v = resolveField(filter.text_match.field, doc, chunk);
		return typeof v === "string" && v.toLowerCase().includes(filter.text_match.query.toLowerCase());
	}
	if ("and" in filter) {
		return filter.and.filters.every((f) => evalFilter(f, doc, chunk));
	}
	if ("or" in filter) {
		return filter.or.filters.some((f) => evalFilter(f, doc, chunk));
	}
	if ("not" in filter) {
		return !evalFilter(filter.not.filter, doc, chunk);
	}
	return false;
}
