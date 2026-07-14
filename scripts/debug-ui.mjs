// scripts/debug-ui.mjs — load the dev UI through the proxy harness and dump
// all console output + page errors + the visible error banner, so we can see
// why the browser wasm engine fails to initialize.
import { chromium } from "@playwright/test";
import { createServer } from "node:http";
import http from "node:http";

const DEV = "http://127.0.0.1:3210";

function proxyUi(req, res) {
  const url = new URL(req.url ?? "/", DEV);
  const headers = { ...req.headers };
  delete headers.host;
  const proxy = http.request(url, { method: req.method, headers }, (pres) => {
    res.writeHead(pres.statusCode ?? 500, {
      ...pres.headers,
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    });
    pres.pipe(res);
  });
  proxy.on("error", () => res.writeHead(502).end("proxy error"));
  req.pipe(proxy);
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? "/", "http://localhost");
  if (url.pathname === "/ui" || url.pathname.startsWith("/ui/")) return proxyUi(req, res);
  res.writeHead(404).end();
});
await new Promise((r) => server.listen(8082, "127.0.0.1", r));

const browser = await chromium.launch();
const page = await browser.newPage();
page.on("console", (m) => console.log(`[console.${m.type()}]`, m.text()));
page.on("pageerror", (e) => console.log("[pageerror]", String(e)));

await page.goto("http://127.0.0.1:8082/ui/?token=test", { waitUntil: "domcontentloaded" });
await page.waitForTimeout(8000);

const banner = await page.evaluate(() => {
  const el = document.querySelector('[class*="text-red"], .alert, [role="alert"]');
  return el ? el.textContent : "(no alert/banner found)";
}).catch((e) => "(eval failed: " + e + ")");
console.log("=== visible error banner ===\n", banner);

await browser.close();
server.close();
