import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { createServer, type Server } from "node:http";
import { mkdtempSync, writeFileSync, rmSync, symlinkSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { platform } from "node:process";
import { resolveSafePath, serveStaticFile } from "../src/http/static-server.js";

describe("http/static-server", () => {
  let server: Server;
  let baseUrl: string;
  let dir: string;

  beforeAll(async () => {
    dir = mkdtempSync(join(tmpdir(), "xberg-ui-test-"));
    writeFileSync(join(dir, "index.html"), "<html><body>hi</body></html>");
    writeFileSync(join(dir, "app.js"), "console.log('hi');");

    server = createServer((req, res) => {
      serveStaticFile(dir, req.url ?? "/", res);
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    if (address === null || typeof address === "string") throw new Error("expected an AddressInfo");
    baseUrl = `http://127.0.0.1:${address.port}`;
  });

  afterAll(async () => {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
    rmSync(dir, { recursive: true, force: true });
  });

  it("serves index.html at the root with COOP/COEP headers", async () => {
    const res = await fetch(`${baseUrl}/`);
    expect(res.status).toBe(200);
    expect(await res.text()).toContain("hi");
    expect(res.headers.get("cross-origin-opener-policy")).toBe("same-origin");
    expect(res.headers.get("cross-origin-embedder-policy")).toBe("require-corp");
  });

  it("serves a JS asset with the correct content-type", async () => {
    const res = await fetch(`${baseUrl}/app.js`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/javascript");
  });

  it("returns 404 for a missing file", async () => {
    const res = await fetch(`${baseUrl}/missing.html`);
    expect(res.status).toBe(404);
  });

  it("resolveSafePath resolves a normal path inside the root", () => {
    expect(resolveSafePath(dir, "/app.js")).toBe(join(dir, "app.js"));
  });

  it("resolveSafePath rejects a traversal attempt above the root", () => {
    expect(resolveSafePath(dir, "/../../../etc/passwd")).toBeNull();
    expect(resolveSafePath(dir, "/..%2f..%2f..%2fetc%2fpasswd")).toBeNull();
  });

  it("returns 403 for a traversal attempt over the wire", async () => {
    const url = new URL(baseUrl);
    const status = await new Promise<number>((resolve, reject) => {
      const socket = connect(Number(url.port), url.hostname, () => {
        // Send a literal request line; fetch/http clients normalize ".." away.
        socket.write("GET /../../../../etc/passwd HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
      });
      let buffer = "";
      socket.on("data", (chunk) => {
        buffer += chunk.toString();
      });
      socket.on("end", () => {
        const match = buffer.match(/^HTTP\/1\.1 (\d+)/);
        resolve(match ? Number(match[1]) : 0);
      });
      socket.on("error", reject);
    });
    expect(status).toBe(403);
  });

  (platform === "win32" ? it.skip : it)("returns 403 for a symlink pointing outside the root directory", async () => {
    const outsideDir = mkdtempSync(join(tmpdir(), "xberg-ui-test-outside-"));
    try {
      const outsideFile = join(outsideDir, "secret.txt");
      writeFileSync(outsideFile, "secret-data");
      const symlinkPath = join(dir, "evil-link.txt");
      symlinkSync(outsideFile, symlinkPath);

      const res = await fetch(`${baseUrl}/evil-link.txt`);
      expect(res.status).toBe(403);
    } finally {
      rmSync(outsideDir, { recursive: true, force: true });
    }
  });
});
