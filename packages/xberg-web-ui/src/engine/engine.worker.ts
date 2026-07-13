// src/engine/engine.worker.ts
/// <reference lib="webworker" />
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord, CollectionSpec } from "xberg-wasm-runtime";
import { XbergEngine } from "@xberg-io/xberg-wasm";
import { postIngest, postMap } from "../lib/sync-client.js";
import { sanitizeExternalId } from "../lib/sanitize-id.js";
import { EMBEDDING_DIM } from "../lib/constants.js";
import type { IngestHistoryEntry } from "../lib/types.js";

declare const self: DedicatedWorkerGlobalScope;

interface IngestMessage {
  type: "ingest";
  requestId: string;
  file: File;
  filename: string;
  mime: string;
  collection: string;
  passphrase: string;
  mcpBaseUrl: string;
}

interface OcrMessage {
  type: "ocr";
  requestId: string;
  bytes: Uint8Array;
}

let mcpBaseUrl = "";
let engine: XbergEngine | null = null;
// Captures the redacted `full_text` seen by the most recent `upsertDocument`
// call (see `createHttpStore` below), so `handleIngest` can persist the
// redacted text instead of the raw pre-redaction extraction output. Safe
// only because ingestion is processed sequentially in this worker (one
// `handleIngest` in flight at a time) — same assumption as the `engine`
// singleton above.
let lastRedactedFullText = "";
// Queue to enforce sequential processing of ingests
let lastIngest: Promise<void> = Promise.resolve();

/**
 * HTTP-backed `VectorStoreInterface`. Only `upsertDocument` matters for
 * `engine.ingest()` — everything else throws, since this shim exists
 * solely to redirect the WASM engine's internal store write to `POST
 * /ingest` instead of a local OPFS/SQLite write.
 */
function createHttpStore(onUpsert: (fullText: string) => void): VectorStoreInterface {
  const notSupported = (name: string) => async () => {
    throw new Error(`${name} is not supported by the browser HTTP-backed store`);
  };
  return {
    close: async () => undefined,
    ensureCollection: notSupported("ensureCollection") as (spec: CollectionSpec) => Promise<string | void>,
    dropCollection: notSupported("dropCollection"),
    getCollection: notSupported("getCollection"),
    deleteDocuments: notSupported("deleteDocuments"),
    deleteByFilter: notSupported("deleteByFilter"),
    retrieve: notSupported("retrieve"),
    collectionStats: notSupported("collectionStats"),
    async upsertDocument(collection: string, doc: DocumentRecord, chunks: ChunkRecord[]): Promise<string> {
      const badChunk = chunks.find((c) => c.embedding.length !== EMBEDDING_DIM);
      if (badChunk) {
        throw new Error(
          `embedder produced ${badChunk.embedding.length}-dim vectors at ordinal ${badChunk.ordinal}, expected ${EMBEDDING_DIM} (EMBEDDING_DIM constant is stale — update it and the /collection embedding_dim together)`
        );
      }
      onUpsert(doc.full_text);
      const { document_id } = await postIngest(mcpBaseUrl, {
        collection,
        external_id: doc.external_id ?? "",
        title: doc.title,
        mime: doc.mime,
        source_uri: doc.source_uri,
        full_text: doc.full_text,
        keywords: doc.keywords,
        metadata: doc.metadata as Record<string, unknown> | undefined,
        chunks: chunks.map((c) => ({ ordinal: c.ordinal, content: c.content, embedding: c.embedding, chunk_metadata: c.chunk_metadata })),
      });
      return document_id;
    },
  };
}

async function getEngine(): Promise<XbergEngine> {
  if (engine) return engine;
  const injection = await createXbergRuntimeFactory();
  injection.store = createHttpStore((fullText) => {
    lastRedactedFullText = fullText;
  });
  engine = new XbergEngine({}, injection);
  return engine;
}

function post(msg: unknown, transfer: Transferable[] = []): void {
  self.postMessage(msg, transfer);
}

async function handleIngest(msg: IngestMessage): Promise<void> {
  const { requestId, file, filename, mime, collection, passphrase } = msg;
  try {
    const xEngine = await getEngine();
    const externalId = sanitizeExternalId(filename);
    const bytes = new Uint8Array(await file.arrayBuffer());

    post({ type: "progress", requestId, stage: "extract" });
    const extracted = await xEngine.extract({ kind: "bytes", bytes, filename }, undefined);
    const first = (extracted as { results?: Array<{ content: string; mimeType: string }> }).results?.[0];
    if (!first) throw new Error(`extraction produced no result for ${filename}`);

    post({ type: "progress", requestId, stage: "ingest" });
    const outcome = (await xEngine.ingest(
      { full_text: first.content, title: filename, mime: first.mimeType || mime, source_uri: filename, external_id: externalId },
      collection
    )) as { document_id: string; rehydration_map: Record<string, string>; pii_category_counts: Record<string, number> };

    post({ type: "progress", requestId, stage: "encrypt" });
    const blob = xEngine.encrypt_map(outcome.rehydration_map, passphrase);

    post({ type: "progress", requestId, stage: "map" });
    // MUST be `externalId`, NOT `outcome.document_id`. `/map`'s `document_id`
    // query param and `rehydrate_tokens`'s `document_id` argument are both
    // named after the *file's* base name (see `mcp-server/src/tools/ingest.ts`:
    // `path.join(rehydrationDir, \`${baseName}.map\`)`), not the store's
    // generated UUID — despite the store's return value happening to also be
    // called `document_id`. These are two different things that share a
    // name; using the UUID here writes a map file no rehydration tool can
    // ever find by the id a human/UI would actually have on hand.
    await postMap(mcpBaseUrl, externalId, blob);

    const entry: IngestHistoryEntry = {
      collection,
      externalId,
      filename,
      mime: first.mimeType || mime,
      redactedText: lastRedactedFullText,
      piiCategoryCounts: outcome.pii_category_counts,
      documentId: outcome.document_id,
      status: "synced",
      ingestedAt: Date.now(),
    };
    post({ type: "result", requestId, entry });
  } catch (err) {
    post({ type: "error", requestId, message: err instanceof Error ? err.message : String(err) });
  }
}

async function handleOcr(msg: OcrMessage): Promise<void> {
  try {
    const xEngine = await getEngine();
    // `engine.ocr` returns the recognized text as a single string (no
    // per-line geometry from the WASM OCR bridge), so split on newlines
    // to recover lines; confidence is unavailable and defaults to 1.
    const text = (await xEngine.ocr(msg.bytes, undefined)) as string;
    const lines = text.split(/\r?\n/).map((t) => ({ text: t, confidence: 1 }));
    post({ type: "ocrResult", requestId: msg.requestId, lines });
  } catch (err) {
    post({ type: "error", requestId: msg.requestId, message: err instanceof Error ? err.message : String(err) });
  }
}

self.addEventListener("message", (ev: MessageEvent) => {
  const msg = ev.data as IngestMessage | OcrMessage;
  if (msg.type === "ingest") {
    mcpBaseUrl = msg.mcpBaseUrl;
    lastIngest = lastIngest.then(() => handleIngest(msg));
  } else if (msg.type === "ocr") {
    void handleOcr(msg);
  }
});
