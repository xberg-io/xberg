import { z } from "zod";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { VectorStoreInterface } from "xberg-wasm-runtime";

const MAX_BODY_BYTES = 1 * 1024 * 1024;
const AdminPayloadSchema = z.discriminatedUnion("op", [
  z.object({ op: z.literal("drop_collection"), collection: z.string().min(1) }),
  z.object({ op: z.literal("delete_documents"), collection: z.string().min(1), external_ids: z.array(z.string().min(1)).min(1) }),
  z.object({ op: z.literal("stats"), collection: z.string().min(1) }),
]);
export type AdminPayload = z.infer<typeof AdminPayloadSchema>;

function statusForError(message: string): number { return message.includes("not found") ? 404 : 400; }

export function createAdminHandler(
  getStore: () => VectorStoreInterface,
): (req: IncomingMessage, res: ServerResponse, url: URL) => Promise<void> {
  return async function handleAdmin(req: IncomingMessage, res: ServerResponse, _url: URL): Promise<void> {
    const chunks: Buffer[] = []; let total = 0;
    try {
      for await (const c of req) {
        total += (c as Buffer).length;
        if (total > MAX_BODY_BYTES) {
          res.writeHead(413, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "payload too large" }));
          req.resume();
          return;
        }
        chunks.push(c as Buffer);
      }
    } catch (e) {
      res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid JSON body" }));
      return;
    }
    let json: unknown;
    try {
      json = JSON.parse(Buffer.concat(chunks).toString("utf-8"));
    } catch {
      res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid JSON body" }));
      return;
    }
    const parsed = AdminPayloadSchema.safeParse(json);
    if (!parsed.success) { res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid admin payload", issues: parsed.error.issues })); return; }
    const p = parsed.data;
    try {
      if (p.op === "drop_collection") { await getStore().dropCollection(p.collection); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ dropped: true })); }
      else if (p.op === "delete_documents") { const deleted = await getStore().deleteDocuments(p.collection, p.external_ids); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ deleted })); }
      else { const stats = await getStore().collectionStats(p.collection); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify(stats)); }
    } catch (err) { const msg = err instanceof Error ? err.message : String(err); res.writeHead(statusForError(msg), { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg })); }
  };
}
