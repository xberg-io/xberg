import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createEmbedder } from "./embedder.js";
import { createNer } from "./ner.js";
import { createOcr } from "./ocr.js";
import { createNodeVectorStore, type NodeVectorStore } from "./store-node.js";
import { detectPiiWithNer, groupByCategory } from "./pii.js";
import type { EmbedderInterface, NerInterface, OcrInterface } from "./types.js";

/**
 * End-to-end integration test against REAL models — no mocks, no stubs.
 * Every other test file in this package exercises one capability at a time
 * (embedder.test.ts, ner.test.ts, ocr.test.ts, store-node.test.ts); this
 * file wires them together the way a real caller (ingest-folder.ts, an MCP
 * tool) would: OCR/text -> NER-aware PII detection -> real embedding ->
 * persistent SQLite storage -> vector/full_text/hybrid retrieval -> graph
 * edges linking related documents.
 *
 * Models load from the default transformers.js/ppu cache (not a fresh
 * tmpdir), so this is fast once `embedder.test.ts`/`ner.test.ts`/
 * `ocr.test.ts` have populated it in the same environment; on a fully cold
 * cache the beforeAll hook downloads ~600MB+ across three models.
 */
describe("full pipeline (real embedder + NER + OCR + SQLite store)", () => {
	let embedder: EmbedderInterface;
	let ner: NerInterface | null;
	let ocr: OcrInterface | null;
	let store: NodeVectorStore;
	let storeDir: string;
	const collection = "full-pipeline-docs";
	const EMBEDDING_DIM = 1024; // Xenova/bge-m3

	beforeAll(async () => {
		[embedder, ner, ocr] = await Promise.all([
			createEmbedder({ models: { embedder: "Xenova/bge-m3" } }),
			createNer({ models: { ner: "Xenova/bert-base-NER" } }),
			createOcr(),
		]);
		storeDir = mkdtempSync(join(tmpdir(), "xberg-full-pipeline-"));
		store = await createNodeVectorStore({ nodeStorePath: join(storeDir, "store.sqlite3") });
		await store.ensureCollection({ name: collection, embedding_dim: EMBEDDING_DIM });
	}, 300_000);

	afterAll(async () => {
		await store.close();
		rmSync(storeDir, { recursive: true, force: true });
	});

	it("detects PII via real NER + regex, embeds, stores, and retrieves the document by vector similarity", async () => {
		const text =
			"Alice Johnson works at Acme Corp in Seattle. Contact her at alice@acme.com or 555-867-5309.";

		if (!ner) throw new Error("NER did not load in this environment — cannot run the full pipeline test");
		const entities = await ner.ner(text);
		const findings = detectPiiWithNer(text, entities);
		expect(findings.length).toBeGreaterThan(0);
		const byCategory = groupByCategory(findings);
		expect(byCategory["EMAIL"]).toBeGreaterThanOrEqual(1);
		expect(byCategory["PHONE"]).toBeGreaterThanOrEqual(1);
		// NER (not regex) is what can find "Alice Johnson"/"Acme Corp"/"Seattle" —
		// regex alone only catches EMAIL/PHONE, so this asserts NER actually
		// contributed findings, not just the deterministic regex layer.
		expect(entities.length).toBeGreaterThan(0);

		const [vector] = await embedder.embed([text]);
		expect(vector).toBeInstanceOf(Float32Array);
		if (!vector) throw new Error("expected an embedding vector");
		expect(vector.length).toBe(EMBEDDING_DIM);

		const documentId = await store.upsertDocument(collection, { full_text: text, title: "employee-record" }, [
			{ ordinal: 0, content: text, embedding: Array.from(vector) },
		]);
		expect(typeof documentId).toBe("string");

		const out = await store.retrieve(collection, {
			top_k: 5,
			mode: "vector",
			query_vector: Array.from(vector),
			include_content: true,
		});
		expect(out.chunks[0]?.document_id).toBe(documentId);
		expect(out.chunks[0]?.content).toBe(text);
		expect(out.chunks[0]?.primary_score.kind).toBe("vector");
	}, 120_000);

	it("extracts text from a real rendered image via OCR, then embeds/stores/retrieves it", async (testCtx) => {
		if (!ocr) {
			testCtx.skip();
			return;
		}

		// Same real (non-mocked) text-rendering approach as ocr.test.ts: render
		// through ppu-ocv's bundled @napi-rs/canvas rather than a static fixture,
		// so detection + recognition both run against real model inference.
		const { createRequire } = await import("module");
		const require = createRequire(import.meta.url);
		const ppuOcvEntry = require.resolve("ppu-ocv");
		const nestedRequire = createRequire(ppuOcvEntry);
		const { createCanvas } = nestedRequire("@napi-rs/canvas") as {
			createCanvas: (
				w: number,
				h: number,
			) => {
				getContext: (kind: "2d") => {
					fillStyle: string;
					fillRect: (x: number, y: number, w: number, h: number) => void;
					font: string;
					fillText: (text: string, x: number, y: number) => void;
				};
				toBuffer: (mime: string) => Buffer;
			};
		};
		const canvas = createCanvas(640, 120);
		const ctx = canvas.getContext("2d");
		ctx.fillStyle = "white";
		ctx.fillRect(0, 0, 640, 120);
		ctx.fillStyle = "black";
		ctx.font = "64px Arial";
		ctx.fillText("INVOICE", 10, 86);
		const pngBuffer = canvas.toBuffer("image/png");

		const ocrResult = await ocr.ocr(new Uint8Array(pngBuffer));
		expect(ocrResult.text.toUpperCase()).toContain("INVOICE");

		const [vector] = await embedder.embed([ocrResult.text]);
		if (!vector) throw new Error("expected an embedding vector");

		const documentId = await store.upsertDocument(
			collection,
			{ full_text: ocrResult.text, title: "scanned-invoice", mime: "image/png" },
			[{ ordinal: 0, content: ocrResult.text, embedding: Array.from(vector) }],
		);

		const out = await store.retrieve(collection, {
			top_k: 5,
			mode: "full_text",
			query_text: "invoice",
			include_content: true,
		});
		expect(out.chunks.some((c) => c.document_id === documentId)).toBe(true);
		expect(out.mode).toBe("full_text");
	}, 120_000);

	it("hybrid retrieve ranks a real document strong on both vector and full-text signals highest", async () => {
		const target = "hybrid search combines dense vectors with keyword ranking";
		const vectorDistractor = "completely unrelated filler content about gardening";
		const textDistractor = "hybrid search hybrid search hybrid search hybrid search";

		const [targetVec, vectorDistractorVec, textDistractorVec] = await embedder.embed([
			target,
			vectorDistractor,
			textDistractor,
		]);
		if (!targetVec || !vectorDistractorVec || !textDistractorVec) {
			throw new Error("expected three embedding vectors");
		}

		const targetId = await store.upsertDocument(collection, { full_text: target }, [
			{ ordinal: 0, content: target, embedding: Array.from(targetVec) },
		]);
		await store.upsertDocument(collection, { full_text: vectorDistractor }, [
			{ ordinal: 0, content: vectorDistractor, embedding: Array.from(vectorDistractorVec) },
		]);
		await store.upsertDocument(collection, { full_text: textDistractor }, [
			{ ordinal: 0, content: textDistractor, embedding: Array.from(textDistractorVec) },
		]);

		const out = await store.retrieve(collection, {
			top_k: 3,
			mode: "hybrid",
			query_vector: Array.from(targetVec),
			query_text: "hybrid search",
			include_content: true,
		});
		expect(out.chunks[0]?.document_id).toBe(targetId);
		expect(out.chunks[0]?.primary_score.kind).toBe("hybrid");
	}, 120_000);

	it("mirrors document relationships via real graph edges and traverses them", async () => {
		const [a, b, c] = await embedder.embed(["doc a", "doc b", "doc c"]);
		if (!a || !b || !c) throw new Error("expected three embedding vectors");

		const docA = await store.upsertDocument(collection, { full_text: "doc a", title: "a" }, [
			{ ordinal: 0, content: "doc a", embedding: Array.from(a) },
		]);
		const docB = await store.upsertDocument(collection, { full_text: "doc b", title: "b" }, [
			{ ordinal: 0, content: "doc b", embedding: Array.from(b) },
		]);
		const docC = await store.upsertDocument(collection, { full_text: "doc c", title: "c" }, [
			{ ordinal: 0, content: "doc c", embedding: Array.from(c) },
		]);

		await store.createEdge({ id: `${docA}->${docB}`, source: docA, target: docB, label: "references" });
		await store.createEdge({ id: `${docB}->${docC}`, source: docB, target: docC, label: "references" });

		const reached = await store.traverseGraph([docA], 2, ["references"]);
		expect(reached).toContain(docA);
		expect(reached).toContain(docB);
		expect(reached).toContain(docC);
	}, 120_000);
});
