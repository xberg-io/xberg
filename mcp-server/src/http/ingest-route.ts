// mcp-server/src/http/ingest-route.ts
import { z } from "zod";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "xberg-wasm-runtime";

const MAX_BODY_BYTES = 10 * 1024 * 1024; // 10 MiB
const MAX_CHUNKS = 10_000;

const ChunkPayloadSchema = z.object({
  ordinal: z.number().int().nonnegative(),
  content: z.string(),
  embedding: z.array(z.number()),
  chunk_metadata: z.unknown().optional(),
});

/**
 * Wire contract pushed by the browser after its WASM pipeline has already
 * extracted, OCR'd, NER'd, PII-redacted, chunked, and embedded a document.
 * `external_id` is the caller-chosen idempotence key: re-posting the same
 * `(collection, external_id)` pair replaces that document's chunks (see
 * `VectorStoreInterface.upsertDocument`, `store-node.ts:186-251`) — reuse the
 * same string as the `document_id` query param on `/map`.
 */
const IngestPayloadSchema = z.object({
  collection: z.string().min(1),
  external_id: z.string().min(1),
  title: z.string().optional(),
  mime: z.string().optional(),
  source_uri: z.string().optional(),
  full_text: z.string(),
  keywords: z.array(z.string()).optional(),
  metadata: z.record(z.unknown()).optional(),
  chunks: z.array(ChunkPayloadSchema).max(MAX_CHUNKS),
});

export type IngestPayload = z.infer<typeof IngestPayloadSchema>;

function statusForError(message: string): number {
  return message.includes("not found") ? 404 : 400;
}

/**
 * Build the `POST /ingest` handler. `getStore` is a lazy getter (not a bound
 * value) so the caller can defer to `getRuntime().store`, which only exists
 * after `initializeEngine()` resolves.
 */
export function createIngestHandler(
  getStore: () => VectorStoreInterface
): (req: IncomingMessage, res: ServerResponse) => Promise<void> {
  return async function handleIngest(req: IncomingMessage, res: ServerResponse): Promise<void> {
    try {
      const chunks: Buffer[] = [];
      let totalBytes = 0;
      for await (const chunk of req) {
        totalBytes += (chunk as Buffer).length;
        if (totalBytes > MAX_BODY_BYTES) {
          res.writeHead(413, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "payload too large" }));
          return;
        }
        chunks.push(chunk as Buffer);
      }

      let json: unknown;
      try {
        json = JSON.parse(Buffer.concat(chunks).toString("utf-8"));
      } catch {
        res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid JSON body" }));
        return;
      }

      const parsed = IngestPayloadSchema.safeParse(json);
      if (!parsed.success) {
        res
          .writeHead(400, { "Content-Type": "application/json" })
          .end(JSON.stringify({ error: "invalid payload", issues: parsed.error.issues }));
        return;
      }
      const payload = parsed.data;

      const doc: DocumentRecord = {
        external_id: payload.external_id,
        title: payload.title,
        mime: payload.mime,
        source_uri: payload.source_uri,
        full_text: payload.full_text,
        keywords: payload.keywords,
        metadata: payload.metadata,
      };
      const chunkRecords: ChunkRecord[] = payload.chunks.map((c) => ({
        ordinal: c.ordinal,
        content: c.content,
        embedding: c.embedding,
        chunk_metadata: c.chunk_metadata,
      }));

      const documentId = await getStore().upsertDocument(payload.collection, doc, chunkRecords);
      res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ document_id: documentId }));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      const status = msg === "payload too large" ? 413 : statusForError(msg);
      if (!res.headersSent) {
        res.writeHead(status, { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg }));
      } else {
        res.end();
      }
    }
  };
}
