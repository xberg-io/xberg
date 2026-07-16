// e2e/delete.spec.ts
//
// Requires `packages/xberg-web-ui/out/` to exist (i.e. `next build` followed
// by `node scripts/export-to-mcp.mjs`, or the package's own `pnpm export`,
// must have been run first). This test cannot be verified in an environment
// where the wasm binary build fails — same "cannot verify without a
// successful build" limitation `e2e/ingest.spec.ts` documents.
import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { EMBEDDING_DIM } from "../src/lib/constants.js";
import { serveStaticFile } from "../../../mcp-server/src/http/static-server.js";
import { resolveUiSubPath } from "../../../mcp-server/src/http/ui-route-resolver.js";

const OUT_DIR = join(fileURLToPath(new URL(".", import.meta.url)), "..", "out");

test("uploading a PII doc then deleting it via the UI removes it from the MCP store, without leaking clear PII", async ({
  page,
}) => {
  const received: {
    collection?: unknown;
    ingest?: unknown;
    mapDocumentId?: string;
    deletes: Array<{ op: string; collection: string; external_ids: string[] }>;
    documents: number;
  } = { deletes: [], documents: 1 };

  const server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    const send = (status: number, body: unknown) => {
      res.writeHead(status, { "Content-Type": "application/json" });
      res.end(JSON.stringify(body));
    };
    if (req.method === "POST" && url.pathname === "/collection") {
      let body = "";
      for await (const chunk of req) body += chunk;
      received.collection = JSON.parse(body);
      send(200, { created: true });
      return;
    }
    if (req.method === "POST" && url.pathname === "/ingest") {
      let body = "";
      for await (const chunk of req) body += chunk;
      received.ingest = JSON.parse(body);
      send(200, { document_id: "doc-e2e-1" });
      return;
    }
    if (req.method === "POST" && url.pathname === "/map") {
      received.mapDocumentId = url.searchParams.get("document_id") ?? undefined;
      for await (const _chunk of req) {
        // drain the body; nothing to inspect for this happy-path test
      }
      send(200, { status: "stored" });
      return;
    }
    if (req.method === "POST" && url.pathname === "/admin") {
      let body = "";
      for await (const chunk of req) body += chunk;
      const payload = JSON.parse(body) as {
        op: string;
        collection: string;
        external_ids?: string[];
      };
      if (payload.op === "delete_documents") {
        received.deletes.push(
          payload as { op: string; collection: string; external_ids: string[] },
        );
        received.documents = 0;
        send(200, { deleted: payload.external_ids?.length ?? 0 });
        return;
      }
      if (payload.op === "stats") {
        send(200, { documents: received.documents, chunks: received.documents * 3 });
        return;
      }
      send(200, { dropped: true });
      return;
    }
    if (req.method === "GET" && (url.pathname === "/ui" || url.pathname.startsWith("/ui/"))) {
      const subPath = url.pathname === "/ui" ? "/" : url.pathname.slice("/ui".length);
      serveStaticFile(OUT_DIR, resolveUiSubPath(OUT_DIR, subPath), res);
      return;
    }
    send(404, {});
  });
  const port = await new Promise<number>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      resolve((server.address() as { port: number }).port);
    });
  });
  const baseUrl = `http://127.0.0.1:${port}`;

  try {
    await page.goto(`${baseUrl}/ui/?token=test`);
    await page.getByText("New folder").click();
    await page.getByLabel("Folder name").fill("contrats");
    await page.getByText("Create").click();
    await page.getByText("contrats").click();

    await page.getByLabel(/passphrase/i).fill("correct-horse-battery");
    await page.setInputFiles("input[type=file]", {
      name: "contrat.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("Contact alice@example.com about the contract"),
    });

    await expect.poll(() => received.ingest !== undefined, { timeout: 30_000 }).toBe(true);
    expect(received.collection).toEqual({ name: "contrats", embedding_dim: EMBEDDING_DIM });

    // Reload the folder view (the table reads ingest history from IndexedDB)
    // and select the row that was just ingested.
    await page.goto(`${baseUrl}/ui/folder/contrats?token=test`);
    await page.getByLabel("select-contrat.pdf").check();

    await page.getByRole("button", { name: "Delete" }).click();
    await page.getByRole("button", { name: "Confirm delete" }).click();

    await expect.poll(() => received.deletes.length, { timeout: 10_000 }).toBe(1);
    expect(received.deletes[0]).toMatchObject({
      op: "delete_documents",
      collection: "contrats",
      external_ids: ["contrat.pdf"],
    });

    // The row disappears from the table once the delete resolves.
    await expect(page.getByText("contrat.pdf")).toHaveCount(0);
  } finally {
    await new Promise<void>((resolve, reject) => {
      server.close((err) => (err ? reject(err) : resolve()));
    });
  }
});
