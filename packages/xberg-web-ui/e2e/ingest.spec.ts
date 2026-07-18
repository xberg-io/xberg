// e2e/ingest.spec.ts
//
// Requires `packages/xberg-web-ui/out/` to exist (i.e. `next build` followed
// by `node scripts/export-to-mcp.mjs`, or the package's own `pnpm export`,
// must have been run first). This test cannot be verified in an environment
// where the wasm binary build fails — same "cannot verify without a
// successful build" limitation this plan has used consistently elsewhere
// (Task 1's `http-ui-routes.test.ts`, Task 6/9's wasm-dependent pieces).
import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { EMBEDDING_DIM } from "../src/lib/constants.js";
import { serveStaticFile } from "../../../mcp-server/src/http/static-server.js";
import { resolveUiSubPath } from "../../../mcp-server/src/http/ui-route-resolver.js";

const OUT_DIR = join(fileURLToPath(new URL(".", import.meta.url)), "..", "out");

test("uploading a document with PII syncs to the MCP store via /collection, /ingest, /map", async ({ page }) => {
  // First-load model download + WASM init (bge-m3 embedder, Candle GLiNER2
  // NER) has taken several minutes in this environment -- the config-level
  // 60s default is nowhere near enough for the real (non-mocked) engine
  // this test exercises. KNOWN OPEN ISSUE: past model init, `xEngine
  // .extract()`/`.ingest()` (the native WASM call path) has been observed
  // to stall indefinitely with flat CPU usage in this environment -- a
  // separate, deeper bug from the one this test file's other fixes address
  // (see FolderPageClient.tsx's onFiles for the fixed live-FileList bug).
  // Not yet root-caused; needs a Promise.race timeout wrapper analogous to
  // handleOcr's OCR_TIMEOUT_MS in engine.worker.ts, or investigation inside
  // crates/xberg-wasm/src/engine.rs's extract/ingest bindings.
  test.setTimeout(300_000);
  const received: { collection?: unknown; ingest?: unknown; mapDocumentId?: string } = {};
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
    if (req.method === "GET" && (url.pathname === "/ui" || url.pathname.startsWith("/ui/"))) {
      // Reuse the real static-serving primitives from `mcp-server/src/http/`
      // (the same ones `ui-server.ts` uses) so this test actually exercises
      // production request handling, including the COOP/COEP headers the
      // Worker/OPFS/wasm path requires AND the dynamic-route fallback
      // (`resolveUiSubPath`) that rewrites e.g. `/folder/<real-name>` to the
      // static export's placeholder shell -- skipping that step (as this
      // test previously did, calling `serveStaticFile` directly) 404s on
      // every real folder/document route, since Next's static export only
      // ever generates the placeholder param combination.
      const subPath = url.pathname === "/ui" ? "/" : url.pathname.slice("/ui".length);
      serveStaticFile(OUT_DIR, resolveUiSubPath(OUT_DIR, subPath), res);
      return;
    }
    send(404, {});
  });
  await new Promise<void>((resolve) => server.listen(8081, "127.0.0.1", resolve));

  const errors: string[] = [];
  const logs: string[] = [];
  const pending = new Map<string, string>();
  const timeline: string[] = [];
  page.on("console", (m) => {
    logs.push(`[${m.type()}] ${m.text()}`);
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("worker", (w) => {
    timeline.push(`[worker created] ${w.url()}`);
    w.on("close", () => timeline.push(`[worker closed] ${w.url()}`));
  });
  page.on("request", (r) => {
    pending.set(r.url(), `${r.method()} ${r.url()}`);
    timeline.push(`[req] ${r.method()} ${r.url()}`);
  });
  page.on("requestfinished", (r) => {
    pending.delete(r.url());
    timeline.push(`[done] ${r.url()}`);
  });
  page.on("requestfailed", (r) => {
    pending.delete(r.url());
    timeline.push(`[reqfail] ${r.url()} ${r.failure()?.errorText ?? ""}`);
  });

  try {
    await page.goto("http://127.0.0.1:8081/ui/?token=test");
    await page.getByText("New folder").click();
    await page.getByLabel("Folder name").fill("contrats");
    await page.getByRole("button", { name: "Create" }).click();
    // The dialog overlay can stay mounted briefly after a successful
    // create, intercepting the click on the folder link underneath it
    // (see ingest-dev.spec.ts's identical workaround).
    await page.keyboard.press("Escape");
    await page.getByText("contrats").click({ timeout: 15_000 });
    // Clicking the folder link is a hard navigation (Next's static export has
    // no client-side route for a real `/folder/<name>` URL -- see
    // FolderPageClient.tsx's comment on `route-params.ts`). If the passphrase
    // is filled before that reload finishes settling, the fill can land on a
    // page instance that's about to be torn down, silently resetting the
    // input back to empty before the file-select fires -- `onFiles` then sees
    // `passphrase === ""` and returns early with no error, no network call,
    // and no visible symptom at all. Wait for the real heading to render
    // (confirms the post-navigation page is the active one) before filling.
    await page.getByRole("heading", { name: "contrats" }).waitFor({ timeout: 15_000 });
    await page.waitForLoadState("networkidle");

    const passphraseInput = page.getByLabel(/passphrase/i);
    await passphraseInput.fill("correct-horse-battery");
    await expect(passphraseInput).toHaveValue("correct-horse-battery");
    await page.setInputFiles("input[type=file]", {
      name: "contrat.pdf",
      mimeType: "application/pdf",
      buffer: Buffer.from("Contact alice@example.com about the contract"),
    });

    await expect.poll(() => received.ingest !== undefined, { timeout: 240_000 }).toBe(true);
    expect(received.collection).toEqual({ name: "contrats", embedding_dim: EMBEDDING_DIM });
    await expect.poll(() => received.mapDocumentId, { timeout: 10_000 }).toBe("contrat.pdf");
    expect((received.ingest as { external_id: string }).external_id).toBe("contrat.pdf");
    expect((received.ingest as { full_text: string }).full_text).not.toContain("alice@example.com");
  } catch (err) {
    const banner = await page
      .evaluate(() => document.body.innerText.slice(0, 800))
      .catch(() => "(eval failed)");
    console.log("=== CONSOLE/PAGE ERRORS ===\n" + errors.join("\n"));
    console.log("=== PAGE TEXT (first 800) ===\n" + banner);
    console.log("=== LOGS (tail) ===\n" + logs.slice(-80).join("\n"));
    console.log("=== PENDING REQUESTS ===\n" + Array.from(pending.values()).join("\n"));
    console.log(`=== TIMELINE (${timeline.length} events, last 100) ===\n` + timeline.slice(-100).join("\n"));
    throw err;
  } finally {
    server.close();
  }
});
