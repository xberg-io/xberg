import { describe, it, expect, afterEach } from "vitest";
import { createServer, type Server } from "node:http";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createMapUploadHandler } from "../src/http/map-route.js";

describe("http/map-route", () => {
  let dir: string;
  let server: Server | null = null;

  afterEach(async () => {
    if (server) {
      await new Promise<void>((resolve, reject) => server!.close((err) => (err ? reject(err) : resolve())));
      server = null;
    }
    if (dir) rmSync(dir, { recursive: true, force: true });
  });

  async function withServer(fn: (baseUrl: string) => Promise<void>): Promise<void> {
    dir = mkdtempSync(join(tmpdir(), "xberg-map-test-"));
    const handler = createMapUploadHandler(() => dir);
    server = createServer((req, res) => {
      const url = new URL(req.url ?? "/", "http://localhost");
      void handler(req, res, url);
    });
    await new Promise<void>((resolve) => server!.listen(0, "127.0.0.1", resolve));
    const address = server!.address();
    if (address === null || typeof address === "string") throw new Error("expected AddressInfo");
    await fn(`http://127.0.0.1:${address.port}`);
  }

  it("writes the raw encrypted body to <dir>/<document_id>.map", async () => {
    await withServer(async (baseUrl) => {
      const blob = Buffer.from("XPII\x01fake-bytes");
      const res = await fetch(`${baseUrl}/map?document_id=doc-1`, { method: "POST", body: blob });
      expect(res.status).toBe(200);

      const mapPath = join(dir, "doc-1.map");
      expect(existsSync(mapPath)).toBe(true);
      expect(readFileSync(mapPath).equals(blob)).toBe(true);
    });
  });

  it("rejects a missing document_id with 400", async () => {
    await withServer(async (baseUrl) => {
      const res = await fetch(`${baseUrl}/map`, { method: "POST", body: Buffer.from("x") });
      expect(res.status).toBe(400);
    });
  });

  it("rejects a document_id that would escape the rehydration dir", async () => {
    await withServer(async (baseUrl) => {
      const res = await fetch(`${baseUrl}/map?document_id=${encodeURIComponent("../../etc/passwd")}`, {
        method: "POST",
        body: Buffer.from("x"),
      });
      expect(res.status).toBe(400);
    });
  });

  it("rejects an empty body with 400", async () => {
    await withServer(async (baseUrl) => {
      const res = await fetch(`${baseUrl}/map?document_id=doc-2`, { method: "POST", body: Buffer.alloc(0) });
      expect(res.status).toBe(400);
    });
  });

  it("rejects a body larger than the cap with 413", async () => {
    await withServer(async (baseUrl) => {
      const big = Buffer.alloc(16 * 1024 * 1024 + 1);
      const res = await fetch(`${baseUrl}/map?document_id=doc-3`, { method: "POST", body: big });
      expect(res.status).toBe(413);
    });
  });

  it("handles concurrent uploads for the same document_id atomically", async () => {
    await withServer(async (baseUrl) => {
      const blob1 = Buffer.from("XPII\x01data-version-1");
      const blob2 = Buffer.from("XPII\x01data-version-2");

      // Fire two concurrent uploads for the same document_id
      const [res1, res2] = await Promise.all([
        fetch(`${baseUrl}/map?document_id=concurrent-doc`, { method: "POST", body: blob1 }),
        fetch(`${baseUrl}/map?document_id=concurrent-doc`, { method: "POST", body: blob2 }),
      ]);

      expect(res1.status).toBe(200);
      expect(res2.status).toBe(200);

      // Read the final file and verify it's one of the two complete blobs (not corrupted/interleaved)
      const mapPath = join(dir, "concurrent-doc.map");
      expect(existsSync(mapPath)).toBe(true);
      const finalContent = readFileSync(mapPath);

      // The file should match exactly one of the two uploads (atomic write ensures this)
      const matches1 = finalContent.equals(blob1);
      const matches2 = finalContent.equals(blob2);
      expect(matches1 || matches2).toBe(true);
    });
  });
});
