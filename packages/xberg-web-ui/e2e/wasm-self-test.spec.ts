// e2e/wasm-self-test.spec.ts
//
// Drives the wasm engine directly (via /ui/wasm-self-test) to verify the
// wasm-pack "web" target loads and the patched host libc shim works — without
// the folder-dialog UI (which has a pre-existing hydration bug).
import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import http from "node:http";

// Headless chromium advertises navigator.gpu, which makes the runtime pick the
// ORT WebGPU (JSEP) backend; its wasm fetch from the CDN stalls in this
// environment. Force the reliable WASM-CPU backend for the self-test.
test.use({ launchOptions: { args: ["--disable-features=WebGPU,Vulkan"] } });

const DEV = "http://127.0.0.1:3210";

function proxyUi(req: import("node:http").IncomingMessage, res: import("node:http").ServerResponse): void {
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

test("wasm engine initializes, builds runtime injection, and embeds (web target)", async ({ page }) => {
  test.setTimeout(150_000);
  const errors: string[] = [];
  const logs: string[] = [];
  const pending = new Map<string, string>();
  page.on("console", (m) => {
    logs.push(`[${m.type()}] ${m.text()}`);
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("request", (r) => pending.set(r.url(), `${r.method()} ${r.url()}`));
  page.on("requestfinished", (r) => pending.delete(r.url()));
  page.on("requestfailed", (r) => {
    pending.delete(r.url());
    logs.push(`[reqfail] ${r.url()} ${r.failure()?.errorText ?? ""}`);
  });

  const server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    if (url.pathname === "/ui" || url.pathname.startsWith("/ui/")) return proxyUi(req, res);
    res.writeHead(404).end();
  });
  await new Promise<void>((resolve) => server.listen(8083, "127.0.0.1", resolve));

  try {
    await page.goto("http://127.0.0.1:8083/ui/wasm-self-test", {
      waitUntil: "domcontentloaded",
    });
    const result = page.getByTestId("result");
    await expect
      .poll(async () => (await result.textContent()) ?? "", { timeout: 130_000 })
      .toMatch(/^OK /);
    expect(errors.filter((e) => /wasm|env\b|init|instantiate|makeEnv/i.test(e))).toEqual([]);
  } catch (err) {
    const txt = await page.getByTestId("result").textContent().catch(() => "(none)");
    console.log("=== RESULT ===\n" + txt);
    console.log("=== ERRORS ===\n" + errors.join("\n"));
    console.log("=== LOGS (tail) ===\n" + logs.slice(-60).join("\n"));
    console.log("=== PENDING REQUESTS ===\n" + Array.from(pending.values()).join("\n"));
    throw err;
  } finally {
    server.close();
  }
});
