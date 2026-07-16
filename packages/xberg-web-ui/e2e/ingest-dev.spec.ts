// e2e/ingest-dev.spec.ts
//
// Runs the real browser wasm engine end-to-end against the DEV server
// (http://127.0.0.1:3210) instead of the static `out/` export. The production
// `next build` currently fails minifying onnxruntime-web's WebGPU/JSEP module
// (a pre-existing Next + onnxruntime-web ESM issue, unrelated to the wasm
// loading fix) — dev does not minify, so this exercises the same runtime path.
//
// The harness proxies `/ui` to the dev server while injecting the COOP/COEP
// headers the Worker/OPFS/wasm path requires, and mocks the MCP `/collection`,
// `/ingest`, `/map` endpoints the UI calls.
import { test, expect } from "@playwright/test";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import http from "node:http";
import { EMBEDDING_DIM } from "../src/lib/constants.js";

const DEV = "http://127.0.0.1:3210";

function proxyUi(req: IncomingMessage, res: ServerResponse): void {
  const url = new URL(req.url ?? "/", DEV);
  const headers = { ...req.headers };
  delete (headers as Record<string, unknown>).host;
  const proxy = http.request(
    url,
    { method: req.method, headers },
    (pres) => {
      res.writeHead(pres.statusCode ?? 500, {
        ...pres.headers,
        "Cross-Origin-Opener-Policy": "same-origin",
        "Cross-Origin-Embedder-Policy": "require-corp",
      });
      pres.pipe(res);
    }
  );
  proxy.on("error", () => res.writeHead(502).end("proxy error"));
  req.pipe(proxy);
}

test("browser wasm engine ingests a PII doc with redaction (dev proxy)", async ({ page }) => {
  const received: { collection?: unknown; ingest?: unknown; mapDocumentId?: string } = {};
  const server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    const send = (status: number, body: unknown) => {
      res.writeHead(status, { "Content-Type": "application/json" });
      res.end(JSON.stringify(body));
    };
    if (req.method === "POST" && url.pathname === "/collection") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        received.collection = JSON.parse(body);
        send(200, { created: true });
      });
      return;
    }
    if (req.method === "POST" && url.pathname === "/ingest") {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        received.ingest = JSON.parse(body);
        send(200, { document_id: "doc-e2e-1" });
      });
      return;
    }
    if (req.method === "POST" && url.pathname === "/map") {
      received.mapDocumentId = url.searchParams.get("document_id") ?? undefined;
      req.on("data", () => undefined);
      req.on("end", () => send(200, { status: "stored" }));
      return;
    }
    if (url.pathname === "/ui" || url.pathname.startsWith("/ui/")) {
      proxyUi(req, res);
      return;
    }
    send(404, {});
  });
  const port = await new Promise<number>((resolve) =>
    server.listen(0, "127.0.0.1", () => resolve((server.address() as { port: number }).port))
  );
  const baseUrl = `http://127.0.0.1:${port}`;

  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));

  try {
    await page.goto(`${baseUrl}/ui/?token=test`, { waitUntil: "domcontentloaded" });
    await page.getByText("New folder").click();
    await page.getByLabel("Folder name").fill("contrats");
    await page.getByRole("button", { name: "Create" }).click();
    // Dev build can leave the dialog overlay mounted briefly; dismiss it.
    await page.keyboard.press("Escape");
    await page
      .getByText("contrats")
      .click({ timeout: 15_000 });

    await page.getByLabel(/passphrase/i).fill("correct-horse-battery");
    await page.setInputFiles("input[type=file]", {
      name: "contrat.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("Contact alice@example.com about the contract"),
    });

    await expect
      .poll(() => received.ingest !== undefined, { timeout: 60_000 })
      .toBe(true);
    expect(received.collection).toEqual({ name: "contrats", embedding_dim: EMBEDDING_DIM });
    await expect.poll(() => received.mapDocumentId, { timeout: 10_000 }).toBe("contrat.pdf");
    expect((received.ingest as { external_id: string }).external_id).toBe("contrat.pdf");
    expect((received.ingest as { full_text: string }).full_text).not.toContain(
      "alice@example.com"
    );

    // The wasm engine must have initialized without runtime errors.
    expect(errors.filter((e) => /wasm|env|init|instantiate/i.test(e))).toEqual([]);
  } catch (err) {
    const banner = await page
      .evaluate(() => document.body.innerText.slice(0, 800))
      .catch(() => "(eval failed)");
    console.log("=== CONSOLE/PAGE ERRORS ===\n" + errors.join("\n"));
    console.log("=== PAGE TEXT (first 800) ===\n" + banner);
    throw err;
  } finally {
    server.close();
  }
});
