import type { IncomingMessage, ServerResponse } from "node:http";
import { mkdir, writeFile, rename, unlink } from "node:fs/promises";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

// Allows plain identifiers and filename-safe punctuation only — no `/` or
// `\`, so the resulting filename can never escape `getRehydrationDir()`.
const DOCUMENT_ID_PATTERN = /^[A-Za-z0-9_.-]+$/;

// Upper bound on the encrypted map blob. The rehydration map is a small
// AES-256-GCM artifact (salt + iv + tag + ciphertext); anything larger is
// either malicious or malformed.
const MAX_BODY_BYTES = 16 * 1024 * 1024;

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { "Content-Type": "application/json" }).end(JSON.stringify(body));
}

/**
 * Build the `POST /map` handler. The body is the already-encrypted
 * rehydration map blob produced client-side by `engine.encrypt_map()` (wire
 * format: `XPII\x01` + salt + iv + tag + ciphertext, matching
 * `mcp-server/src/redaction/rehydration.ts`'s `encryptMapFile`) — the server
 * writes it verbatim and never sees the passphrase.
 */
export function createMapUploadHandler(
  getRehydrationDir: () => string
): (req: IncomingMessage, res: ServerResponse, url: URL) => Promise<void> {
  return async function handleMapUpload(req: IncomingMessage, res: ServerResponse, url: URL): Promise<void> {
    const documentId = url.searchParams.get("document_id");
    if (!documentId || !DOCUMENT_ID_PATTERN.test(documentId)) {
      sendJson(res, 400, {
        error: "document_id query param must match [A-Za-z0-9_.-]+",
      });
      return;
    }

    const chunks: Buffer[] = [];
    let total = 0;
    try {
      for await (const chunk of req) {
        const buf = chunk as Buffer;
        total += buf.length;
        if (total > MAX_BODY_BYTES) {
          sendJson(res, 413, { error: "body too large" });
          req.resume();
          return;
        }
        chunks.push(buf);
      }

      const body = Buffer.concat(chunks);
      if (body.length === 0) {
        sendJson(res, 400, { error: "empty body" });
        return;
      }

      const dir = getRehydrationDir();
      await mkdir(dir, { recursive: true });
      const mapPath = join(dir, `${documentId}.map`);
      const tmpPath = `${mapPath}.${randomBytes(8).toString("hex")}.tmp`;
      try {
        await writeFile(tmpPath, body);
        await rename(tmpPath, mapPath);
      } catch (writeErr) {
        await unlink(tmpPath).catch(() => {});
        throw writeErr;
      }

      sendJson(res, 200, { status: "stored" });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (!res.headersSent) {
        res.writeHead(500, { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg }));
      } else {
        res.end();
      }
    }
  };
}
