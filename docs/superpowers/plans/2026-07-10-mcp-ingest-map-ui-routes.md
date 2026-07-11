# MCP Ingest/Map/UI HTTP Routes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `POST /ingest`, `POST /map`, and `GET /ui/*` routes to the MCP server's existing HTTP transport, gated by a shared localhost auth token, so a future browser UI (Lot 2) can push pre-chunked/pre-embedded documents and encrypted rehydration maps straight into the MCP's own SQLite store, and serve its static build.

**Architecture:** The MCP process already runs an in-process `XbergEngine` wired to a Node-native SQLite vector store (`packages/xberg-wasm-runtime/src/store-node.ts`, reached via `getRuntime().store`). This plan adds three composable HTTP handlers (`auth`, `static-server`, `ingest-route`, `map-route`) wired together by a new `ui-server.ts`, then plugs that into the existing `transports/http.ts` server and starts it alongside the stdio transport in `index.ts`. No new store logic is needed — `store.upsertDocument()` (idempotent per `(collection, external_id)`) already does exactly what the ingest route needs.

**Tech Stack:** Node.js (ESM, `.js`-suffixed relative imports per this package's existing convention), TypeScript (`strict`, `noUncheckedIndexedAccess`), `node:http` (no framework — matches the existing minimal `transports/http.ts`), `zod` for payload validation, Vitest for tests.

## Global Constraints

- Auth is a single shared localhost token (no per-user accounts) — this is explicitly scoped to "for now," per `docs/superpowers/specs/2026-07-10-web-ui-wasm-ingestion-mcp-consumption-design.md`.
- `/ui/*` responses (and only those) must carry `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` — required for OPFS/SharedArrayBuffer in the Lot 2 browser UI.
- The rehydration map written by `/map` must land at the exact path `rehydrate_document` already reads: `path.join(getCacheDir(), "rehydration", `${document_id}.map`)` (see `mcp-server/src/tools/rehydrate.ts:81`). No new directory convention.
- `/map`'s `document_id` is the caller-chosen identifier — the same string passed as `external_id` to `/ingest` — not the store-generated UUID `upsertDocument` returns. This matches the existing `ingest_folder` tool's convention (`mcp-server/src/tools/ingest.ts:264`, which keys map files by filename base, not the returned document id).
- `upsertDocument(collection, doc, chunks)` is idempotent per `(collection, external_id)` at whole-document granularity: re-calling it for the same `external_id` deletes all prior chunks for that document and inserts the new set (`packages/xberg-wasm-runtime/src/store-node.ts:186-251`). Do not re-implement dedup in the route handler.
- All relative imports use an explicit `.js` extension even though the source file is `.ts` (existing convention throughout `mcp-server/src`).
- Every new pure-logic module must be unit-testable without booting the wasm engine; only the final wiring test (Task 6) may depend on `initializeEngine()`, and it must be added to `wasmEngineTests` in `mcp-server/vitest.config.ts` so `XBERG_SKIP_WASM_TESTS=1` CI jobs skip it.
- No `unwrap`/uncaught throws across the HTTP boundary — every handler must catch and return a JSON error body with an appropriate status code.

---

### Task 1: Auth token module

**Files:**
- Create: `mcp-server/src/http/auth.ts`
- Test: `mcp-server/tests/http-auth.test.ts`

**Interfaces:**
- Produces: `generateAuthToken(): string`, `extractToken(req: { headers: IncomingHttpHeaders }, url: URL): string | null`, `isValidToken(candidate: string | null, expected: string): boolean` — consumed by Task 5's `ui-server.ts`.

- [ ] **Step 1: Write the failing test**

```typescript
// mcp-server/tests/http-auth.test.ts
import { describe, it, expect } from "vitest";
import { generateAuthToken, extractToken, isValidToken } from "../src/http/auth.js";

describe("http/auth", () => {
  it("generateAuthToken returns a 64-char hex string", () => {
    const token = generateAuthToken();
    expect(token).toMatch(/^[0-9a-f]{64}$/);
  });

  it("extractToken reads a Bearer header over a query param", () => {
    const req = { headers: { authorization: "Bearer header-token" } };
    const url = new URL("http://localhost/ingest?token=query-token");
    expect(extractToken(req, url)).toBe("header-token");
  });

  it("extractToken falls back to the token query param", () => {
    const req = { headers: {} };
    const url = new URL("http://localhost/ui/?token=query-token");
    expect(extractToken(req, url)).toBe("query-token");
  });

  it("extractToken returns null when neither is present", () => {
    const req = { headers: {} };
    const url = new URL("http://localhost/ingest");
    expect(extractToken(req, url)).toBeNull();
  });

  it("isValidToken accepts the exact expected token", () => {
    expect(isValidToken("secret", "secret")).toBe(true);
  });

  it("isValidToken rejects a wrong or missing token", () => {
    expect(isValidToken("wrong", "secret")).toBe(false);
    expect(isValidToken(null, "secret")).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-auth.test.ts`
Expected: FAIL — `Cannot find module '../src/http/auth.js'`

- [ ] **Step 3: Write minimal implementation**

```typescript
// mcp-server/src/http/auth.ts
import { randomBytes, timingSafeEqual } from "node:crypto";
import type { IncomingHttpHeaders } from "node:http";

const BEARER_PREFIX = "Bearer ";

/** Generate a random 256-bit token, hex-encoded, for the localhost UI auth gate. */
export function generateAuthToken(): string {
  return randomBytes(32).toString("hex");
}

/**
 * Read the auth token from an `Authorization: Bearer <token>` header (used by
 * fetch calls from the UI's JS) or, failing that, from a `?token=` query
 * param (used for the initial page navigation, where custom headers aren't
 * available).
 */
export function extractToken(req: { headers: IncomingHttpHeaders }, url: URL): string | null {
  const header = req.headers.authorization;
  if (header?.startsWith(BEARER_PREFIX)) return header.slice(BEARER_PREFIX.length);
  return url.searchParams.get("token");
}

/** Constant-time comparison against the server's expected token. */
export function isValidToken(candidate: string | null, expected: string): boolean {
  if (!candidate) return false;
  const a = Buffer.from(candidate, "utf-8");
  const b = Buffer.from(expected, "utf-8");
  if (a.length !== b.length) return false;
  return timingSafeEqual(a, b);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mcp-server && npx vitest run tests/http-auth.test.ts`
Expected: PASS (6 tests)

- [ ] **Step 5: Commit**

```bash
git add mcp-server/src/http/auth.ts mcp-server/tests/http-auth.test.ts
git commit -m "feat(mcp): add localhost UI auth token module"
```

---

### Task 2: Static file server with cross-origin isolation headers

**Files:**
- Create: `mcp-server/src/http/static-server.ts`
- Test: `mcp-server/tests/http-static-server.test.ts`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `resolveSafePath(rootDir: string, requestPath: string): string | null`, `serveStaticFile(rootDir: string, requestPath: string, res: ServerResponse): void` — consumed by Task 5's `ui-server.ts`.

- [ ] **Step 1: Write the failing test**

```typescript
// mcp-server/tests/http-static-server.test.ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { createServer, type Server } from "node:http";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-static-server.test.ts`
Expected: FAIL — `Cannot find module '../src/http/static-server.js'`

- [ ] **Step 3: Write minimal implementation**

```typescript
// mcp-server/src/http/static-server.ts
import { createReadStream, existsSync, statSync } from "node:fs";
import { extname, join, normalize, resolve, sep } from "node:path";
import type { ServerResponse } from "node:http";

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

const CROSS_ORIGIN_ISOLATION_HEADERS = {
  "Cross-Origin-Opener-Policy": "same-origin",
  "Cross-Origin-Embedder-Policy": "require-corp",
};

/**
 * Resolve `requestPath` against `rootDir`, decoding percent-escapes and
 * normalizing `..` segments first, then verifying the final absolute path is
 * still inside `rootDir`. Returns `null` if the request would escape the
 * root (path traversal). The containment check at the end is the actual
 * guarantee — normalization alone is not trusted.
 */
export function resolveSafePath(rootDir: string, requestPath: string): string | null {
  let decoded: string;
  try {
    decoded = decodeURIComponent(requestPath);
  } catch {
    return null;
  }
  const cleanPath = normalize(decoded).replace(/^(\.\.[/\\])+/, "");
  const root = resolve(rootDir);
  const target = resolve(root, `.${sep}${cleanPath}`);
  if (target !== root && !target.startsWith(root + sep)) return null;
  return target;
}

/** Serve a single file from `rootDir` for `requestPath`, or a 403/404. */
export function serveStaticFile(rootDir: string, requestPath: string, res: ServerResponse): void {
  let filePath = resolveSafePath(rootDir, requestPath === "/" ? "/index.html" : requestPath);
  if (filePath === null) {
    res.writeHead(403).end("Forbidden");
    return;
  }
  if (existsSync(filePath) && statSync(filePath).isDirectory()) {
    filePath = join(filePath, "index.html");
  }
  if (!existsSync(filePath)) {
    res.writeHead(404).end("Not Found");
    return;
  }
  const contentType = CONTENT_TYPES[extname(filePath)] ?? "application/octet-stream";
  res.writeHead(200, { "Content-Type": contentType, ...CROSS_ORIGIN_ISOLATION_HEADERS });
  createReadStream(filePath).pipe(res);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mcp-server && npx vitest run tests/http-static-server.test.ts`
Expected: PASS (5 tests)

- [ ] **Step 5: Commit**

```bash
git add mcp-server/src/http/static-server.ts mcp-server/tests/http-static-server.test.ts
git commit -m "feat(mcp): add static file server with COOP/COEP headers"
```

---

### Task 3: Ingest route handler

**Files:**
- Create: `mcp-server/src/http/ingest-route.ts`
- Test: `mcp-server/tests/http-ingest-route.test.ts`

**Interfaces:**
- Consumes: `VectorStoreInterface` from `xberg-wasm-runtime` (already used throughout `mcp-server/src/tools/collection.ts`); specifically its `upsertDocument(collection: string, doc: DocumentRecord, chunks: ChunkRecord[]): Promise<string>`.
- Produces: `createIngestHandler(getStore: () => VectorStoreInterface): (req: IncomingMessage, res: ServerResponse) => Promise<void>` — consumed by Task 5's `ui-server.ts`.

- [ ] **Step 1: Write the failing test**

```typescript
// mcp-server/tests/http-ingest-route.test.ts
import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "xberg-wasm-runtime";
import { createIngestHandler } from "../src/http/ingest-route.js";

function notImplemented(name: string) {
  return async () => {
    throw new Error(`${name} not implemented in fake store`);
  };
}

function makeFakeStore(overrides: Partial<VectorStoreInterface> = {}): VectorStoreInterface {
  return {
    ensureCollection: notImplemented("ensureCollection"),
    dropCollection: notImplemented("dropCollection"),
    getCollection: notImplemented("getCollection"),
    upsertDocument: notImplemented("upsertDocument"),
    deleteDocuments: notImplemented("deleteDocuments"),
    deleteByFilter: notImplemented("deleteByFilter"),
    retrieve: notImplemented("retrieve"),
    collectionStats: notImplemented("collectionStats"),
    ...overrides,
  } as VectorStoreInterface;
}

async function withServer(
  store: VectorStoreInterface,
  fn: (baseUrl: string) => Promise<void>
): Promise<void> {
  const handler = createIngestHandler(() => store);
  const server: Server = createServer((req, res) => {
    void handler(req, res);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("expected AddressInfo");
  try {
    await fn(`http://127.0.0.1:${address.port}`);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  }
}

describe("http/ingest-route", () => {
  it("upserts a valid payload and returns the document id", async () => {
    let received: { collection: string; doc: DocumentRecord; chunks: ChunkRecord[] } | null = null;
    const store = makeFakeStore({
      upsertDocument: async (collection, doc, chunks) => {
        received = { collection, doc, chunks };
        return "doc-123";
      },
    });

    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({
          collection: "c1",
          external_id: "doc-1",
          full_text: "hello [EMAIL_1]",
          chunks: [{ ordinal: 0, content: "hello [EMAIL_1]", embedding: [0.1, 0.2, 0.3, 0.4] }],
        }),
      });
      expect(res.status).toBe(200);
      const body = (await res.json()) as { document_id: string };
      expect(body.document_id).toBe("doc-123");
    });

    expect(received).not.toBeNull();
    expect(received!.collection).toBe("c1");
    expect(received!.doc.external_id).toBe("doc-1");
    expect(received!.chunks).toHaveLength(1);
  });

  it("rejects an invalid payload with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, { method: "POST", body: JSON.stringify({ collection: "c1" }) });
      expect(res.status).toBe(400);
    });
  });

  it("maps a 'not found' store error to 404", async () => {
    const store = makeFakeStore({
      upsertDocument: async () => {
        throw new Error("collection not found: missing");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({ collection: "missing", external_id: "d", full_text: "t", chunks: [] }),
      });
      expect(res.status).toBe(404);
    });
  });

  it("maps a dimension-mismatch store error to 400", async () => {
    const store = makeFakeStore({
      upsertDocument: async () => {
        throw new Error("embedding dimension mismatch: expected 4, got 2");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/ingest`, {
        method: "POST",
        body: JSON.stringify({
          collection: "c1",
          external_id: "d",
          full_text: "t",
          chunks: [{ ordinal: 0, content: "t", embedding: [0.1, 0.2] }],
        }),
      });
      expect(res.status).toBe(400);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-ingest-route.test.ts`
Expected: FAIL — `Cannot find module '../src/http/ingest-route.js'`

- [ ] **Step 3: Write minimal implementation**

```typescript
// mcp-server/src/http/ingest-route.ts
import { z } from "zod";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord } from "xberg-wasm-runtime";

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
  chunks: z.array(ChunkPayloadSchema),
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
    const chunks: Buffer[] = [];
    for await (const chunk of req) chunks.push(chunk as Buffer);

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

    try {
      const documentId = await getStore().upsertDocument(payload.collection, doc, chunkRecords);
      res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ document_id: documentId }));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      res.writeHead(statusForError(msg), { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg }));
    }
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mcp-server && npx vitest run tests/http-ingest-route.test.ts`
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add mcp-server/src/http/ingest-route.ts mcp-server/tests/http-ingest-route.test.ts
git commit -m "feat(mcp): add POST /ingest route handler"
```

---

### Task 4: Rehydration map upload handler

**Files:**
- Create: `mcp-server/src/http/map-route.ts`
- Test: `mcp-server/tests/http-map-route.test.ts`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `createMapUploadHandler(getRehydrationDir: () => string): (req: IncomingMessage, res: ServerResponse, url: URL) => Promise<void>` — consumed by Task 5's `ui-server.ts`.

- [ ] **Step 1: Write the failing test**

```typescript
// mcp-server/tests/http-map-route.test.ts
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
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-map-route.test.ts`
Expected: FAIL — `Cannot find module '../src/http/map-route.js'`

- [ ] **Step 3: Write minimal implementation**

```typescript
// mcp-server/src/http/map-route.ts
import type { IncomingMessage, ServerResponse } from "node:http";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

// Allows plain identifiers and filename-safe punctuation only — no `/` or
// `\`, so the resulting filename can never escape `getRehydrationDir()`.
const DOCUMENT_ID_PATTERN = /^[A-Za-z0-9_.-]+$/;

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
      res
        .writeHead(400, { "Content-Type": "application/json" })
        .end(JSON.stringify({ error: "document_id query param must match [A-Za-z0-9_.-]+" }));
      return;
    }

    const chunks: Buffer[] = [];
    for await (const chunk of req) chunks.push(chunk as Buffer);
    const body = Buffer.concat(chunks);
    if (body.length === 0) {
      res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "empty body" }));
      return;
    }

    const dir = getRehydrationDir();
    mkdirSync(dir, { recursive: true });
    const mapPath = join(dir, `${documentId}.map`);
    writeFileSync(mapPath, body);

    res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ status: "stored" }));
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mcp-server && npx vitest run tests/http-map-route.test.ts`
Expected: PASS (4 tests)

- [ ] **Step 5: Commit**

```bash
git add mcp-server/src/http/map-route.ts mcp-server/tests/http-map-route.test.ts
git commit -m "feat(mcp): add POST /map rehydration upload handler"
```

---

### Task 5: Wire routes into the HTTP transport, start it alongside stdio, add the UI placeholder

**Files:**
- Create: `mcp-server/src/http/ui-server.ts`
- Create: `mcp-server/ui-dist/index.html`
- Modify: `mcp-server/src/transports/http.ts`
- Modify: `mcp-server/src/index.ts`

**Interfaces:**
- Consumes: `generateAuthToken`/`extractToken`/`isValidToken` (Task 1), `serveStaticFile` (Task 2), `createIngestHandler` (Task 3), `createMapUploadHandler` (Task 4), `getRuntime` (`mcp-server/src/engine.ts:127`), `getCacheDir` (`mcp-server/src/paths.ts:10`).
- Produces: `createUiRoutes(): { token: string; handleRequest(req, res, url): Promise<boolean> }`; `startHttp` now returns `Promise<{ port: number; uiToken: string; close(): Promise<void> }>` instead of `Promise<void>` — consumed by Task 6's integration test.

This task has no isolated unit test of its own (it is pure wiring); Task 6's integration test is the verification for this task, so implement Task 5 completely, then proceed straight to Task 6 without a separate commit in between — commit both together at the end of Task 6.

- [ ] **Step 1: Create the UI placeholder page**

```html
<!-- mcp-server/ui-dist/index.html -->
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Xberg</title>
  </head>
  <body>
    <p>Xberg UI placeholder. The real Next.js build lands here in Lot 2.</p>
  </body>
</html>
```

- [ ] **Step 2: Create `ui-server.ts`**

```typescript
// mcp-server/src/http/ui-server.ts
import type { IncomingMessage, ServerResponse } from "node:http";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { generateAuthToken, extractToken, isValidToken } from "./auth.js";
import { serveStaticFile } from "./static-server.js";
import { createIngestHandler } from "./ingest-route.js";
import { createMapUploadHandler } from "./map-route.js";
import { getRuntime } from "../engine.js";
import { getCacheDir } from "../paths.js";

// This file lives at `src/http/ui-server.ts` in dev (`tsx`) and
// `dist/http/ui-server.js` after `tsc` — both are two directories below the
// package root, so `../../` resolves to the package root in either case.
const PACKAGE_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

export interface UiRoutes {
  /** The token clients must present via `Authorization: Bearer` or `?token=`. */
  token: string;
  /** Returns `true` if this request matched a UI/ingest/map route (handled or rejected), `false` to fall through. */
  handleRequest(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean>;
}

export function createUiRoutes(): UiRoutes {
  const token = process.env["XBERG_MCP_UI_TOKEN"] ?? generateAuthToken();
  const uiDistDir = process.env["XBERG_UI_DIST_DIR"] ?? join(PACKAGE_ROOT, "ui-dist");
  const rehydrationDir = (): string => join(getCacheDir(), "rehydration");

  const ingestHandler = createIngestHandler(() => getRuntime().store);
  const mapHandler = createMapUploadHandler(rehydrationDir);

  return {
    token,
    async handleRequest(req, res, url) {
      const isUi = url.pathname === "/ui" || url.pathname.startsWith("/ui/");
      const isIngest = req.method === "POST" && url.pathname === "/ingest";
      const isMap = req.method === "POST" && url.pathname === "/map";
      if (!isUi && !isIngest && !isMap) return false;

      const candidate = extractToken(req, url);
      if (!isValidToken(candidate, token)) {
        res.writeHead(401, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "unauthorized" }));
        return true;
      }

      if (isIngest) {
        await ingestHandler(req, res);
        return true;
      }
      if (isMap) {
        await mapHandler(req, res, url);
        return true;
      }
      const subPath = url.pathname === "/ui" ? "/" : url.pathname.slice("/ui".length);
      serveStaticFile(uiDistDir, subPath, res);
      return true;
    },
  };
}
```

- [ ] **Step 3: Wire `ui-server.ts` into `transports/http.ts`**

Replace the full contents of `mcp-server/src/transports/http.ts`:

```typescript
// mcp-server/src/transports/http.ts
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { createServer, type IncomingMessage, type ServerResponse } from "node:http";
import { createUiRoutes } from "../http/ui-server.js";

const DEFAULT_PORT = Number(process.env["XBERG_MCP_PORT"] ?? 8080);
const DEFAULT_HOST = process.env["XBERG_MCP_HOST"] ?? "127.0.0.1";

export interface HttpHandle {
  port: number;
  uiToken: string;
  close(): Promise<void>;
}

export async function startHttp(
  server: McpServer,
  options: { host?: string; port?: number } = {},
): Promise<HttpHandle> {
  const host = options.host ?? DEFAULT_HOST;
  const port = options.port ?? DEFAULT_PORT;

  // SSE transport: each GET /sse opens a session; POST /message sends a message
  // Requires @modelcontextprotocol/sdk >= 1.0 with SSEServerTransport
  let SSEServerTransport: new (path: string, res: ServerResponse) => import("@modelcontextprotocol/sdk/server/sse.js").SSEServerTransport;
  try {
    const mod = await import("@modelcontextprotocol/sdk/server/sse.js");
    SSEServerTransport = mod.SSEServerTransport;
  } catch {
    process.stderr.write("[xberg-mcp] HTTP transport requires @modelcontextprotocol/sdk >= 1.0 with SSE support\n");
    throw new Error("SSE transport unavailable");
  }

  const sessions = new Map<string, InstanceType<typeof SSEServerTransport>>();
  const ui = createUiRoutes();

  const httpServer = createServer(async (req: IncomingMessage, res: ServerResponse) => {
    const url = new URL(req.url ?? "/", `http://${host}`);

    if (req.method === "GET" && url.pathname === "/sse") {
      const transport = new SSEServerTransport("/message", res);
      sessions.set(transport.sessionId, transport);
      res.on("close", () => sessions.delete(transport.sessionId));
      await server.connect(transport);
      return;
    }

    if (req.method === "POST" && url.pathname === "/message") {
      const sessionId = url.searchParams.get("sessionId") ?? "";
      const transport = sessions.get(sessionId);
      if (!transport) {
        res.writeHead(404).end("Unknown session");
        return;
      }
      const chunks: Buffer[] = [];
      for await (const chunk of req) chunks.push(chunk as Buffer);
      await transport.handlePostMessage(req, res, Buffer.concat(chunks));
      return;
    }

    if (req.method === "GET" && url.pathname === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" })
        .end(JSON.stringify({ status: "ok", server: "xberg-mcp" }));
      return;
    }

    if (await ui.handleRequest(req, res, url)) return;

    res.writeHead(404).end("Not Found");
  });

  await new Promise<void>((resolve) => httpServer.listen(port, host, resolve));
  const address = httpServer.address();
  const actualPort = address !== null && typeof address !== "string" ? address.port : port;

  process.stderr.write(`[xberg-mcp] HTTP/SSE transport started on http://${host}:${actualPort}/sse\n`);
  process.stderr.write(`[xberg-mcp] UI available at http://${host}:${actualPort}/ui?token=${ui.token}\n`);

  return {
    port: actualPort,
    uiToken: ui.token,
    close: () => new Promise<void>((resolve, reject) => httpServer.close((err) => (err ? reject(err) : resolve()))),
  };
}
```

- [ ] **Step 4: Start the HTTP transport alongside stdio in `index.ts`**

In `mcp-server/src/index.ts`, add the import and start call. Change:

```typescript
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
```

to:

```typescript
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { startHttp } from "./transports/http.js";
```

And in `main()`, change:

```typescript
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("[xberg-mcp] started");
}
```

to:

```typescript
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("[xberg-mcp] started");

  try {
    await startHttp(server);
  } catch (err) {
    console.error(`[xberg-mcp] HTTP transport failed to start (stdio still works): ${err instanceof Error ? err.message : String(err)}`);
  }
}
```

- [ ] **Step 5: Type-check**

Run: `cd mcp-server && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 6: Proceed to Task 6** (integration test covers this task; commit happens at the end of Task 6)

---

### Task 6: End-to-end integration test and CI wiring

**Files:**
- Create: `mcp-server/tests/http-ui-routes.test.ts`
- Modify: `mcp-server/vitest.config.ts`

**Interfaces:**
- Consumes: `startHttp` (Task 5, now returning `HttpHandle`), `initializeEngine`/`getRuntime` (`mcp-server/src/engine.ts`).

- [ ] **Step 1: Write the integration test**

```typescript
// mcp-server/tests/http-ui-routes.test.ts
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { initializeEngine, getRuntime } from "../src/engine.js";
import { startHttp, type HttpHandle } from "../src/transports/http.js";

const EMBEDDING_DIM = 384; // matches xberg-wasm-runtime's default embedder model

describe("HTTP ingest/map/ui routes (Task 6)", () => {
  let handle: HttpHandle;
  let baseUrl: string;
  let token: string;

  beforeAll(async () => {
    await initializeEngine();
    const { store } = getRuntime();
    await store.ensureCollection({ name: "http_ingest_test", embedding_dim: EMBEDDING_DIM });

    const server = new McpServer({ name: "test", version: "0.0.0" });
    handle = await startHttp(server, { port: 0, host: "127.0.0.1" });
    token = handle.uiToken;
    baseUrl = `http://127.0.0.1:${handle.port}`;
  }, 180_000);

  afterAll(async () => {
    await handle.close();
  });

  it("rejects /ingest without a valid token", async () => {
    const res = await fetch(`${baseUrl}/ingest`, { method: "POST", body: "{}" });
    expect(res.status).toBe(401);
  });

  it("POST /ingest stores a document via the runtime store", async () => {
    const payload = {
      collection: "http_ingest_test",
      external_id: "doc-1",
      title: "Test doc",
      full_text: "Redacted text with [EMAIL_1] token.",
      chunks: [
        { ordinal: 0, content: "Redacted text with [EMAIL_1] token.", embedding: Array(EMBEDDING_DIM).fill(0.01) },
      ],
    };
    const res = await fetch(`${baseUrl}/ingest?token=${token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { document_id: string };
    expect(typeof body.document_id).toBe("string");

    const { store } = getRuntime();
    const stats = await store.collectionStats("http_ingest_test");
    expect(stats.documents).toBeGreaterThanOrEqual(1);
  }, 60_000);

  it("POST /ingest with an unknown collection returns 404", async () => {
    const payload = { collection: "does_not_exist", external_id: "doc-x", full_text: "text", chunks: [] };
    const res = await fetch(`${baseUrl}/ingest?token=${token}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    expect(res.status).toBe(404);
  });

  it("POST /map stores an encrypted blob at the path rehydrate_document reads", async () => {
    // document_id matches the `external_id` used in the /ingest test above,
    // per this plan's convention (not the store-generated document_id).
    const blob = Buffer.from("XPII\x01fake-encrypted-bytes");
    const res = await fetch(`${baseUrl}/map?token=${token}&document_id=doc-1`, {
      method: "POST",
      body: blob,
    });
    expect(res.status).toBe(200);
  });

  it("GET /ui serves the static placeholder with cross-origin isolation headers", async () => {
    const res = await fetch(`${baseUrl}/ui/?token=${token}`);
    expect(res.status).toBe(200);
    expect(res.headers.get("cross-origin-opener-policy")).toBe("same-origin");
    expect(res.headers.get("cross-origin-embedder-policy")).toBe("require-corp");
  });

  it("GET /ui without a token is rejected", async () => {
    const res = await fetch(`${baseUrl}/ui/`);
    expect(res.status).toBe(401);
  });
});
```

- [ ] **Step 2: Add the new suite to the wasm-engine skip list**

In `mcp-server/vitest.config.ts`, add `"tests/http-ui-routes.test.ts"` to the `wasmEngineTests` array:

```typescript
const wasmEngineTests = [
  "tests/collections.test.ts",
  "tests/e2e.test.ts",
  "tests/engine.test.ts",
  "tests/http-ui-routes.test.ts",
  "tests/ingest.test.ts",
  "tests/pii.test.ts",
  "tests/pii_parity.test.ts",
  "tests/query.test.ts",
  "tests/rehydration_compat.test.ts",
  "tests/tools.test.ts",
];
```

- [ ] **Step 3: Run the full new suite to verify it fails first (before Task 5's changes, this file would 404 on `/ingest`/`/map`/`/ui`; run now to confirm it passes with Task 5's wiring in place)**

Run: `cd mcp-server && npx vitest run tests/http-ui-routes.test.ts`
Expected: PASS (6 tests) — this requires the wasm engine binary; if it's unavailable locally, this step cannot be verified in this environment and must be run where `crates/xberg-wasm/pkg/nodejs/xberg_wasm_bg.wasm` is built.

- [ ] **Step 4: Run the full mcp-server suite to check nothing else broke**

Run: `cd mcp-server && npx vitest run`
Expected: PASS (all suites, or only pre-existing unrelated failures)

- [ ] **Step 5: Commit Task 5 and Task 6 together**

```bash
git add mcp-server/src/http/ui-server.ts mcp-server/ui-dist/index.html \
        mcp-server/src/transports/http.ts mcp-server/src/index.ts \
        mcp-server/tests/http-ui-routes.test.ts mcp-server/vitest.config.ts
git commit -m "feat(mcp): serve /ui, /ingest, /map behind a localhost auth token"
```

---

## Self-Review Notes

- **Spec coverage:** `/ingest` (Task 3+5), `/map` (Task 4+5), static `/ui` serving with COOP/COEP (Task 2+5), localhost token auth (Task 1+5), idempotent upsert reuse (documented in Global Constraints, no new logic), HTTP transport actually started (Task 5 Step 4 — previously dead code). Next.js/shadcn/extend-hq UI itself is explicitly Lot 2, not this plan.
- **Type consistency:** `HttpHandle` (Task 5) is the same shape used by Task 6's test import. `createIngestHandler`'s `getStore: () => VectorStoreInterface` matches `() => getRuntime().store` in `ui-server.ts`. `createMapUploadHandler`'s `getRehydrationDir: () => string` matches `rehydrationDir` in `ui-server.ts`.
- **No placeholders:** the UI placeholder page is real, minimal, testable content — not a TODO; it is explicitly documented as being replaced by Lot 2's Next.js export.
