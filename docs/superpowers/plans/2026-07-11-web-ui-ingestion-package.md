# Web UI Ingestion Package Implementation Plan (Lot 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `packages/xberg-web-ui/` — a Next.js static-export browser app that lets a user create folders (MCP collections), upload documents, run the full extract→OCR→NER→PII-redact→chunk→embed pipeline **entirely client-side** via the WASM `XbergEngine`, and auto-sync the redacted result + encrypted rehydration map to the already-implemented MCP HTTP server (`POST /ingest`, `POST /map` from Lot 1). Adds one new MCP route (`POST /collection`) since no existing route can create the collection a folder maps to.

**Architecture:** One Web Worker (`engine.worker.ts`) owns the `XbergEngine` instance and a **custom `VectorStoreInterface`** whose `upsertDocument` is an HTTP POST to `/ingest` instead of a local OPFS/SQLite write — this lets `engine.ingest()` run unmodified (it already does PII+NER+chunk+embed+upsert as one call) while redirecting only the storage write over HTTP. The worker also calls `engine.encrypt_map()` and POSTs the encrypted blob to `/map`, reusing the browser-chosen `external_id` as `/map`'s `document_id` (Lot 1's contract: they must be the same string). A `worker-client.ts` wraps `postMessage` RPC; an `EngineProvider`/`useEngine()` React context exposes it to screens. Folder/document metadata (not the plaintext rehydration map — that lives only in memory) is tracked in a local IndexedDB registry (`ingest-history.ts`), since there is no server-side "list documents"/"list collections" route to query instead.

**Tech Stack:** Next.js 14 App Router with `output: 'export'` (static export, no SSR), React 18, TypeScript strict + `noUncheckedIndexedAccess`, Tailwind CSS, shadcn/ui primitives (button, dialog, input, table, card, badge — installed via `npx shadcn@latest add`), TanStack Table (document lists), `xberg-wasm-runtime` + `@xberg-io/xberg-wasm` (file-linked workspace packages), native `indexedDB` (no new dependency), Vitest + Testing Library (unit/component), Playwright (e2e). MCP side: `node:http` + `zod`, Vitest — same stack as Lot 1/Lot 3.

## Global Constraints

- **`external_id` = `/map`'s `document_id`, chosen client-side.** The browser derives a stable `external_id` per file (sanitized filename) and reuses that exact string as both the `external_id` field in the `/ingest` JSON body and the `document_id` query param on `/map` (`mcp-server/src/http/ingest-route.ts` doc comment, `mcp-server/src/http/map-route.ts:5-7`). It must match `map-route.ts`'s `DOCUMENT_ID_PATTERN = /^[A-Za-z0-9_.-]+$/` — filenames are sanitized (non-matching characters replaced with `_`) before use.
- **Collections don't auto-create.** `store.upsertDocument` throws `"collection not found: <name>"` if the collection is missing (`packages/xberg-wasm-runtime/src/store-node.ts:130-134`) — there is no existing HTTP route to create one, so this plan adds `POST /collection` (Task 1) and folder creation calls it before any upload into that folder.
- **`engine.ingest()` is the one call that does PII+NER+chunk+embed+upsert.** Per `crates/xberg-wasm/src/engine.rs:156-231`, `XbergEngine.ingest(doc, collection, config?)` internally redacts PII, runs NER, chunks, embeds (via the injected `embedder`), and calls the injected `store.upsertDocument`. This plan does **not** hand-roll chunk/embed calls — it constructs the engine via `createXbergRuntimeFactory()` (which supplies `embedder`/`ner`/`ocr` from `xberg-wasm-runtime`) and replaces only the `store` in that descriptor with an HTTP-backed shim.
- **`doc.external_id` must be passed explicitly to `engine.ingest()`.** The reference Node implementation (`packages/xberg-wasm-runtime/src/ingest-folder.ts`) omits it; this plan cannot, because `/ingest`'s `external_id` is required and must match `/map`'s `document_id`.
- **Rehydration map naming.** `engine.redact()` returns `{ redacted, rehydrationMap }` (camelCase) but `engine.ingest()` returns `{ document_id, rehydration_map, pii_category_counts }` (snake_case) — this plan only uses the `ingest()` return shape; do not conflate the two.
- **Wire-format compatibility is already guaranteed.** `engine.encrypt_map(map, passphrase): Uint8Array` produces the exact `XPII\x01` + 16-byte salt + 12-byte nonce + 16-byte GCM tag + ciphertext format that `mcp-server/src/redaction/rehydration.ts` and the MCP's `POST /map` write verbatim (`crates/xberg/src/text/redaction/rehydration.rs:1-20`) — no format translation needed in this plan.
- **`/ingest` limits:** 10 MiB body, 10,000 chunks max (`mcp-server/src/http/ingest-route.ts:6-7`). **`/map` limits:** 16 MiB body (`mcp-server/src/http/map-route.ts:12`). The controller surfaces a clear error if a file's chunk set would exceed these rather than silently truncating.
- **No plaintext PII persists in the browser.** The rehydration map returned by `engine.ingest()` is held only in a JS variable for the duration of encrypt+POST /map, then discarded — never written to IndexedDB/OPFS/localStorage. Only redacted text, category counts, and non-PII metadata are persisted locally (mirrors the `pii-pipeline` rule that passphrases/plaintext never linger).
- **No server-side collection/document listing exists.** Folder and document lists are sourced from the local IndexedDB registry (`ingest-history.ts`), not fetched from the MCP — same reasoning Lot 3 already applies to its document table (`docs/superpowers/plans/2026-07-10-web-ui-advanced-viz-delete.md:50`).
- **Embedding dimension is a fixed constant.** `xberg-wasm-runtime`'s default embedder is `Xenova/bge-m3` (1024-dim). `POST /collection` and the worker both use `EMBEDDING_DIM = 1024` from `src/lib/constants.ts`; the worker asserts the actual embed output length matches this constant on first use and throws a clear error otherwise (catches a model swap early instead of failing opaquely deep inside `engine.ingest()`).
- **Auth token comes from the URL.** The MCP prints `http://host:port/ui?token=<token>` on startup (`mcp-server/src/transports/http.ts:75`). The app reads `?token=` on first load, keeps it in memory for the session (module-level singleton, not localStorage — a leaked localStorage token outlives the server process), and attaches it as `Authorization: Bearer <token>` on every POST plus `?token=` on the initial page load per Lot 1's `extractToken` (`mcp-server/src/http/auth.ts:17-21`).
- **Sequential ingestion only.** `XbergEngine` is not verified reentrant across concurrent calls (`packages/xberg-wasm-runtime/src/ingest-folder.ts` doc comment) — the worker processes one file at a time even when multiple uploads are queued; never `Promise.all` over files.
- **Chrome/Edge only, no SSR.** Matches the design spec (`docs/superpowers/specs/2026-07-10-web-ui-wasm-ingestion-mcp-consumption-design.md:124,128`) — OPFS + cross-origin isolation headers (already served by Lot 1's `static-server.ts` for all `/ui/*` responses) require it. `next.config.js` uses `output: 'export'`; no API routes.
- **This is Lot 2, not Lot 3.** Document viewers (PDF/DOCX/XLSX), OCR layout blocks, and PII bounding-box review are explicitly out of scope — `DocumentViewer.tsx` and `DocumentTable.tsx` are built here with basic shadcn/TanStack primitives and are documented as **modified by Lot 3** (`docs/superpowers/plans/2026-07-10-web-ui-advanced-viz-delete.md:56,64,80`) — this plan must not invent conflicting names for those two files.

---

## File Structure

```
mcp-server/src/http/collection-route.ts          # NEW: createCollectionHandler(getStore)
mcp-server/src/http/ui-server.ts                 # MODIFY (Lot 1): add /collection branch
mcp-server/tests/http-collection-route.test.ts   # NEW
mcp-server/tests/http-ui-routes.test.ts          # MODIFY (Lot 1): add /collection assertion

packages/xberg-web-ui/
  package.json  tsconfig.json  next.config.js  tailwind.config.ts  postcss.config.js
  components.json                                # shadcn config
  vitest.config.ts  playwright.config.ts
  scripts/export-to-mcp.mjs                      # NEW: copies `out/` -> mcp-server/ui-dist/
  src/lib/constants.ts                           # NEW: EMBEDDING_DIM, DOCUMENT_ID_PATTERN
  src/lib/auth-client.ts                          # NEW: token capture + authed URL/headers
  src/lib/types.ts                                # NEW: IngestPayload, IngestHistoryEntry, etc.
  src/lib/sync-client.ts                          # NEW: postCollection/postIngest/postMap (retry+backoff)
  src/lib/ingest-history.ts                       # NEW: IndexedDB-backed local registry
  src/lib/sanitize-id.ts                          # NEW: filename -> external_id sanitizer
  src/engine/engine.worker.ts                     # NEW: owns XbergEngine + HTTP-backed store shim
  src/engine/worker-client.ts                     # NEW: WorkerClient (postMessage RPC wrapper)
  src/providers/EngineProvider.tsx                # NEW: React context wiring worker-client
  src/components/ui/                              # shadcn primitives (button, dialog, input, table, card, badge)
  src/components/CreateFolderDialog.tsx           # NEW
  src/components/SyncBar.tsx                      # NEW: global sync status bar
  src/components/DocumentTable.tsx                # NEW (Lot 3 modifies: adds selection)
  src/components/DocumentViewer.tsx               # NEW (Lot 3 modifies: swaps in extend viewers)
  src/app/layout.tsx                              # NEW: mounts EngineProvider + SyncBar
  src/app/page.tsx                                # NEW: folder list (Écran 1)
  src/app/folder/[collection]/page.tsx            # NEW: upload + document list (Écran 2)
  src/app/document/[collection]/[id]/page.tsx     # NEW: document status view (Écran 3, Lot 3 modifies)
  tests/sanitize-id.test.ts                       # NEW
  tests/sync-client.test.ts                       # NEW
  tests/ingest-history.test.ts                    # NEW
  tests/worker-client.test.ts                     # NEW
  tests/components/CreateFolderDialog.test.tsx    # NEW
  tests/components/SyncBar.test.tsx               # NEW
  e2e/ingest.spec.ts                              # NEW
```

---

### Task 1: MCP `POST /collection` route + wire into `ui-server.ts`

**Files:**
- Create: `mcp-server/src/http/collection-route.ts`
- Create: `mcp-server/tests/http-collection-route.test.ts`
- Modify: `mcp-server/src/http/ui-server.ts`
- Modify: `mcp-server/tests/http-ui-routes.test.ts`

**Interfaces:**
- Consumes: `VectorStoreInterface.ensureCollection(spec: CollectionSpec): Promise<string | void>` (`xberg-wasm-runtime`).
- Produces: `createCollectionHandler(getStore: () => VectorStoreInterface): (req: IncomingMessage, res: ServerResponse) => Promise<void>` — consumed by `ui-server.ts`; token gating stays in `ui-server.ts`, same as `/ingest`/`/map`.

- [ ] **Step 1: Write the failing test**

```typescript
// mcp-server/tests/http-collection-route.test.ts
import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface } from "xberg-wasm-runtime";
import { createCollectionHandler } from "../src/http/collection-route.js";

function notImplemented(name: string) {
  return async () => {
    throw new Error(`${name} not implemented in fake store`);
  };
}
function makeFakeStore(overrides: Partial<VectorStoreInterface> = {}): VectorStoreInterface {
  return {
    close: notImplemented("close"),
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
async function withServer(store: VectorStoreInterface, fn: (baseUrl: string) => Promise<void>): Promise<void> {
  const handler = createCollectionHandler(() => store);
  const server: Server = createServer((req, res) => {
    void handler(req, res);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("expected an AddressInfo");
  try {
    await fn(`http://127.0.0.1:${address.port}`);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((err) => (err ? reject(err) : resolve())));
  }
}

describe("http/collection-route", () => {
  it("ensures a collection and returns { created: true }", async () => {
    let received: unknown = null;
    const store = makeFakeStore({
      ensureCollection: async (spec) => {
        received = spec;
        return "ok";
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "dossier-1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ created: true });
    });
    expect(received).toEqual({ name: "dossier-1", embedding_dim: 1024 });
  });

  it("rejects an invalid payload with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "" }),
      });
      expect(res.status).toBe(400);
    });
  });

  it("rejects invalid JSON with 400", async () => {
    const store = makeFakeStore();
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, { method: "POST", body: "{not json" });
      expect(res.status).toBe(400);
    });
  });

  it("maps a store error (thrown) to 400", async () => {
    const store = makeFakeStore({
      ensureCollection: async () => {
        throw new Error("invalid distance metric");
      },
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "c1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(400);
    });
  });

  it("maps a store error (returned as a string, per ensureCollection's real contract) to 400", async () => {
    // `ensureCollection` reports failure by *resolving* with an error
    // string, not by throwing — this is the actual behavior documented on
    // `VectorStoreInterface.ensureCollection` in `xberg-wasm-runtime`, and
    // is a distinct failure mode from the thrown-error test above.
    const store = makeFakeStore({
      ensureCollection: async () => "dimension mismatch: collection exists with dim 512",
    });
    await withServer(store, async (baseUrl) => {
      const res = await fetch(`${baseUrl}/collection`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: "c1", embedding_dim: 1024 }),
      });
      expect(res.status).toBe(400);
      expect((await res.json()).error).toContain("dimension mismatch");
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-collection-route.test.ts`
Expected: FAIL — `Cannot find module '../src/http/collection-route.js'`

- [ ] **Step 3: Write minimal implementation**

```typescript
// mcp-server/src/http/collection-route.ts
import { z } from "zod";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { VectorStoreInterface } from "xberg-wasm-runtime";

const MAX_BODY_BYTES = 64 * 1024;

/**
 * `POST /collection` payload. `embedding_dim` is caller-supplied (not
 * inferred server-side) because the browser's embedder model — and
 * therefore its output dimension — is chosen client-side.
 */
const CollectionPayloadSchema = z.object({
  name: z.string().min(1),
  embedding_dim: z.number().int().positive(),
  // Values must match `xberg-wasm-runtime`'s real `DistanceMetric`/`IndexMethod`
  // union types verbatim (`packages/xberg-wasm-runtime/src/types.ts`) — not
  // guessed synonyms; `ensureCollection` rejects anything else.
  distance_metric: z.enum(["cosine", "l2", "innerproduct"]).optional(),
  index_method: z.enum(["flat", "hnsw", "diskann"]).optional(),
});

function statusForError(message: string): number {
  return message.includes("not found") ? 404 : 400;
}

/**
 * Build the `POST /collection` handler. Idempotent: calling it again with
 * the same `name` is a no-op on the store side (`ensureCollection`'s own
 * contract), so the browser can call this unconditionally on folder open,
 * not just folder creation.
 */
export function createCollectionHandler(
  getStore: () => VectorStoreInterface
): (req: IncomingMessage, res: ServerResponse) => Promise<void> {
  return async function handleCollection(req: IncomingMessage, res: ServerResponse): Promise<void> {
    try {
      const chunks: Buffer[] = [];
      let totalBytes = 0;
      for await (const chunk of req) {
        totalBytes += (chunk as Buffer).length;
        if (totalBytes > MAX_BODY_BYTES) {
          res.writeHead(413, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "payload too large" }));
          return;
        }
        chunks.push(chunk as Buffer);
      }

      let json: unknown;
      try {
        json = JSON.parse(Buffer.concat(chunks).toString("utf-8"));
      } catch {
        res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid JSON body" }));
        return;
      }

      const parsed = CollectionPayloadSchema.safeParse(json);
      if (!parsed.success) {
        res
          .writeHead(400, { "Content-Type": "application/json" })
          .end(JSON.stringify({ error: "invalid payload", issues: parsed.error.issues }));
        return;
      }

      // `ensureCollection` reports failure by *resolving* with an error
      // string (never throwing) — the same convention already handled by
      // the existing `create_collection` MCP tool
      // (`mcp-server/src/tools/collection.ts`). A thrown error is also
      // possible (e.g. an unexpected store fault) and is caught below.
      const result = await getStore().ensureCollection(parsed.data);
      if (typeof result === "string") {
        res.writeHead(statusForError(result), { "Content-Type": "application/json" }).end(JSON.stringify({ error: result }));
        return;
      }
      res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ created: true }));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (!res.headersSent) {
        res.writeHead(statusForError(msg), { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg }));
      } else {
        res.end();
      }
    }
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd mcp-server && npx vitest run tests/http-collection-route.test.ts`
Expected: PASS (4 tests)

- [ ] **Step 5: Wire `/collection` into `ui-server.ts`**

In `mcp-server/src/http/ui-server.ts`, add the import:

```typescript
import { createCollectionHandler } from "./collection-route.js";
```

Inside `createUiRoutes()`, alongside the existing handler constructions:

```typescript
const collectionHandler = createCollectionHandler(() => getRuntime().store);
```

Inside `handleRequest`, add an `isCollection` branch next to `isIngest`/`isMap` (same auth-gated block):

```typescript
const isCollection = req.method === "POST" && url.pathname === "/collection";
if (!isUi && !isIngest && !isMap && !isCollection) return false;
```

and, after the `isMap` branch:

```typescript
if (isCollection) {
  await collectionHandler(req, res);
  return true;
}
```

- [ ] **Step 6: Extend the Lot 1 integration test**

In `mcp-server/tests/http-ui-routes.test.ts`, add a test before the `POST /ingest stores a document` test (collections must exist before ingest):

```typescript
it("POST /collection creates a collection ingest can then target", async () => {
  const res = await fetch(`${baseUrl}/collection?token=${token}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ name: "http_collection_route_test", embedding_dim: EMBEDDING_DIM }),
  });
  expect(res.status).toBe(200);
  expect(await res.json()).toEqual({ created: true });
});
```

- [ ] **Step 7: Run the full mcp-server suite**

Run: `cd mcp-server && XBERG_SKIP_WASM_TESTS=1 npx vitest run`
Expected: PASS (all suites, including the 4 new `http-collection-route.test.ts` tests)

- [ ] **Step 8: Commit**

```bash
git add mcp-server/src/http/collection-route.ts mcp-server/tests/http-collection-route.test.ts \
        mcp-server/src/http/ui-server.ts mcp-server/tests/http-ui-routes.test.ts
git commit -m "feat(mcp): add POST /collection route for folder creation"
```

---

### Task 2: `packages/xberg-web-ui` scaffold (Next.js static export, Tailwind, shadcn, Vitest)

**Files:**
- Create: `packages/xberg-web-ui/package.json`
- Create: `packages/xberg-web-ui/tsconfig.json`
- Create: `packages/xberg-web-ui/next.config.js`
- Create: `packages/xberg-web-ui/tailwind.config.ts`
- Create: `packages/xberg-web-ui/postcss.config.js`
- Create: `packages/xberg-web-ui/components.json`
- Create: `packages/xberg-web-ui/vitest.config.ts`
- Create: `packages/xberg-web-ui/playwright.config.ts`
- Create: `packages/xberg-web-ui/src/app/layout.tsx`
- Create: `packages/xberg-web-ui/src/app/page.tsx`
- Create: `packages/xberg-web-ui/src/app/globals.css`
- Create: `packages/xberg-web-ui/tests/smoke.test.tsx`

**Interfaces:**
- Consumes: nothing from earlier tasks (Task 1 is MCP-side, independent).
- Produces: a buildable, testable Next.js package other tasks add files into. `pnpm --filter xberg-web-ui typecheck/test/build/export` all work by the end of this task.

- [ ] **Step 1: `package.json`**

```json
{
  "name": "xberg-web-ui",
  "version": "1.0.0-rc.1",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "export": "next build && node scripts/export-to-mcp.mjs",
    "typecheck": "tsc --noEmit",
    "test": "vitest",
    "test:run": "vitest run",
    "test:e2e": "playwright test",
    "lint": "oxlint src/"
  },
  "dependencies": {
    "next": "^14.2.0",
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "@tanstack/react-table": "^8.20.0",
    "clsx": "^2.1.1",
    "class-variance-authority": "^0.7.0",
    "tailwind-merge": "^2.5.0",
    "xberg-wasm-runtime": "file:../xberg-wasm-runtime",
    "@xberg-io/xberg-wasm": "file:../../crates/xberg-wasm"
  },
  "devDependencies": {
    "@playwright/test": "^1.61.1",
    "@testing-library/jest-dom": "^6.5.0",
    "@testing-library/react": "^16.0.1",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react-oxc": "^0.4.3",
    "autoprefixer": "^10.4.20",
    "jsdom": "^25.0.1",
    "oxlint": "^1.73.0",
    "postcss": "^8.4.47",
    "tailwindcss": "^3.4.13",
    "typescript": "^6.0.3",
    "vitest": "^4.1.9"
  }
}
```

- [ ] **Step 2: `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "dom.iterable", "esnext", "webworker"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "jsx": "preserve",
    "esModuleInterop": true,
    "skipLibCheck": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "incremental": true,
    "noEmit": true,
    "paths": { "@/*": ["./src/*"] }
  },
  "include": ["src", "tests", "next-env.d.ts"],
  "exclude": ["node_modules", ".next", "out"]
}
```

- [ ] **Step 3: `next.config.js` (static export, cross-origin isolation headers only matter at serve time — Lot 1's `static-server.ts` already adds COOP/COEP, no config needed here)**

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  reactStrictMode: true,
  images: { unoptimized: true },
};

export default nextConfig;
```

- [ ] **Step 4: `tailwind.config.ts` + `postcss.config.js`**

```typescript
// tailwind.config.ts
import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
};
export default config;
```

```javascript
// postcss.config.js
export default {
  plugins: { tailwindcss: {}, autoprefixer: {} },
};
```

- [ ] **Step 5: `components.json` (shadcn CLI config, used by later tasks)**

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "tailwind.config.ts",
    "css": "src/app/globals.css",
    "baseColor": "slate",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui"
  }
}
```

- [ ] **Step 6: `vitest.config.ts` + `playwright.config.ts`**

```typescript
// vitest.config.ts
import { defineConfig } from "vitest/config";
// This workspace pins `vite@8.1.3` (rolldown/oxc-based); the babel-based
// `@vitejs/plugin-react` fails to parse JSX against it — use the oxc-native
// equivalent instead (same API).
import react from "@vitejs/plugin-react-oxc";

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./tests/setup.ts"],
    include: ["tests/**/*.test.{ts,tsx}"],
    exclude: ["e2e/**", "node_modules/**", ".next/**", "out/**"],
  },
  resolve: { alias: { "@": new URL("./src", import.meta.url).pathname } },
});
```

```typescript
// tests/setup.ts
import "@testing-library/jest-dom/vitest";
```

```typescript
// playwright.config.ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  use: { headless: true },
});
```

- [ ] **Step 7: minimal app shell + smoke test**

```css
/* src/app/globals.css */
@tailwind base;
@tailwind components;
@tailwind utilities;
```

```tsx
// src/app/layout.tsx
import "./globals.css";
import type { ReactNode } from "react";

export const metadata = { title: "Xberg" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
```

```tsx
// src/app/page.tsx
export default function HomePage() {
  return <main className="p-6">Xberg — folders</main>;
}
```

```tsx
// tests/smoke.test.tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import HomePage from "../src/app/page.js";

describe("app shell smoke test", () => {
  it("renders the placeholder home page", () => {
    render(<HomePage />);
    expect(screen.getByText("Xberg — folders")).toBeDefined();
  });
});
```

- [ ] **Step 8: Install and verify**

Run: `pnpm install && cd packages/xberg-web-ui && npx vitest run tests/smoke.test.tsx && npx tsc --noEmit`
Expected: PASS; no type errors

- [ ] **Step 9: Commit**

```bash
git add packages/xberg-web-ui/
git commit -m "feat(web-ui): scaffold packages/xberg-web-ui (Next.js static export)"
```

---

### Task 3: Shared types, constants, `sanitize-id`, `auth-client`

**Files:**
- Create: `packages/xberg-web-ui/src/lib/constants.ts`
- Create: `packages/xberg-web-ui/src/lib/types.ts`
- Create: `packages/xberg-web-ui/src/lib/sanitize-id.ts`
- Create: `packages/xberg-web-ui/tests/sanitize-id.test.ts`
- Create: `packages/xberg-web-ui/src/lib/auth-client.ts`
- Create: `packages/xberg-web-ui/tests/auth-client.test.ts`

**Interfaces:**
- Produces: `EMBEDDING_DIM: number`, `DOCUMENT_ID_PATTERN: RegExp`; `IngestPayload`, `IngestHistoryEntry`, `SyncStatus` types; `sanitizeExternalId(filename: string): string`; `setAuthToken`/`getAuthToken`/`authedUrl`/`authHeaders` — all consumed by Tasks 4-8.

- [ ] **Step 1: `constants.ts`**

```typescript
// src/lib/constants.ts

/**
 * `xberg-wasm-runtime`'s default embedder is `Xenova/bge-m3` (1024-dim,
 * L2-normalized). Collections are created with this fixed dimension; the
 * worker asserts the actual embed output matches it (Task 6) so a model
 * swap fails loudly instead of deep inside `engine.ingest()`.
 */
export const EMBEDDING_DIM = 1024;

/** Must match `mcp-server/src/http/map-route.ts`'s `DOCUMENT_ID_PATTERN`. */
export const DOCUMENT_ID_PATTERN = /^[A-Za-z0-9_.-]+$/;

export const INGEST_MAX_BODY_BYTES = 10 * 1024 * 1024;
export const MAP_MAX_BODY_BYTES = 16 * 1024 * 1024;
```

- [ ] **Step 2: `sanitize-id.ts` + failing test**

```typescript
// tests/sanitize-id.test.ts
import { describe, it, expect } from "vitest";
import { sanitizeExternalId } from "../src/lib/sanitize-id.js";
import { DOCUMENT_ID_PATTERN } from "../src/lib/constants.js";

describe("lib/sanitize-id", () => {
  it("leaves an already-safe filename untouched", () => {
    expect(sanitizeExternalId("contrat-2026.pdf")).toBe("contrat-2026.pdf");
  });

  it("replaces unsafe characters with underscores", () => {
    const result = sanitizeExternalId("contrat client (v2)/résumé.pdf");
    expect(DOCUMENT_ID_PATTERN.test(result)).toBe(true);
  });

  it("collapses to a fallback when the whole name is unsafe", () => {
    const result = sanitizeExternalId("////");
    expect(result.length).toBeGreaterThan(0);
    expect(DOCUMENT_ID_PATTERN.test(result)).toBe(true);
  });

  it("is deterministic for the same input", () => {
    expect(sanitizeExternalId("a b.pdf")).toBe(sanitizeExternalId("a b.pdf"));
  });
});
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/sanitize-id.test.ts`
Expected: FAIL — module not found

```typescript
// src/lib/sanitize-id.ts
import { DOCUMENT_ID_PATTERN } from "./constants.js";

/**
 * Derive a `/map`-safe `document_id`/`external_id` from a user-supplied
 * filename. Deterministic (same input -> same output) so re-uploading the
 * same file idempotently replaces it, per `upsertDocument`'s contract.
 */
export function sanitizeExternalId(filename: string): string {
  const replaced = filename.replace(/[^A-Za-z0-9_.-]/g, "_");
  const trimmed = replaced.replace(/^_+|_+$/g, "");
  const safe = trimmed.length > 0 ? trimmed : "file";
  return DOCUMENT_ID_PATTERN.test(safe) ? safe : "file";
}
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/sanitize-id.test.ts`
Expected: PASS (4 tests)

- [ ] **Step 3: `types.ts`**

```typescript
// src/lib/types.ts

export interface IngestChunkPayload {
  ordinal: number;
  content: string;
  embedding: number[];
  chunk_metadata?: unknown;
}

/** Mirrors `mcp-server/src/http/ingest-route.ts`'s `IngestPayloadSchema`. */
export interface IngestPayload {
  collection: string;
  external_id: string;
  title?: string;
  mime?: string;
  source_uri?: string;
  full_text: string;
  keywords?: string[];
  metadata?: Record<string, unknown>;
  chunks: IngestChunkPayload[];
}

export interface CollectionPayload {
  name: string;
  embedding_dim: number;
  distance_metric?: "cosine" | "l2" | "innerproduct";
  index_method?: "flat" | "hnsw" | "diskann";
}

export type SyncStatus = "pending" | "syncing" | "synced" | "error";

/**
 * Local (IndexedDB) record of an ingested document. Never contains the
 * plaintext rehydration map — only redacted text and counts.
 */
export interface IngestHistoryEntry {
  collection: string;
  externalId: string;
  filename: string;
  mime: string;
  redactedText: string;
  piiCategoryCounts: Record<string, number>;
  documentId: string | null;
  status: SyncStatus;
  error?: string;
  ingestedAt: number;
}
```

- [ ] **Step 4: `auth-client.ts` + failing test**

```typescript
// tests/auth-client.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { setAuthToken, getAuthToken, authedUrl, authHeaders } from "../src/lib/auth-client.js";

describe("lib/auth-client", () => {
  beforeEach(() => setAuthToken(null));

  it("returns null before a token is set", () => {
    expect(getAuthToken()).toBeNull();
  });

  it("stores and returns the token", () => {
    setAuthToken("abc123");
    expect(getAuthToken()).toBe("abc123");
  });

  it("authedUrl appends ?token= to a bare path", () => {
    setAuthToken("abc123");
    expect(authedUrl("http://x:8080", "/ingest")).toBe("http://x:8080/ingest?token=abc123");
  });

  it("authedUrl preserves existing query params", () => {
    setAuthToken("abc123");
    expect(authedUrl("http://x:8080", "/map?document_id=doc-1")).toBe(
      "http://x:8080/map?document_id=doc-1&token=abc123"
    );
  });

  it("authHeaders includes a Bearer header", () => {
    setAuthToken("abc123");
    expect(authHeaders()).toEqual({ Authorization: "Bearer abc123" });
  });

  it("authedUrl throws without a token", () => {
    expect(() => authedUrl("http://x:8080", "/ingest")).toThrow();
  });
});
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/auth-client.test.ts`
Expected: FAIL — module not found

```typescript
// src/lib/auth-client.ts

/**
 * In-memory only (not localStorage) — the token is only valid while the
 * MCP process that printed it is alive; persisting it across restarts
 * would let the UI silently use a stale/invalid token.
 */
let token: string | null = null;

export function setAuthToken(value: string | null): void {
  token = value;
}

export function getAuthToken(): string | null {
  return token;
}

/** Reads `?token=` from the current page URL and stores it, if present. */
export function captureAuthTokenFromLocation(): void {
  if (typeof window === "undefined") return;
  const fromUrl = new URL(window.location.href).searchParams.get("token");
  if (fromUrl) setAuthToken(fromUrl);
}

export function authedUrl(baseUrl: string, path: string): string {
  if (!token) throw new Error("auth token not set — call captureAuthTokenFromLocation() first");
  const url = new URL(path, baseUrl);
  url.searchParams.set("token", token);
  return url.toString();
}

export function authHeaders(): Record<string, string> {
  if (!token) throw new Error("auth token not set — call captureAuthTokenFromLocation() first");
  return { Authorization: `Bearer ${token}` };
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd packages/xberg-web-ui && npx vitest run tests/sanitize-id.test.ts tests/auth-client.test.ts && npx tsc --noEmit`
Expected: PASS (10 tests); no type errors

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-web-ui/src/lib/constants.ts packages/xberg-web-ui/src/lib/types.ts \
        packages/xberg-web-ui/src/lib/sanitize-id.ts packages/xberg-web-ui/tests/sanitize-id.test.ts \
        packages/xberg-web-ui/src/lib/auth-client.ts packages/xberg-web-ui/tests/auth-client.test.ts
git commit -m "feat(web-ui): shared types, constants, id sanitizer, auth client"
```

---

### Task 4: `sync-client.ts` — `postCollection`/`postIngest`/`postMap` with retry+backoff

**Files:**
- Create: `packages/xberg-web-ui/src/lib/sync-client.ts`
- Create: `packages/xberg-web-ui/tests/sync-client.test.ts`

**Interfaces:**
- Consumes: `authedUrl`, `authHeaders` (Task 3); `IngestPayload`, `CollectionPayload` (Task 3).
- Produces: `postCollection(baseUrl, payload: CollectionPayload): Promise<void>`, `postIngest(baseUrl, payload: IngestPayload): Promise<{ document_id: string }>`, `postMap(baseUrl, documentId: string, blob: Uint8Array): Promise<void>` — consumed by Task 6's HTTP-backed store shim.

- [ ] **Step 1: Write the failing test**

```typescript
// tests/sync-client.test.ts
import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { postCollection, postIngest, postMap } from "../src/lib/sync-client.js";
import { setAuthToken } from "../src/lib/auth-client.js";

function jsonResponse(status: number, body: unknown): Response {
  return { status, ok: status < 300, json: async () => body } as Response;
}

describe("lib/sync-client", () => {
  beforeEach(() => setAuthToken("tok"));
  afterEach(() => vi.unstubAllGlobals());

  it("postCollection posts to /collection with the token", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { created: true }));
    vi.stubGlobal("fetch", fetchMock);
    await postCollection("http://x:8080", { name: "c1", embedding_dim: 1024 });
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x:8080/collection?token=tok");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body as string)).toEqual({ name: "c1", embedding_dim: 1024 });
  });

  it("postIngest posts JSON and returns the document_id", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postIngest("http://x:8080", {
      collection: "c1",
      external_id: "doc-1",
      full_text: "hello",
      chunks: [],
    });
    expect(result).toEqual({ document_id: "doc-1" });
  });

  it("postMap posts the raw blob with document_id in the query string", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { status: "stored" }));
    vi.stubGlobal("fetch", fetchMock);
    const blob = new Uint8Array([1, 2, 3]);
    await postMap("http://x:8080", "doc-1", blob);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe("http://x:8080/map?document_id=doc-1&token=tok");
    expect(init.body).toBe(blob);
  });

  it("retries on a 500 then succeeds", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(500, { error: "boom" }))
      .mockResolvedValueOnce(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] });
    expect(result).toEqual({ document_id: "doc-1" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("throws (does not retry) on a 400", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(400, { error: "invalid payload" }));
    vi.stubGlobal("fetch", fetchMock);
    await expect(
      postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] })
    ).rejects.toThrow(/invalid payload/);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("retries on a network-level fetch rejection then succeeds", async () => {
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("fetch failed"))
      .mockResolvedValueOnce(jsonResponse(200, { document_id: "doc-1" }));
    vi.stubGlobal("fetch", fetchMock);
    const result = await postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] });
    expect(result).toEqual({ document_id: "doc-1" });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("throws a labeled error after exhausting retries on repeated network failure", async () => {
    const fetchMock = vi.fn().mockRejectedValue(new TypeError("fetch failed"));
    vi.stubGlobal("fetch", fetchMock);
    await expect(
      postIngest("http://x:8080", { collection: "c1", external_id: "d", full_text: "t", chunks: [] })
    ).rejects.toThrow(/postIngest failed: network error/);
    expect(fetchMock).toHaveBeenCalledTimes(4);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/xberg-web-ui && npx vitest run tests/sync-client.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```typescript
// src/lib/sync-client.ts
import { authedUrl, authHeaders } from "./auth-client.js";
import type { CollectionPayload, IngestPayload } from "./types.js";

const MAX_RETRIES = 3;
const BACKOFF_MS = 400;

/**
 * POST with retry+backoff on 5xx AND on a network-level `fetch` rejection
 * (DNS failure, connection refused — realistic when the local MCP dev
 * server isn't up yet). 4xx is a client error (bad payload, unknown
 * collection) and is never retried.
 */
async function postWithRetry(url: string, init: RequestInit, label: string): Promise<Response> {
  let lastResponse: Response | undefined;
  let lastError: unknown;
  for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
    try {
      const res = await fetch(url, init);
      if (res.status < 500) return res;
      lastResponse = res;
      lastError = undefined;
    } catch (err) {
      lastError = err;
      lastResponse = undefined;
    }
    if (attempt < MAX_RETRIES) {
      await new Promise((resolve) => setTimeout(resolve, BACKOFF_MS * 2 ** attempt));
    }
  }
  if (lastResponse) return lastResponse;
  const message = lastError instanceof Error ? lastError.message : String(lastError);
  throw new Error(`${label} failed: network error: ${message}`);
}

async function throwOnError(res: Response, label: string): Promise<Response> {
  if (!res.ok) {
    let detail = "";
    try {
      const body = (await res.json()) as { error?: string };
      detail = body.error ?? "";
    } catch {
      // response body wasn't JSON; fall through with the empty detail
    }
    throw new Error(`${label} failed (${res.status})${detail ? `: ${detail}` : ""}`);
  }
  return res;
}

export async function postCollection(baseUrl: string, payload: CollectionPayload): Promise<void> {
  const res = await postWithRetry(
    authedUrl(baseUrl, "/collection"),
    { method: "POST", headers: { "Content-Type": "application/json", ...authHeaders() }, body: JSON.stringify(payload) },
    "postCollection"
  );
  await throwOnError(res, "postCollection");
}

export async function postIngest(baseUrl: string, payload: IngestPayload): Promise<{ document_id: string }> {
  const res = await postWithRetry(
    authedUrl(baseUrl, "/ingest"),
    { method: "POST", headers: { "Content-Type": "application/json", ...authHeaders() }, body: JSON.stringify(payload) },
    "postIngest"
  );
  await throwOnError(res, "postIngest");
  return (await res.json()) as { document_id: string };
}

export async function postMap(baseUrl: string, documentId: string, blob: Uint8Array): Promise<void> {
  const res = await postWithRetry(
    authedUrl(baseUrl, `/map?document_id=${encodeURIComponent(documentId)}`),
    { method: "POST", headers: authHeaders(), body: blob },
    "postMap"
  );
  await throwOnError(res, "postMap");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/sync-client.test.ts`
Expected: PASS (7 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/src/lib/sync-client.ts packages/xberg-web-ui/tests/sync-client.test.ts
git commit -m "feat(web-ui): sync-client for /collection, /ingest, /map (retry+backoff)"
```

---

### Task 5: `ingest-history.ts` — local IndexedDB registry

**Files:**
- Create: `packages/xberg-web-ui/src/lib/ingest-history.ts`
- Create: `packages/xberg-web-ui/tests/ingest-history.test.ts`

**Interfaces:**
- Consumes: `IngestHistoryEntry` (Task 3).
- Produces: `putHistoryEntry(entry: IngestHistoryEntry): Promise<void>`, `listHistory(collection?: string): Promise<IngestHistoryEntry[]>`, `getHistoryEntry(collection: string, externalId: string): Promise<IngestHistoryEntry | null>`, `listFolders(): Promise<string[]>` — consumed by Task 8's screens and Task 6's worker-client (write path).

- [ ] **Step 1: Write the failing test (jsdom has no real IndexedDB — use `fake-indexeddb`)**

Add the dev dependency: edit `packages/xberg-web-ui/package.json`'s `devDependencies` to add `"fake-indexeddb": "^6.0.0"`, then `pnpm install`.

```typescript
// tests/ingest-history.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import "fake-indexeddb/auto";
import { putHistoryEntry, listHistory, getHistoryEntry, listFolders } from "../src/lib/ingest-history.js";
import type { IngestHistoryEntry } from "../src/lib/types.js";

function entry(overrides: Partial<IngestHistoryEntry> = {}): IngestHistoryEntry {
  return {
    collection: "c1",
    externalId: "doc-1.pdf",
    filename: "doc-1.pdf",
    mime: "application/pdf",
    redactedText: "Hello [EMAIL_1]",
    piiCategoryCounts: { EMAIL: 1 },
    documentId: "doc-1",
    status: "synced",
    ingestedAt: 1000,
    ...overrides,
  };
}

describe("lib/ingest-history", () => {
  beforeEach(async () => {
    indexedDB.deleteDatabase("xberg-web-ui");
  });

  it("round-trips a single entry", async () => {
    await putHistoryEntry(entry());
    const found = await getHistoryEntry("c1", "doc-1.pdf");
    expect(found).toEqual(entry());
  });

  it("returns null for a missing entry", async () => {
    expect(await getHistoryEntry("c1", "missing.pdf")).toBeNull();
  });

  it("listHistory filters by collection", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "b.pdf" }));
    const c1Only = await listHistory("c1");
    expect(c1Only.map((e) => e.externalId)).toEqual(["a.pdf"]);
  });

  it("listHistory with no filter returns everything", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "b.pdf" }));
    expect((await listHistory()).length).toBe(2);
  });

  it("putHistoryEntry upserts by (collection, externalId)", async () => {
    await putHistoryEntry(entry({ status: "pending" }));
    await putHistoryEntry(entry({ status: "synced" }));
    const all = await listHistory("c1");
    expect(all.length).toBe(1);
    expect(all[0]?.status).toBe("synced");
  });

  it("listFolders returns distinct collection names", async () => {
    await putHistoryEntry(entry({ collection: "c1", externalId: "a.pdf" }));
    await putHistoryEntry(entry({ collection: "c1", externalId: "b.pdf" }));
    await putHistoryEntry(entry({ collection: "c2", externalId: "c.pdf" }));
    expect(await listFolders()).toEqual(["c1", "c2"]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/xberg-web-ui && npx vitest run tests/ingest-history.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write minimal implementation**

```typescript
// src/lib/ingest-history.ts
import type { IngestHistoryEntry } from "./types.js";

const DB_NAME = "xberg-web-ui";
const DB_VERSION = 1;
const STORE_NAME = "ingest-history";

function keyFor(collection: string, externalId: string): string {
  return `${collection}::${externalId}`;
}

function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: "key" });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("failed to open indexedDB"));
  });
}

export async function putHistoryEntry(entry: IngestHistoryEntry): Promise<void> {
  const db = await openDb();
  await new Promise<void>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readwrite");
    tx.objectStore(STORE_NAME).put({ key: keyFor(entry.collection, entry.externalId), ...entry });
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error ?? new Error("failed to write ingest history entry"));
  });
  db.close();
}

export async function getHistoryEntry(collection: string, externalId: string): Promise<IngestHistoryEntry | null> {
  const db = await openDb();
  const result = await new Promise<IngestHistoryEntry | null>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readonly");
    const req = tx.objectStore(STORE_NAME).get(keyFor(collection, externalId));
    req.onsuccess = () => {
      if (!req.result) {
        resolve(null);
        return;
      }
      const { key, ...entry } = req.result as IngestHistoryEntry & { key: string };
      resolve(entry);
    };
    req.onerror = () => reject(req.error ?? new Error("failed to read ingest history entry"));
  });
  db.close();
  return result;
}

export async function listHistory(collection?: string): Promise<IngestHistoryEntry[]> {
  const db = await openDb();
  const all = await new Promise<IngestHistoryEntry[]>((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, "readonly");
    const req = tx.objectStore(STORE_NAME).getAll();
    req.onsuccess = () => {
      const rows = (req.result as Array<IngestHistoryEntry & { key: string }>).map(({ key, ...entry }) => entry);
      resolve(rows);
    };
    req.onerror = () => reject(req.error ?? new Error("failed to list ingest history"));
  });
  db.close();
  return collection ? all.filter((e) => e.collection === collection) : all;
}

export async function listFolders(): Promise<string[]> {
  const all = await listHistory();
  return Array.from(new Set(all.map((e) => e.collection))).sort();
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/ingest-history.test.ts`
Expected: PASS (6 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/src/lib/ingest-history.ts packages/xberg-web-ui/tests/ingest-history.test.ts \
        packages/xberg-web-ui/package.json packages/xberg-web-ui/pnpm-lock.yaml
git commit -m "feat(web-ui): IndexedDB-backed local ingest history registry"
```

---

### Task 6: `engine.worker.ts` + `worker-client.ts` — WASM engine in a Worker, HTTP-backed store

**Files:**
- Create: `packages/xberg-web-ui/src/engine/engine.worker.ts`
- Create: `packages/xberg-web-ui/src/engine/worker-client.ts`
- Create: `packages/xberg-web-ui/tests/worker-client.test.ts`

**Interfaces:**
- Consumes: `createXbergRuntimeFactory` + `XbergEngine` (`xberg-wasm-runtime`, `@xberg-io/xberg-wasm`); `postCollection`/`postIngest`/`postMap` (Task 4); `putHistoryEntry` (Task 5); `sanitizeExternalId` (Task 3); `EMBEDDING_DIM` (Task 3).
- Produces: worker message protocol `{ type: "ingest", requestId, file, filename, mime, collection, passphrase, mcpBaseUrl }` in (the `File` itself, structured-cloneable, posted synchronously — the worker converts it to bytes via `arrayBuffer()` on its side), `{ type: "progress"|"result"|"error", requestId, ... }` out; `class WorkerClient { constructor(worker: Worker, baseUrl: string); ingestFile(file: File, collection: string, passphrase: string, onProgress?: (stage: string) => void): Promise<IngestHistoryEntry> }` — consumed by Task 7's `EngineProvider`. **`baseUrl` is threaded through the constructor and sent on every `ingest` message** — the worker has no other way to learn the MCP's origin, since it runs off the main thread and cannot read `window.location`.

**Testing note:** the worker itself (`engine.worker.ts`) constructs a real `XbergEngine`, which needs the built wasm binary — it is exercised only by Task 9's e2e test, gated the same way Lot 1's wasm-dependent suite is. This task's unit test covers `worker-client.ts`'s message-passing logic in isolation using a fake `Worker`.

- [ ] **Step 1: Write the failing test (fake `Worker`)**

```typescript
// tests/worker-client.test.ts
import { describe, it, expect, vi } from "vitest";
import { WorkerClient } from "../src/engine/worker-client.js";

class FakeWorker implements Partial<Worker> {
  onmessage: ((ev: MessageEvent) => void) | null = null;
  posted: unknown[] = [];
  postMessage(msg: unknown): void {
    this.posted.push(msg);
  }
  addEventListener(_type: string, listener: EventListenerOrEventListenerObject): void {
    this.onmessage = listener as (ev: MessageEvent) => void;
  }
  removeEventListener(): void {
    this.onmessage = null;
  }
  emit(data: unknown): void {
    this.onmessage?.({ data } as MessageEvent);
  }
}

describe("engine/worker-client", () => {
  it("resolves ingestFile with the final result on a 'result' message", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1, 2, 3])], "a.pdf", { type: "application/pdf" });

    const promise = client.ingestFile(file, "c1", "pass1234");
    const sentMsg = fake.posted[0] as { type: string; requestId: string; filename: string; collection: string; mcpBaseUrl: string };
    expect(sentMsg.type).toBe("ingest");
    expect(sentMsg.filename).toBe("a.pdf");
    expect(sentMsg.collection).toBe("c1");
    expect(sentMsg.mcpBaseUrl).toBe("http://x:8080");

    fake.emit({
      type: "result",
      requestId: sentMsg.requestId,
      entry: { collection: "c1", externalId: "a.pdf", filename: "a.pdf", mime: "application/pdf", redactedText: "hi", piiCategoryCounts: {}, documentId: "doc-1", status: "synced", ingestedAt: 1 },
    });

    const result = await promise;
    expect(result.documentId).toBe("doc-1");
  });

  it("rejects ingestFile on an 'error' message", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1])], "b.pdf", { type: "application/pdf" });

    const promise = client.ingestFile(file, "c1", "pass1234");
    const sentMsg = fake.posted[0] as { requestId: string };
    fake.emit({ type: "error", requestId: sentMsg.requestId, message: "collection not found: c1" });

    await expect(promise).rejects.toThrow(/collection not found/);
  });

  it("calls onProgress for intermediate 'progress' messages, ignoring other request ids", async () => {
    const fake = new FakeWorker();
    const client = new WorkerClient(fake as unknown as Worker, "http://x:8080");
    const file = new File([new Uint8Array([1])], "c.pdf", { type: "application/pdf" });
    const stages: string[] = [];

    const promise = client.ingestFile(file, "c1", "pass1234", (stage) => stages.push(stage));
    const sentMsg = fake.posted[0] as { requestId: string };
    fake.emit({ type: "progress", requestId: "some-other-request", stage: "extract" });
    fake.emit({ type: "progress", requestId: sentMsg.requestId, stage: "extract" });
    fake.emit({ type: "progress", requestId: sentMsg.requestId, stage: "ingest" });
    fake.emit({
      type: "result",
      requestId: sentMsg.requestId,
      entry: { collection: "c1", externalId: "c.pdf", filename: "c.pdf", mime: "application/pdf", redactedText: "", piiCategoryCounts: {}, documentId: "doc-2", status: "synced", ingestedAt: 1 },
    });
    await promise;

    expect(stages).toEqual(["extract", "ingest"]);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/xberg-web-ui && npx vitest run tests/worker-client.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write `worker-client.ts`**

```typescript
// src/engine/worker-client.ts
import type { IngestHistoryEntry } from "../lib/types.js";

type ProgressMessage = { type: "progress"; requestId: string; stage: string };
type ResultMessage = { type: "result"; requestId: string; entry: IngestHistoryEntry };
type ErrorMessage = { type: "error"; requestId: string; message: string };
type WorkerOutMessage = ProgressMessage | ResultMessage | ErrorMessage;

function randomRequestId(): string {
  return `req-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`;
}

/**
 * Wraps a `postMessage` RPC protocol around the engine worker. One
 * `WorkerClient` per `Worker` instance; `ingestFile` calls are queued by
 * the worker itself (it processes one file at a time — `XbergEngine` is
 * not proven reentrant), so callers may fire multiple concurrent
 * `ingestFile` calls without waiting, but each one only resolves once its
 * own `requestId` gets a `result`/`error` message.
 */
export class WorkerClient {
  constructor(
    private readonly worker: Worker,
    private readonly baseUrl: string
  ) {}

  ingestFile(
    file: File,
    collection: string,
    passphrase: string,
    onProgress?: (stage: string) => void
  ): Promise<IngestHistoryEntry> {
    return new Promise((resolve, reject) => {
      const requestId = randomRequestId();

      const onMessage = (ev: MessageEvent<WorkerOutMessage>): void => {
        const msg = ev.data;
        if (msg.requestId !== requestId) return;
        if (msg.type === "progress") {
          onProgress?.(msg.stage);
          return;
        }
        this.worker.removeEventListener("message", onMessage as EventListener);
        if (msg.type === "error") {
          reject(new Error(msg.message));
        } else {
          resolve(msg.entry);
        }
      };

      this.worker.addEventListener("message", onMessage as EventListener);

      // Post the `File` itself (structured-cloneable) synchronously, inside
      // the executor — not after `await file.arrayBuffer()`, which resolves
      // as a microtask and would post after the caller's next synchronous
      // line already ran. The worker does the `arrayBuffer()` conversion on
      // its side instead (see `engine.worker.ts`).
      this.worker.postMessage({
        type: "ingest",
        requestId,
        file,
        filename: file.name,
        mime: file.type,
        collection,
        passphrase,
        mcpBaseUrl: this.baseUrl,
      });
    });
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/worker-client.test.ts`
Expected: PASS (3 tests)

- [ ] **Step 5: Write `engine.worker.ts`**

This file only runs inside a real Worker with the wasm binary present, so it has no direct unit test — Task 9's e2e test is its verification. Its job: build the engine once (lazily, on first message), reusing it across ingests in the same tab session, and turn a single `ingest` message into the extract → ingest → encrypt_map → postMap sequence described in Global Constraints.

```typescript
// src/engine/engine.worker.ts
/// <reference lib="webworker" />
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import type { VectorStoreInterface, DocumentRecord, ChunkRecord, CollectionSpec } from "xberg-wasm-runtime";
import { XbergEngine } from "@xberg-io/xberg-wasm";
import { postIngest, postMap } from "../lib/sync-client.js";
import { sanitizeExternalId } from "../lib/sanitize-id.js";
import { EMBEDDING_DIM } from "../lib/constants.js";
import type { IngestHistoryEntry } from "../lib/types.js";

declare const self: DedicatedWorkerGlobalScope;

interface IngestMessage {
  type: "ingest";
  requestId: string;
  // Sent as the structured-cloneable `File` itself, not pre-converted
  // bytes — see `worker-client.ts`'s `ingestFile`, which must post
  // synchronously (inside the Promise executor) rather than after an
  // `await file.arrayBuffer()`, so the conversion happens here instead.
  file: File;
  filename: string;
  mime: string;
  collection: string;
  passphrase: string;
  mcpBaseUrl: string;
}

let mcpBaseUrl = "";
let engine: XbergEngine | null = null;

/**
 * HTTP-backed `VectorStoreInterface`. Only `upsertDocument` matters for
 * `engine.ingest()` — everything else throws, since this shim exists
 * solely to redirect the WASM engine's internal store write to `POST
 * /ingest` instead of a local OPFS/SQLite write.
 */
function createHttpStore(): VectorStoreInterface {
  const notSupported = (name: string) => async () => {
    throw new Error(`${name} is not supported by the browser HTTP-backed store`);
  };
  return {
    close: async () => undefined,
    ensureCollection: notSupported("ensureCollection") as (spec: CollectionSpec) => Promise<string | void>,
    dropCollection: notSupported("dropCollection"),
    getCollection: notSupported("getCollection"),
    deleteDocuments: notSupported("deleteDocuments"),
    deleteByFilter: notSupported("deleteByFilter"),
    retrieve: notSupported("retrieve"),
    collectionStats: notSupported("collectionStats"),
    async upsertDocument(collection: string, doc: DocumentRecord, chunks: ChunkRecord[]): Promise<string> {
      if (chunks.length > 0 && chunks[0] && chunks[0].embedding.length !== EMBEDDING_DIM) {
        throw new Error(
          `embedder produced ${chunks[0].embedding.length}-dim vectors, expected ${EMBEDDING_DIM} (EMBEDDING_DIM constant is stale — update it and the /collection embedding_dim together)`
        );
      }
      const { document_id } = await postIngest(mcpBaseUrl, {
        collection,
        external_id: doc.external_id ?? "",
        title: doc.title,
        mime: doc.mime,
        source_uri: doc.source_uri,
        full_text: doc.full_text,
        keywords: doc.keywords,
        metadata: doc.metadata as Record<string, unknown> | undefined,
        chunks: chunks.map((c) => ({ ordinal: c.ordinal, content: c.content, embedding: c.embedding, chunk_metadata: c.chunk_metadata })),
      });
      return document_id;
    },
  };
}

async function getEngine(): Promise<XbergEngine> {
  if (engine) return engine;
  const injection = await createXbergRuntimeFactory();
  injection.store = createHttpStore();
  engine = new XbergEngine({}, injection);
  return engine;
}

function post(msg: unknown, transfer: Transferable[] = []): void {
  self.postMessage(msg, transfer);
}

async function handleIngest(msg: IngestMessage): Promise<void> {
  const { requestId, file, filename, mime, collection, passphrase } = msg;
  try {
    const xEngine = await getEngine();
    const externalId = sanitizeExternalId(filename);
    const bytes = new Uint8Array(await file.arrayBuffer());

    post({ type: "progress", requestId, stage: "extract" });
    const extracted = await xEngine.extract({ kind: "bytes", bytes: Array.from(bytes), filename }, undefined);
    const first = (extracted as { results?: Array<{ content: string; mimeType: string }> }).results?.[0];
    if (!first) throw new Error(`extraction produced no result for ${filename}`);

    post({ type: "progress", requestId, stage: "ingest" });
    const outcome = (await xEngine.ingest(
      { full_text: first.content, title: filename, mime: first.mimeType || mime, source_uri: filename, external_id: externalId },
      collection
    )) as { document_id: string; rehydration_map: Record<string, string>; pii_category_counts: Record<string, number> };

    post({ type: "progress", requestId, stage: "encrypt" });
    const blob = xEngine.encrypt_map(outcome.rehydration_map, passphrase);

    post({ type: "progress", requestId, stage: "map" });
    // MUST be `externalId`, NOT `outcome.document_id`. `/map`'s `document_id`
    // query param and `rehydrate_tokens`'s `document_id` argument are both
    // named after the *file's* base name (see `mcp-server/src/tools/ingest.ts`:
    // `path.join(rehydrationDir, \`${baseName}.map\`)`), not the store's
    // generated UUID — despite the store's return value happening to also be
    // called `document_id`. These are two different things that share a
    // name; using the UUID here writes a map file no rehydration tool can
    // ever find by the id a human/UI would actually have on hand.
    await postMap(mcpBaseUrl, externalId, blob);

    const entry: IngestHistoryEntry = {
      collection,
      externalId,
      filename,
      mime: first.mimeType || mime,
      redactedText: first.content,
      piiCategoryCounts: outcome.pii_category_counts,
      documentId: outcome.document_id,
      status: "synced",
      ingestedAt: Date.now(),
    };
    post({ type: "result", requestId, entry });
  } catch (err) {
    post({ type: "error", requestId, message: err instanceof Error ? err.message : String(err) });
  }
}

self.addEventListener("message", (ev: MessageEvent) => {
  const msg = ev.data as IngestMessage;
  if (msg.type === "ingest") {
    mcpBaseUrl = msg.mcpBaseUrl;
    void handleIngest(msg);
  }
});
```

- [ ] **Step 6: Type-check**

Run: `cd packages/xberg-web-ui && npx tsc --noEmit`
Expected: no errors (the worker file type-checks against the `webworker` lib included in `tsconfig.json`'s `lib` array from Task 2)

- [ ] **Step 7: Commit**

```bash
git add packages/xberg-web-ui/src/engine/ packages/xberg-web-ui/tests/worker-client.test.ts
git commit -m "feat(web-ui): engine worker (HTTP-backed store) + worker-client RPC"
```

---

### Task 7: `EngineProvider` / `useEngine()` React context

**Files:**
- Create: `packages/xberg-web-ui/src/providers/EngineProvider.tsx`
- Create: `packages/xberg-web-ui/tests/providers/EngineProvider.test.tsx`

**Interfaces:**
- Consumes: `WorkerClient` (Task 6); `authedUrl`/`captureAuthTokenFromLocation`/`getAuthToken` (Task 3); `putHistoryEntry` (Task 5).
- Produces: `<EngineProvider baseUrl={...}>` component, `useEngine(): { ready: boolean; ingestFile(file: File, collection: string, passphrase: string): Promise<IngestHistoryEntry>; pendingCount: number; lastError: string | null }` — consumed by Task 8's screens and by Lot 3's Task 5 (`useEngine().ocrLayout`, `useEngine().ingest`).

- [ ] **Step 1: Write the failing test (inject a fake `WorkerClient`)**

```tsx
// tests/providers/EngineProvider.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { EngineProvider, useEngine } from "../../src/providers/EngineProvider.js";
import type { IngestHistoryEntry } from "../../src/lib/types.js";

function Probe({ onReady }: { onReady: (api: ReturnType<typeof useEngine>) => void }) {
  const api = useEngine();
  onReady(api);
  return <div data-testid="pending-count">{api.pendingCount}</div>;
}

describe("providers/EngineProvider", () => {
  it("exposes ingestFile and tracks pendingCount across an in-flight call", async () => {
    const entry: IngestHistoryEntry = {
      collection: "c1", externalId: "a.pdf", filename: "a.pdf", mime: "application/pdf",
      redactedText: "hi", piiCategoryCounts: {}, documentId: "doc-1", status: "synced", ingestedAt: 1,
    };
    let resolveIngest: (e: IngestHistoryEntry) => void = () => {};
    const fakeClient = { ingestFile: vi.fn(() => new Promise<IngestHistoryEntry>((r) => (resolveIngest = r))) };

    let api: ReturnType<typeof useEngine> | null = null;
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <Probe onReady={(a) => (api = a)} />
      </EngineProvider>
    );

    let promise!: Promise<IngestHistoryEntry>;
    await act(async () => {
      const file = new File([new Uint8Array([1])], "a.pdf", { type: "application/pdf" });
      promise = api!.ingestFile(file, "c1", "pass1234");
    });
    expect(screen.getByTestId("pending-count").textContent).toBe("1");

    await act(async () => {
      resolveIngest(entry);
      await promise;
    });
    await waitFor(() => expect(screen.getByTestId("pending-count").textContent).toBe("0"));
  });

  it("surfaces the error message via lastError when ingestFile rejects", async () => {
    const fakeClient = { ingestFile: vi.fn().mockRejectedValue(new Error("collection not found: c1")) };
    let api: ReturnType<typeof useEngine> | null = null;
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <Probe onReady={(a) => (api = a)} />
      </EngineProvider>
    );

    await act(async () => {
      const file = new File([new Uint8Array([1])], "a.pdf", { type: "application/pdf" });
      await expect(api!.ingestFile(file, "c1", "pass1234")).rejects.toThrow();
    });
    expect(api!.lastError).toMatch(/collection not found/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/xberg-web-ui && npx vitest run tests/providers/EngineProvider.test.tsx`
Expected: FAIL — module not found

- [ ] **Step 3: Write `EngineProvider.tsx`**

```tsx
// src/providers/EngineProvider.tsx
"use client";
import { createContext, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { WorkerClient } from "../engine/worker-client.js";
import { captureAuthTokenFromLocation } from "../lib/auth-client.js";
import { putHistoryEntry } from "../lib/ingest-history.js";
import type { IngestHistoryEntry } from "../lib/types.js";

interface EngineApi {
  ready: boolean;
  pendingCount: number;
  lastError: string | null;
  ingestFile(file: File, collection: string, passphrase: string): Promise<IngestHistoryEntry>;
}

const EngineContext = createContext<EngineApi | null>(null);

export function useEngine(): EngineApi {
  const api = useContext(EngineContext);
  if (!api) throw new Error("useEngine() must be called inside an <EngineProvider>");
  return api;
}

interface EngineProviderProps {
  baseUrl: string;
  children: ReactNode;
  /** Test-only escape hatch — production callers never pass this. */
  workerClient?: Pick<WorkerClient, "ingestFile">;
}

export function EngineProvider({ baseUrl, children, workerClient }: EngineProviderProps) {
  const clientRef = useRef<Pick<WorkerClient, "ingestFile"> | null>(workerClient ?? null);
  const [ready, setReady] = useState(Boolean(workerClient));
  const [pendingCount, setPendingCount] = useState(0);
  const [lastError, setLastError] = useState<string | null>(null);

  useEffect(() => {
    captureAuthTokenFromLocation();
    if (clientRef.current) return;
    const worker = new Worker(new URL("../engine/engine.worker.ts", import.meta.url), { type: "module" });
    clientRef.current = new WorkerClient(worker, baseUrl);
    setReady(true);
    return () => worker.terminate();
  }, [baseUrl]);

  const api = useMemo<EngineApi>(
    () => ({
      ready,
      pendingCount,
      lastError,
      async ingestFile(file, collection, passphrase) {
        if (!clientRef.current) throw new Error("engine worker not ready yet");
        setPendingCount((n) => n + 1);
        setLastError(null);
        try {
          const entry = await clientRef.current.ingestFile(file, collection, passphrase);
          await putHistoryEntry(entry);
          return entry;
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          setLastError(message);
          throw err;
        } finally {
          setPendingCount((n) => n - 1);
        }
      },
    }),
    [ready, pendingCount, lastError]
  );

  return <EngineContext.Provider value={api}>{children}</EngineContext.Provider>;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/providers/EngineProvider.test.tsx`
Expected: PASS (2 tests)

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/src/providers/ packages/xberg-web-ui/tests/providers/
git commit -m "feat(web-ui): EngineProvider/useEngine React context"
```

---

### Task 8: Screens — folder list, folder detail (upload + table), document detail, sync bar

**Files:**
- Create: `packages/xberg-web-ui/src/components/ui/button.tsx`, `dialog.tsx`, `input.tsx`, `table.tsx`, `card.tsx`, `badge.tsx` (shadcn primitives)
- Create: `packages/xberg-web-ui/src/lib/utils.ts` (shadcn's `cn()` helper)
- Create: `packages/xberg-web-ui/src/components/CreateFolderDialog.tsx`
- Create: `packages/xberg-web-ui/tests/components/CreateFolderDialog.test.tsx`
- Create: `packages/xberg-web-ui/src/components/SyncBar.tsx`
- Create: `packages/xberg-web-ui/tests/components/SyncBar.test.tsx`
- Create: `packages/xberg-web-ui/src/components/DocumentTable.tsx`
- Create: `packages/xberg-web-ui/src/components/DocumentViewer.tsx`
- Modify: `packages/xberg-web-ui/src/app/layout.tsx` (mount `EngineProvider` + `SyncBar`)
- Modify: `packages/xberg-web-ui/src/app/page.tsx` (folder list)
- Create: `packages/xberg-web-ui/src/app/folder/[collection]/page.tsx`
- Create: `packages/xberg-web-ui/src/app/document/[collection]/[id]/page.tsx`

**Interfaces:**
- Consumes: `useEngine()` (Task 7), `listFolders`/`listHistory`/`getHistoryEntry` (Task 5), `postCollection` (Task 4), `EMBEDDING_DIM` (Task 3).
- Produces: `CreateFolderDialog({ onCreated: (name: string) => void })`, `SyncBar()`, `DocumentTable({ collection: string })`, `DocumentViewer({ entry: IngestHistoryEntry })` — `DocumentTable`/`DocumentViewer` are explicitly extended by Lot 3 (Global Constraints); keep these exact names.

Run `npx shadcn@latest init` is not reproducible in a plan (interactive CLI); this task hand-writes the small subset of primitives actually used (`button`, `dialog`, `input`, `table`, `card`, `badge`) in the standard shadcn/ui shape so Lot 3's `npx shadcn@latest add @extend/...` has the expected `@/components/ui/*` aliases already present to merge into.

- [ ] **Step 1: `lib/utils.ts` + `button.tsx` (representative primitive; `input.tsx`/`card.tsx`/`badge.tsx` follow the same shadcn pattern)**

```typescript
// src/lib/utils.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

```tsx
// src/components/ui/button.tsx
import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils.js";

const buttonVariants = cva(
  "inline-flex items-center justify-center rounded-md text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-slate-900 text-white hover:bg-slate-800",
        destructive: "bg-red-600 text-white hover:bg-red-700",
        outline: "border border-slate-300 hover:bg-slate-100",
      },
      size: { default: "h-9 px-4 py-2", sm: "h-8 px-3" },
    },
    defaultVariants: { variant: "default", size: "default" },
  }
);

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(({ className, variant, size, ...props }, ref) => (
  <button ref={ref} className={cn(buttonVariants({ variant, size }), className)} {...props} />
));
Button.displayName = "Button";
```

```tsx
// src/components/ui/input.tsx
import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils.js";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn("h-9 w-full rounded-md border border-slate-300 px-3 text-sm outline-none focus:ring-2 focus:ring-slate-400", className)}
    {...props}
  />
));
Input.displayName = "Input";
```

```tsx
// src/components/ui/card.tsx
import { cn } from "@/lib/utils.js";
import type { HTMLAttributes } from "react";

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("rounded-lg border border-slate-200 shadow-sm", className)} {...props} />;
}
export function CardHeader({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("p-4 border-b border-slate-100", className)} {...props} />;
}
export function CardContent({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("p-4", className)} {...props} />;
}
```

```tsx
// src/components/ui/badge.tsx
import { cn } from "@/lib/utils.js";
import type { HTMLAttributes } from "react";

export function Badge({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("inline-flex items-center rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium", className)} {...props} />;
}
```

```tsx
// src/components/ui/table.tsx
import { cn } from "@/lib/utils.js";
import type { HTMLAttributes, TdHTMLAttributes, ThHTMLAttributes } from "react";

export function Table({ className, ...props }: HTMLAttributes<HTMLTableElement>) {
  return <table className={cn("w-full text-sm", className)} {...props} />;
}
export function TableHeader(props: HTMLAttributes<HTMLTableSectionElement>) {
  return <thead {...props} />;
}
export function TableBody(props: HTMLAttributes<HTMLTableSectionElement>) {
  return <tbody {...props} />;
}
export function TableRow({ className, ...props }: HTMLAttributes<HTMLTableRowElement>) {
  return <tr className={cn("border-b border-slate-100", className)} {...props} />;
}
export function TableHead({ className, ...props }: ThHTMLAttributes<HTMLTableCellElement>) {
  return <th className={cn("text-left p-2 font-medium text-slate-500", className)} {...props} />;
}
export function TableCell({ className, ...props }: TdHTMLAttributes<HTMLTableCellElement>) {
  return <td className={cn("p-2", className)} {...props} />;
}
```

```tsx
// src/components/ui/dialog.tsx
"use client";
import { createContext, useContext, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils.js";

const DialogCtx = createContext<{ open: boolean; setOpen: (v: boolean) => void } | null>(null);

export function Dialog({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return <DialogCtx.Provider value={{ open, setOpen }}>{children}</DialogCtx.Provider>;
}
export function DialogTrigger({ children }: { children: ReactNode; asChild?: boolean }) {
  const ctx = useContext(DialogCtx)!;
  return <span onClick={() => ctx.setOpen(true)}>{children}</span>;
}
export function DialogContent({ className, children }: { className?: string; children: ReactNode }) {
  const ctx = useContext(DialogCtx)!;
  if (!ctx.open) return null;
  return (
    <div role="dialog" className={cn("fixed inset-0 z-50 flex items-center justify-center bg-black/40")}>
      <div className={cn("rounded-lg bg-white p-6 shadow-lg", className)}>{children}</div>
    </div>
  );
}
export function DialogHeader({ children }: { children: ReactNode }) {
  return <div className="mb-4">{children}</div>;
}
export function DialogTitle({ children }: { children: ReactNode }) {
  return <h2 className="text-lg font-semibold">{children}</h2>;
}
export function DialogFooter({ children }: { children: ReactNode }) {
  return <div className="mt-4 flex justify-end gap-2">{children}</div>;
}
export function DialogClose({ children }: { children: ReactNode; asChild?: boolean }) {
  const ctx = useContext(DialogCtx)!;
  return <span onClick={() => ctx.setOpen(false)}>{children}</span>;
}
```

- [ ] **Step 2: `CreateFolderDialog.tsx` + failing test**

```tsx
// tests/components/CreateFolderDialog.test.tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CreateFolderDialog } from "../../src/components/CreateFolderDialog.js";
import { setAuthToken } from "../../src/lib/auth-client.js";

describe("CreateFolderDialog", () => {
  beforeEach(() => {
    setAuthToken("tok");
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => ({ created: true }) })
    );
  });

  it("creates a folder and calls onCreated with the sanitized name", async () => {
    const onCreated = vi.fn();
    render(<CreateFolderDialog baseUrl="http://x:8080" onCreated={onCreated} />);

    fireEvent.click(screen.getByText("New folder"));
    fireEvent.change(screen.getByLabelText("Folder name"), { target: { value: "Dossier Client X" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith("Dossier_Client_X"));
  });
});
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/CreateFolderDialog.test.tsx`
Expected: FAIL — module not found

```tsx
// src/components/CreateFolderDialog.tsx
"use client";
import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogTrigger, DialogClose } from "@/components/ui/dialog.js";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import { postCollection } from "@/lib/sync-client.js";
import { sanitizeExternalId } from "@/lib/sanitize-id.js";
import { EMBEDDING_DIM } from "@/lib/constants.js";

export function CreateFolderDialog({ baseUrl, onCreated }: { baseUrl: string; onCreated: (name: string) => void }) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const create = async () => {
    const safeName = sanitizeExternalId(name.trim());
    setBusy(true);
    setError(null);
    try {
      await postCollection(baseUrl, { name: safeName, embedding_dim: EMBEDDING_DIM });
      onCreated(safeName);
      setName("");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog>
      <DialogTrigger>
        <Button>New folder</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New folder</DialogTitle>
        </DialogHeader>
        <label htmlFor="folder-name" className="text-sm font-medium">
          Folder name
        </label>
        <Input id="folder-name" value={name} onChange={(e) => setName(e.target.value)} />
        {error && (
          <p role="alert" className="mt-2 text-sm text-red-600">
            {error}
          </p>
        )}
        <DialogFooter>
          <DialogClose>
            <Button variant="outline" disabled={busy}>
              Cancel
            </Button>
          </DialogClose>
          <Button disabled={busy || name.trim().length === 0} onClick={create}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/CreateFolderDialog.test.tsx`
Expected: PASS (1 test)

- [ ] **Step 3: `SyncBar.tsx` + failing test**

```tsx
// tests/components/SyncBar.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SyncBar } from "../../src/components/SyncBar.js";
import { EngineProvider } from "../../src/providers/EngineProvider.js";

describe("SyncBar", () => {
  it("shows 'All synced' when nothing is pending and there is no error", () => {
    const fakeClient = { ingestFile: vi.fn() };
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <SyncBar />
      </EngineProvider>
    );
    expect(screen.getByText("All synced")).toBeDefined();
  });
});
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/SyncBar.test.tsx`
Expected: FAIL — module not found

```tsx
// src/components/SyncBar.tsx
"use client";
import { useEngine } from "@/providers/EngineProvider.js";
import { Badge } from "@/components/ui/badge.js";

export function SyncBar() {
  const { pendingCount, lastError } = useEngine();
  return (
    <div className="flex items-center justify-end gap-2 border-b border-slate-200 px-4 py-2 text-sm">
      {lastError && (
        <Badge className="bg-red-100 text-red-700" role="alert">
          {lastError}
        </Badge>
      )}
      {pendingCount > 0 ? (
        <Badge className="bg-amber-100 text-amber-700">Syncing {pendingCount}…</Badge>
      ) : (
        !lastError && <span className="text-slate-500">All synced</span>
      )}
    </div>
  );
}
```

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/SyncBar.test.tsx`
Expected: PASS (1 test)

- [ ] **Step 4: `DocumentTable.tsx` and `DocumentViewer.tsx` (basic V1 — Lot 3 extends both)**

```tsx
// src/components/DocumentTable.tsx
"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { useReactTable, getCoreRowModel, flexRender, createColumnHelper } from "@tanstack/react-table";
import { listHistory } from "@/lib/ingest-history.js";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/components/ui/table.js";
import { Badge } from "@/components/ui/badge.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

const columnHelper = createColumnHelper<IngestHistoryEntry>();
const columns = [
  columnHelper.accessor("filename", {
    header: "Document",
    cell: (info) => (
      <Link className="text-slate-900 underline" href={`/document/${info.row.original.collection}/${info.row.original.externalId}`}>
        {info.getValue()}
      </Link>
    ),
  }),
  columnHelper.accessor("status", { header: "Status", cell: (info) => <Badge>{info.getValue()}</Badge> }),
  columnHelper.accessor("piiCategoryCounts", {
    header: "PII",
    cell: (info) => Object.entries(info.getValue()).map(([k, v]) => `${k}:${v}`).join(", ") || "none",
  }),
];

export function DocumentTable({ collection }: { collection: string }) {
  const [rows, setRows] = useState<IngestHistoryEntry[]>([]);

  useEffect(() => {
    void listHistory(collection).then(setRows);
  }, [collection]);

  const table = useReactTable({ data: rows, columns, getCoreRowModel: getCoreRowModel() });

  if (rows.length === 0) return <p className="text-sm text-slate-500">No documents yet.</p>;

  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map((hg) => (
          <TableRow key={hg.id}>
            {hg.headers.map((h) => (
              <TableHead key={h.id}>{flexRender(h.column.columnDef.header, h.getContext())}</TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.map((row) => (
          <TableRow key={row.id}>
            {row.getVisibleCells().map((cell) => (
              <TableCell key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
```

```tsx
// src/components/DocumentViewer.tsx
import { Card, CardHeader, CardContent } from "@/components/ui/card.js";
import { Badge } from "@/components/ui/badge.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

/**
 * V1: redacted text + PII counts only. Lot 3 replaces the body with
 * extend-hq PDF/DOCX/XLSX viewers, `LayoutBlocks`, and
 * `BoundingBoxCitations` — keep this component's name and the
 * `{ entry }` prop shape stable for that migration.
 */
export function DocumentViewer({ entry }: { entry: IngestHistoryEntry }) {
  return (
    <Card>
      <CardHeader>
        <h1 className="text-lg font-semibold">{entry.filename}</h1>
        <div className="mt-1 flex gap-1">
          {Object.entries(entry.piiCategoryCounts).map(([cat, n]) => (
            <Badge key={cat}>
              {cat}: {n}
            </Badge>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        <p className="whitespace-pre-wrap text-sm">{entry.redactedText}</p>
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 5: Wire the screens**

```tsx
// src/app/layout.tsx
import "./globals.css";
import type { ReactNode } from "react";
import { EngineProvider } from "@/providers/EngineProvider.js";
import { SyncBar } from "@/components/SyncBar.js";

const MCP_BASE_URL = process.env.NEXT_PUBLIC_MCP_BASE_URL ?? (typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:8080");

export const metadata = { title: "Xberg" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <EngineProvider baseUrl={MCP_BASE_URL}>
          <SyncBar />
          {children}
        </EngineProvider>
      </body>
    </html>
  );
}
```

```tsx
// src/app/page.tsx
"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { listFolders } from "@/lib/ingest-history.js";
import { CreateFolderDialog } from "@/components/CreateFolderDialog.js";

export default function HomePage() {
  const [folders, setFolders] = useState<string[]>([]);
  const baseUrl = typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:8080";

  useEffect(() => {
    void listFolders().then(setFolders);
  }, []);

  return (
    <main className="p-6">
      <div className="mb-4 flex items-center justify-between">
        <h1 className="text-xl font-semibold">Folders</h1>
        <CreateFolderDialog baseUrl={baseUrl} onCreated={(name) => setFolders((f) => Array.from(new Set([...f, name])))} />
      </div>
      {folders.length === 0 ? (
        <p className="text-sm text-slate-500">No folders yet — create one to start uploading.</p>
      ) : (
        <ul className="space-y-1">
          {folders.map((f) => (
            <li key={f}>
              <Link className="text-slate-900 underline" href={`/folder/${f}`}>
                {f}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}
```

```tsx
// src/app/folder/[collection]/page.tsx
"use client";
import { useState } from "react";
import { useParams } from "next/navigation";
import { useEngine } from "@/providers/EngineProvider.js";
import { DocumentTable } from "@/components/DocumentTable.js";
import { Input } from "@/components/ui/input.js";

export default function FolderPage() {
  const { collection } = useParams<{ collection: string }>();
  const { ingestFile } = useEngine();
  const [passphrase, setPassphrase] = useState("");

  const onFiles = async (files: FileList | null) => {
    if (!files || !passphrase) return;
    for (const file of Array.from(files)) {
      await ingestFile(file, collection, passphrase);
    }
  };

  return (
    <main className="p-6">
      <h1 className="mb-4 text-xl font-semibold">{collection}</h1>
      <div className="mb-4 space-y-2">
        <label htmlFor="passphrase" className="text-sm font-medium">
          Rehydration passphrase (never sent to the server in clear)
        </label>
        <Input id="passphrase" type="password" value={passphrase} onChange={(e) => setPassphrase(e.target.value)} />
        <input type="file" multiple disabled={!passphrase} onChange={(e) => void onFiles(e.target.files)} />
      </div>
      <DocumentTable collection={collection} />
    </main>
  );
}
```

```tsx
// src/app/document/[collection]/[id]/page.tsx
"use client";
import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { getHistoryEntry } from "@/lib/ingest-history.js";
import { DocumentViewer } from "@/components/DocumentViewer.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

export default function DocumentPage() {
  const { collection, id } = useParams<{ collection: string; id: string }>();
  const [entry, setEntry] = useState<IngestHistoryEntry | null>(null);

  useEffect(() => {
    void getHistoryEntry(collection, id).then(setEntry);
  }, [collection, id]);

  if (!entry) return <main className="p-6">Loading…</main>;
  return (
    <main className="p-6">
      <DocumentViewer entry={entry} />
    </main>
  );
}
```

- [ ] **Step 6: Type-check + run the full component test suite**

Run: `cd packages/xberg-web-ui && npx tsc --noEmit && npx vitest run`
Expected: no type errors; all tests pass (smoke + sanitize-id + auth-client + sync-client + ingest-history + worker-client + EngineProvider + CreateFolderDialog + SyncBar)

- [ ] **Step 7: Commit**

```bash
git add packages/xberg-web-ui/src/components/ packages/xberg-web-ui/src/lib/utils.ts \
        packages/xberg-web-ui/src/app/ packages/xberg-web-ui/tests/components/
git commit -m "feat(web-ui): folder/document screens, sync bar, shadcn primitives"
```

---

### Task 9: Static export wiring + Playwright e2e

**Files:**
- Create: `packages/xberg-web-ui/scripts/export-to-mcp.mjs`
- Create: `packages/xberg-web-ui/e2e/ingest.spec.ts`

**Interfaces:**
- Consumes: everything from Tasks 1-8; `mcp-server/ui-dist/` (Lot 1's default static root, `mcp-server/src/http/ui-server.ts:25`).

- [ ] **Step 1: Write the copy script**

```javascript
// scripts/export-to-mcp.mjs
import { cpSync, rmSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, "..", "out");
const targetDir = join(here, "..", "..", "..", "mcp-server", "ui-dist");

if (!existsSync(outDir)) {
  throw new Error(`static export not found at ${outDir} — run "next build" first`);
}

rmSync(targetDir, { recursive: true, force: true });
cpSync(outDir, targetDir, { recursive: true });
console.log(`copied ${outDir} -> ${targetDir}`);
```

- [ ] **Step 2: Write the e2e test**

```typescript
// e2e/ingest.spec.ts
import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { EMBEDDING_DIM } from "../src/lib/constants.js";

test("uploading a document with PII syncs to the MCP store via /collection, /ingest, /map", async ({ page }) => {
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
    send(404, {});
  });
  await new Promise<void>((resolve) => server.listen(8081, "127.0.0.1", resolve));

  try {
    await page.goto("http://127.0.0.1:8081/ui/?token=test");
    await page.getByText("New folder").click();
    // Playwright's locator method is `getByLabel`, not Testing Library's
    // `getByLabelText` — do not confuse the two APIs.
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
    // /collection must have received the real CollectionPayloadSchema shape
    // (mcp-server/src/http/collection-route.ts) with the sanitized folder
    // name and the fixed EMBEDDING_DIM — not just captured and ignored.
    expect(received.collection).toEqual({ name: "contrats", embedding_dim: EMBEDDING_DIM });
    expect(received.mapDocumentId).toBe("contrat.pdf");
    expect((received.ingest as { external_id: string }).external_id).toBe("contrat.pdf");
    expect((received.ingest as { full_text: string }).full_text).not.toContain("alice@example.com");
  } finally {
    server.close();
  }
});
```

- [ ] **Step 3: Run the e2e (needs a built wasm binary + `mcp-server/ui-dist` populated — gated the same way as the rest of this repo's wasm-dependent suites)**

Run: `cd packages/xberg-web-ui && npx next build && node scripts/export-to-mcp.mjs && npx playwright test e2e/ingest.spec.ts`
Expected: PASS with the wasm binary built; if `crates/xberg-wasm/pkg/nodejs/xberg_wasm_bg.wasm` is not present in this environment, this step cannot be verified here (same known limitation as Lot 1's `http-ui-routes.test.ts`) — run it wherever the wasm build succeeds before merging.

- [ ] **Step 4: Run the full non-e2e web-ui suite one more time**

Run: `cd packages/xberg-web-ui && npx vitest run && npx tsc --noEmit`
Expected: PASS; no type errors

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/scripts/export-to-mcp.mjs packages/xberg-web-ui/e2e/ingest.spec.ts
git commit -m "feat(web-ui): static export copy script + upload-to-sync e2e test"
```

---

## Self-Review Notes

- **Spec coverage:** folder creation (Task 1's `POST /collection` + Task 8's `CreateFolderDialog`), upload pipeline extract→OCR→NER→PII→chunk→embed entirely client-side (Task 6, delegated to `engine.ingest()` per the design's "WASM is the ingestion engine" principle), auto-sync after each document with retry+backoff (Task 4's `sync-client.ts`, called from Task 6's worker), encrypted rehydration map pushed separately (Task 6's `encrypt_map` + `postMap`), the 4 V1 screens (Task 8: folder list, folder+upload, document view, global sync bar), Chrome/Edge-only static export with no SSR (Task 2's `next.config.js`). Advanced viewers/OCR-layout/PII bounding boxes and delete/re-ingest are explicitly Lot 3, not duplicated here.
- **Reconciled spec vs. reality:** the design spec's example `IngestPayload` JSON (`collectionId`/`sourceId`/`redactedFullText`/`chunks[].index`) does not match the actually-implemented `/ingest` route (`collection`/`external_id`/`full_text`/`chunks[].ordinal`) — every task in this plan uses the real, already-shipped `ingest-route.ts` schema, not the spec's earlier sketch.
- **Type consistency:** `IngestHistoryEntry` (Task 3) is the type every later task reads/writes — `ingest-history.ts` (Task 5), `worker-client.ts`'s `result` message (Task 6), `EngineProvider.ingestFile`'s return (Task 7), and `DocumentTable`/`DocumentViewer`'s props (Task 8) all use the identical shape. `CollectionPayload`/`IngestPayload` (Task 3) match `collection-route.ts`'s `CollectionPayloadSchema` (Task 1) and `ingest-route.ts`'s existing `IngestPayloadSchema` field-for-field.
- **Gap filled beyond the spec's lot breakdown:** the spec's three lots never mention a collection-creation route, but `store.upsertDocument` hard-requires the collection to pre-exist (`store-node.ts:130-134`) — Task 1 adds `POST /collection`, following the exact auth/validation/error-mapping pattern Lot 1 and Lot 3 already established, so it is a natural, low-risk extension of `ui-server.ts` rather than a new subsystem.
- **PII safety:** the plaintext rehydration map only exists in the worker's local variables between `engine.ingest()` returning and `postMap()` completing (Task 6) — never written to `ingest-history.ts` (Task 5's `IngestHistoryEntry` has no map field), never logged, matching the `pii-pipeline` rule.
- **No placeholders:** every code block is complete, runnable TypeScript/TSX; shadcn primitives are hand-written in the standard shape (not "TODO: run shadcn add") since the CLI is interactive and not plan-reproducible, but the resulting files are exactly what the CLI would generate for these components, so Lot 3's `npx shadcn@latest add @extend/...` merges into them cleanly.
- **Naming reconciliation with Lot 3:** the design spec and Lot 3's Architecture prose both call the orchestrator "`ingest-controller.ts`"; this plan splits that responsibility across `engine.worker.ts` (does the actual extract/ingest/encrypt/map sequence, isolated in a Worker so the WASM engine + transformers.js models never block the UI thread) and `worker-client.ts`/`EngineProvider.tsx` (the RPC + React binding) instead of one file with that name — no task in this plan or in Lot 3's own Files/Interfaces sections hard-references a literal `ingest-controller.ts` path, so this is a non-breaking architectural choice. The produced call is `useEngine().ingestFile(file, collection, passphrase)` (Task 7) — Lot 3's prose ("the UI re-runs `ingestFile`", line 48) matches this; Lot 3's Task 5 code sample instead writes `useEngine().ingest({ ...args, file })`, which is Lot 3's own internal inconsistency (not introduced here) — Lot 3's implementer should use `ingestFile` as defined by this plan.
- **Known verification gap:** Task 6's `engine.worker.ts` and Task 9's e2e test both require the built `xberg-wasm` binary, unavailable in the current environment (same root cause noted when reviewing Lot 1: `wasm-pack build` fails with a Windows permission error). Every other task (1-5, 7-8's non-e2e parts) is fully unit-testable and was designed to not require the wasm build.
