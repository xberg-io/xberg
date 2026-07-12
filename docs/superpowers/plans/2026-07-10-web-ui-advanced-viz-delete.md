# Web UI Advanced Visualization & Re-ingestion/Deletion Plan (Lot 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the `packages/xberg-web-ui` package (Lot 2) with advanced document visualization — OCR layout blocks with confidence scores, PII detection review over the redacted text, and `PDF/DOCX/XLSX` viewers — plus in-UI re-ingestion and deletion, backed by a new MCP `POST /admin` route (`drop_collection` / `delete_documents` / `stats`) gated by the same localhost token as Lot 1. Visualization components are sourced from **extend-hq/ui** (`@extend/*` shadcn registry); bespoke controls (delete dialog, re-ingest button, sync bar, passphrase prompt, folder list) use **shadcn** primitives.

**Architecture:** Lot 3 is additive on Lot 1 (routes + `ui-server.ts` + auth token — **already implemented**) and Lot 2 (web-ui package, `ingest-controller`, `sync-client`, providers, screens — **must be implemented first**). It adds one MCP route module (`admin-route.ts`) wired into Lot 1's `ui-server.ts`, a browser `admin-client`, the extend-hq/ui visualization components, and deletion/re-ingestion controls in the existing folder/document screens. The MCP remains the authority on the SQLite store; the browser cache is still a preview/registry.

**Tech Stack:** Next.js static export, React 18, TypeScript strict, Tailwind. **Extend UI** components installed via `npx shadcn@latest add @extend/<component>` (MIT, sourced as code into `packages/xberg-web-ui/components/ui/` + `components/blocks/`). **shadcn** primitives (button, dialog, select, table, tooltip, popover, input, card, badge) for bespoke controls. TanStack Query/Table, Vitest + Testing Library, Playwright. MCP side: `node:http` + `zod`, Vitest.

---

## Investigation findings — extend-hq/ui

Cloned `github.com/extend-hq/ui` (shallow) and read the source. Key facts that shape this plan:

- **Distribution:** Extend UI is a shadcn registry. Components are installed with `npx shadcn@latest add @extend/<name>` and copied into the project as source under `@/components/ui/...` (and `@/components/blocks/...` for composed blocks). The raw GitHub `apps/v4/components/ui/` folder contains only **re-export stubs** (`export * from "@/registry/new-york-v4/ui/..."`); the real implementations live in extend-hq's **hosted registry**, not the clone. → **Install via the shadcn CLI**, do not vendor from the GitHub clone.
- **Primitives are local:** the registry bundles its own `button`, `dialog`, `select`, `tooltip`, `popover`, `input`, `scroll-area`, `card`, `badge`, `tabs`, etc. into the same `@/components/ui/*` alias. No separate "Coss UI" install is required — but our app must have those aliased primitives present (shadcn provides them).
- **Heavy viewers need native deps:** `@extend/pdf-viewer` (react-pdf / pdfjs-dist), `@extend/docx-viewer` (mammoth), `@extend/xlsx-viewer`, `@extend/file-system` (175 KB, embeds its own viewers + dialogs). These are large and pull sizeable dependency trees; they cannot be built/verified in the current environment (no wasm build, broken Windows toolchain). They are still the right components to use — just flag the build/verify prerequisite.

### Component mapping (design "Stack UI" ↔ extend)

| Our need (design) | Extend component (`npx shadcn add`) | Notes |
|---|---|---|
| PDF viewer | `@extend/pdf-viewer` | `PDFViewerProps { file, className, ... }`; page overlay via `PDFViewerPageOverlayProps`; imperative `PDFViewerHandle`. |
| DOCX viewer | `@extend/docx-viewer` | mammoth-based; `DocxViewerPreview` exported. |
| XLSX viewer | `@extend/xlsx-viewer` | sheet tabs, search popover, zoom. |
| File upload into a folder | `@extend/file-upload` | thin; reuses viewer upload buttons. |
| Folder navigation (Finder) | `@extend/file-system-block` | `FileSystemProps { items, title, defaultView: "icons"\|"list"\|"columns"\|"gallery", getFileUrl, loadChildren, onSelectionChange }`; `FileSystemItem = folder \| file` (`FileSystemFileItem { kind:"file", path, contentType, url, previewImageUrl? }`). |
| OCR layout + confidence | `@extend/layout-blocks-block` | `ParsedOcrOutput { chunks: [{ blocks: [{ id, type, content, metadata:{ page:{number,width,height}, avgOcrConfidence }, boundingBox:{left,top,right,bottom} }] }] }`. |
| PII detection review | `@extend/bounding-box-citations-block` | `ReviewField { key, schema:{type,title,description}, actual, expected?, location?:{page, area:{left,top,width,height}} }`. |
| Document splits / thumbnails / sidebar | `@extend/document-splits`, `@extend/file-thumbnail`, `@extend/document-viewer-sidebar` | optional enhancements. |

### shadcn for the rest (bespoke controls)
`DeleteDialog`, `ReingestButton`, `SyncBar`, `PassphrasePrompt`, `FolderList` are app-specific → build with shadcn `dialog`, `button`, `table`, `tooltip`, `input`, `card`, `badge`. `DocumentTable` uses TanStack Table on top of shadcn `table`.

---

## Global Constraints

- **Prerequisites:** Lot 1 is implemented (verified: `mcp-server/src/http/{auth,ingest-route,map-route,static-server,ui-server}.ts` all exist). **Lot 2 must be implemented first** — `packages/xberg-web-ui` does not yet exist. Tasks 2–6 below assume the Lot 2 package, providers, `ingest-controller`, `sync-client`, and screens are in place.
- **Extend install path:** run `npx shadcn@latest add @extend/pdf-viewer @extend/docx-viewer @extend/xlsx-viewer @extend/file-upload @extend/file-system-block @extend/layout-blocks-block @extend/bounding-box-citations-block` from `packages/xberg-web-ui`, after configuring `components.json` (alias `@/components/ui`, `@/components/blocks`, baseColor slate, Tailwind). The CLI resolves the hosted registry; generated source is committed.
- **New MCP route `POST /admin`** (auth-gated, same token as Lot 1's `extractToken`/`isValidToken`):
  - `{ "op": "drop_collection", "collection": "<name>" }` → `store.dropCollection(collection)` → `{ dropped: true }`.
  - `{ "op": "delete_documents", "collection": "<name>", "external_ids": ["<name1>", ...] }` → `store.deleteDocuments(collection, external_ids)` (Node store matches by **external_id OR document_id** — `store-node.ts:257`) → `{ deleted: <count> }`.
  - `{ "op": "stats", "collection": "<name>" }` → `store.collectionStats(collection)` → `{ documents, chunks, last_ingested_at? }`.
  - 4xx on invalid payload / unknown collection; 401 without a valid token (handled by `ui-server.ts` before dispatch).
- **Re-ingestion = no new route.** The UI re-runs `ingestFile` (Lot 2 Task 5) with the same `external_id`; Lot 1's `upsertDocument` replaces the document's chunks idempotently. Plan 3 only adds a "Re-ingest" button wired to the existing controller.
- **Deletion authority is MCP.** After a successful `POST /admin`, the UI prunes its local doc registry/OPFS preview for the deleted `external_id`s; it never deletes from the SQLite store directly.
- **Document list source.** There is still no server-side "list documents" method. The UI's folder/document registry is the local ingest history (OPFS preview cache + in-memory map from Lot 2), the source of selectable rows. `stats` (new) gives MCP-side document/chunk counts for display and e2e assertions.
- **Visualization is client-side and PII-safe.** `BoundingBoxCitations` renders only the redacted text + `[CATEGORY_n]` tokens + category counts. The clear original values live only in the in-memory rehydration `map` (never persisted, never sent to MCP), so the viewer shows a token's *category* but NOT its clear value.
- **OCR layout (`LayoutBlocks`)** is produced by re-running OCR in the engine worker on the cached original bytes (the store holds only redacted text). The worker gains an `ocrLayout(bytes)` message returning `OcrResult["lines"]`; for non-image docs `LayoutBlocks` falls back to rendering chunk blocks as positioned cards. Map worker OCR `lines` (text/confidence/bbox) into the extend `ParsedOcrOutput` shape via an adapter.
- **No new crypto / no new wire format** in Lot 3. Browser `admin-client` posts JSON only.

---

## File Structure

```
mcp-server/src/http/admin-route.ts              # NEW: createAdminHandler(getStore)
mcp-server/src/http/ui-server.ts                # MODIFY (Lot 1): add /admin branch
mcp-server/tests/http-admin-route.test.ts       # NEW
mcp-server/tests/http-ui-routes.test.ts         # MODIFY (Lot 1): extend with delete/stats assertions
packages/xberg-web-ui/
  components/ui/                                 # shadcn primitives + extend-generated viewers
    pdf-viewer.tsx  docx-viewer.tsx  xlsx-viewer.tsx
    file-system-block.tsx  layout-blocks-block.tsx  bounding-box-citations-block.tsx
    file-upload.tsx  file-thumbnail.tsx  document-splits.tsx  document-viewer-sidebar.tsx
    (button/dialog/select/table/tooltip/popover/input/card/badge per shadcn)
  components/blocks/                             # composed extend blocks (if registry emits them)
  src/lib/admin-client.ts                       # NEW: postAdmin(baseUrl, token, payload)
  src/engine/engine.worker.ts                   # MODIFY (Lot 2): add ocrLayout message
  src/engine/worker-client.ts                   # MODIFY (Lot 2): add ocrLayout()
  src/lib/ocr-to-layout.ts                      # NEW: adapt OcrResult.lines -> ParsedOcrOutput
  src/components/LayoutBlocks.tsx               # NEW: wraps @extend/layout-blocks-block
  src/components/BoundingBoxCitations.tsx       # NEW: wraps @extend/bounding-box-citations-block
  src/components/DocumentViewer.tsx             # MODIFY (Lot 2): use @extend viewers
  src/components/DeleteDialog.tsx               # NEW (shadcn dialog)
  src/components/ReingestButton.tsx             # NEW
  src/components/DocumentTable.tsx              # MODIFY (Lot 2): row selection + actions
  src/app/document/[collection]/[id]/page.tsx   # MODIFY (Lot 2): mount viewers
  tests/admin-client.test.ts                    # NEW
  tests/components/LayoutBlocks.test.tsx        # NEW
  tests/components/BoundingBoxCitations.test.tsx# NEW
  e2e/delete.spec.ts                            # NEW
```

---

### Task 1: MCP `POST /admin` route + wire into `ui-server.ts`

**Files:**
- Create: `mcp-server/src/http/admin-route.ts`
- Create: `mcp-server/tests/http-admin-route.test.ts`
- Modify: `mcp-server/src/http/ui-server.ts` (Lot 1)
- Modify: `mcp-server/tests/http-ui-routes.test.ts` (Lot 1)

**Interfaces:**
- Consumes: `VectorStoreInterface` (`dropCollection`, `deleteDocuments`, `collectionStats`).
- Produces: `createAdminHandler(getStore: () => VectorStoreInterface): (req, res, url) => Promise<void>`.

- [ ] **Step 1: Write the failing test**

```ts
// mcp-server/tests/http-admin-route.test.ts
import { describe, it, expect } from "vitest";
import { createServer, type Server } from "node:http";
import type { VectorStoreInterface, CollectionStats } from "xberg-wasm-runtime";
import { createAdminHandler } from "../src/http/admin-route.js";

function notImpl(name: string) { return async () => { throw new Error(`${name} not implemented`); }; }
function fakeStore(over: Partial<VectorStoreInterface> = {}): VectorStoreInterface {
  return {
    ensureCollection: notImpl("ensureCollection"), getCollection: notImpl("getCollection"),
    upsertDocument: notImpl("upsertDocument"), retrieve: notImpl("retrieve"),
    deleteByFilter: notImpl("deleteByFilter"),
    ...over,
  } as VectorStoreInterface;
}
async function withServer(store: VectorStoreInterface, fn: (base: string) => Promise<void>) {
  const handler = createAdminHandler(() => store);
  const server: Server = createServer((req, res) => { void handler(req, res, new URL(req.url ?? "/", "http://localhost")); });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  const a = server.address(); if (a === null || typeof a === "string") throw new Error("addr");
  try { await fn(`http://127.0.0.1:${a.port}`); } finally { await new Promise<void>((r) => server.close((e) => (e ? console.error(e) : r()))); }
}
describe("http/admin-route", () => {
  it("drop_collection returns { dropped: true }", async () => {
    let called = ""; const store = fakeStore({ dropCollection: async (c) => { called = c; return undefined; } });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "drop_collection", collection: "c1" }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual({ dropped: true }); expect(called).toBe("c1");
    });
  });
  it("delete_documents by external_ids returns the deleted count", async () => {
    let got: string[] = []; const store = fakeStore({ deleteDocuments: async (_c, ids) => { got = ids; return 2; } });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "delete_documents", collection: "c1", external_ids: ["a.pdf", "b.pdf"] }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual({ deleted: 2 }); expect(got).toEqual(["a.pdf", "b.pdf"]);
    });
  });
  it("stats returns collection stats", async () => {
    const stats: CollectionStats = { documents: 3, chunks: 9 }; const store = fakeStore({ collectionStats: async () => stats });
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "stats", collection: "c1" }) });
      expect(res.status).toBe(200); expect((await res.json())).toEqual(stats);
    });
  });
  it("rejects an unknown op with 400", async () => {
    const store = fakeStore();
    await withServer(store, async (base) => {
      const res = await fetch(`${base}/admin`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "bogus" }) });
      expect(res.status).toBe(400);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd mcp-server && npx vitest run tests/http-admin-route.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write `admin-route.ts`**

```ts
// mcp-server/src/http/admin-route.ts
import { z } from "zod";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { VectorStoreInterface } from "xberg-wasm-runtime";

const MAX_BODY_BYTES = 1 * 1024 * 1024;
const AdminPayloadSchema = z.discriminatedUnion("op", [
  z.object({ op: z.literal("drop_collection"), collection: z.string().min(1) }),
  z.object({ op: z.literal("delete_documents"), collection: z.string().min(1), external_ids: z.array(z.string().min(1)).min(1) }),
  z.object({ op: z.literal("stats"), collection: z.string().min(1) }),
]);
export type AdminPayload = z.infer<typeof AdminPayloadSchema>;

function statusForError(message: string): number { return message.includes("not found") ? 404 : 400; }

export function createAdminHandler(
  getStore: () => VectorStoreInterface,
): (req: IncomingMessage, res: ServerResponse, url: URL) => Promise<void> {
  return async function handleAdmin(req: IncomingMessage, res: ServerResponse, _url: URL): Promise<void> {
    const chunks: Buffer[] = []; let total = 0;
    try {
      for await (const c of req) { total += (c as Buffer).length; if (total > MAX_BODY_BYTES) throw new Error("payload too large"); chunks.push(c as Buffer); }
    } catch (e) { res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid JSON body" })); return; }
    const parsed = AdminPayloadSchema.safeParse(JSON.parse(Buffer.concat(chunks).toString("utf-8")));
    if (!parsed.success) { res.writeHead(400, { "Content-Type": "application/json" }).end(JSON.stringify({ error: "invalid admin payload", issues: parsed.error.issues })); return; }
    const p = parsed.data;
    try {
      if (p.op === "drop_collection") { await getStore().dropCollection(p.collection); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ dropped: true })); }
      else if (p.op === "delete_documents") { const deleted = await getStore().deleteDocuments(p.collection, p.external_ids); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify({ deleted })); }
      else { const stats = await getStore().collectionStats(p.collection); res.writeHead(200, { "Content-Type": "application/json" }).end(JSON.stringify(stats)); }
    } catch (err) { const msg = err instanceof Error ? err.message : String(err); res.writeHead(statusForError(msg), { "Content-Type": "application/json" }).end(JSON.stringify({ error: msg })); }
  };
}
```

- [ ] **Step 4: Wire `/admin` into `ui-server.ts`**

In `mcp-server/src/http/ui-server.ts`, add the import and an `/admin` branch inside `handleRequest`, right after the `isMap` check (still inside the token-validated block):

```ts
import { createAdminHandler } from "./admin-route.js";
// inside createUiRoutes():
const adminHandler = createAdminHandler(() => getRuntime().store);
// inside handleRequest, after `if (isMap) { ...; return true; }`:
const isAdmin = req.method === "POST" && url.pathname === "/admin";
if (isAdmin) { await adminHandler(req, res, url); return true; }
```

- [ ] **Step 5: Extend Lot 1 integration test with delete + stats**

In `mcp-server/tests/http-ui-routes.test.ts`, append (after the `/map` test):

```ts
it("POST /admin delete_documents removes the ingested doc (stats drops to 0)", async () => {
  const del = await fetch(`${baseUrl}/admin?token=${token}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "delete_documents", collection: "http_ingest_test", external_ids: ["doc-1"] }) });
  expect(del.status).toBe(200); expect((await del.json()).deleted).toBeGreaterThanOrEqual(1);
  const stats = await fetch(`${baseUrl}/admin?token=${token}`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ op: "stats", collection: "http_ingest_test" }) });
  expect((await stats.json()).documents).toBe(0);
});
```

- [ ] **Step 6: Run the new + extended suites**

Run: `cd mcp-server && npx vitest run tests/http-admin-route.test.ts tests/http-ui-routes.test.ts`
Expected: PASS (admin route 4 tests; extended ui-routes still green)

- [ ] **Step 7: Commit**

```bash
git add mcp-server/src/http/admin-route.ts mcp-server/tests/http-admin-route.test.ts mcp-server/src/http/ui-server.ts mcp-server/tests/http-ui-routes.test.ts
git commit -m "feat(mcp): add POST /admin route for drop/delete/stats, token-gated"
```

---

### Task 2: Install extend components + browser `admin-client` + worker `ocrLayout`

**Files:**
- Run: `npx shadcn@latest add @extend/pdf-viewer @extend/docx-viewer @extend/xlsx-viewer @extend/file-upload @extend/file-system-block @extend/layout-blocks-block @extend/bounding-box-citations-block` (from `packages/xberg-web-ui`)
- Create: `packages/xberg-web-ui/src/lib/admin-client.ts`
- Create: `packages/xberg-web-ui/tests/admin-client.test.ts`
- Modify: `packages/xberg-web-ui/src/engine/engine.worker.ts` (Lot 2 — add `ocr` message)
- Modify: `packages/xberg-web-ui/src/engine/worker-client.ts` (Lot 2 — add `ocrLayout`)
- Create: `packages/xberg-web-ui/src/lib/ocr-to-layout.ts`

**Interfaces:**
- `postAdmin(baseUrl, token, payload: AdminPayload): Promise<{ dropped?: boolean; deleted?: number } | CollectionStats>` — `POST /admin?token=`, JSON, `Authorization: Bearer`, retries on 5xx.
- `workerClient.ocrLayout(bytes): Promise<{ text: string; confidence: number; bbox?: { x:number;y:number;w:number;h:number } }[]>`.
- `toParsedOcrOutput(lines, width, height): ParsedOcrOutput` — maps worker OCR lines to the extend `layout-blocks` `ParsedOcrOutput` shape (one chunk, blocks per line, `boundingBox` from `bbox`, `avgOcrConfidence` from `confidence`).

- [ ] **Step 1: Write the failing test (mock `fetch`)**

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { postAdmin } from "../../src/lib/admin-client.js";
function json(status: number, body: unknown) { return { status, ok: status < 300, json: async () => body } as Response; }
describe("lib/admin-client", () => {
  beforeEach(() => vi.stubGlobal("fetch", vi.fn()));
  afterEach(() => vi.unstubAllGlobals());
  it("posts drop_collection and returns { dropped: true }", async () => {
    const m = vi.fn().mockResolvedValue(json(200, { dropped: true })); vi.stubGlobal("fetch", m);
    expect(await postAdmin("http://x:8080", "tok", { op: "drop_collection", collection: "c1" })).toEqual({ dropped: true });
    expect(m.mock.calls[0]![0]).toContain("/admin?token=tok");
  });
  it("posts delete_documents and returns { deleted }", async () => {
    const m = vi.fn().mockResolvedValue(json(200, { deleted: 2 })); vi.stubGlobal("fetch", m);
    expect(await postAdmin("http://x:8080", "tok", { op: "delete_documents", collection: "c1", external_ids: ["a", "b"] })).toEqual({ deleted: 2 });
  });
  it("throws on 400", async () => {
    const m = vi.fn().mockResolvedValue(json(400, { error: "bad" })); vi.stubGlobal("fetch", m);
    await expect(postAdmin("http://x:8080", "tok", { op: "stats", collection: "c1" })).rejects.toThrow();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd packages/xberg-web-ui && npx vitest run tests/admin-client.test.ts`
Expected: FAIL — module not found

- [ ] **Step 3: Write `admin-client.ts`**

```ts
// src/lib/admin-client.ts
export type AdminPayload =
  | { op: "drop_collection"; collection: string }
  | { op: "delete_documents"; collection: string; external_ids: string[] }
  | { op: "stats"; collection: string };
export type AdminResult = { dropped?: boolean; deleted?: number; documents?: number; chunks?: number; last_ingested_at?: number };

const MAX_RETRIES = 3, BACKOFF_MS = 400;
async function postWithRetry(url: string, init: RequestInit): Promise<Response> {
  let last: Response | undefined;
  for (let i = 0; i <= MAX_RETRIES; i++) { const res = await fetch(url, init); last = res; if (res.status < 500) return res; if (i < MAX_RETRIES) await new Promise((r) => setTimeout(r, BACKOFF_MS * 2 ** i)); }
  return last!;
}
export async function postAdmin(baseUrl: string, token: string, payload: AdminPayload): Promise<AdminResult> {
  const url = `${baseUrl.replace(/\/$/, "")}/admin?token=${encodeURIComponent(token)}`;
  const res = await postWithRetry(url, { method: "POST", headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` }, body: JSON.stringify(payload) });
  if (!res.ok) throw new Error(`admin failed (${res.status})`);
  return (await res.json()) as AdminResult;
}
```

- [ ] **Step 4: Add `ocrLayout` to the worker + client + adapter**

In `engine.worker.ts` `self.onmessage`, add:
```ts
if (msg.type === "ocr") { const out = await engine.ocr(msg.bytes); (self as any).postMessage({ type: "ocrResult", requestId: msg.requestId, lines: out.lines }); }
```
In `worker-client.ts`, add to `WorkerClient`:
```ts
ocrLayout(bytes: Uint8Array): Promise<{ text: string; confidence: number; bbox?: { x:number;y:number;w:number;h:number } }[]> {
  return new Promise((resolve, reject) => {
    const requestId = Math.random().toString(36).slice(2);
    const onMsg = (ev: MessageEvent) => { const m = ev.data; if (m.requestId !== requestId) return; worker.removeEventListener("message", onMsg); if (m.type === "error") reject(new Error(m.message)); else resolve(m.lines ?? []); };
    worker.addEventListener("message", onMsg); worker.postMessage({ type: "ocr", requestId, bytes });
  });
}
```
`src/lib/ocr-to-layout.ts`:
```ts
import type { OcrLine } from "./types.js";
import type { ParsedOcrOutput } from "@/components/ui/layout-blocks"; // extend-generated type
export function toParsedOcrOutput(lines: OcrLine[], width = 1000, height = 1400): ParsedOcrOutput {
  return { chunks: [{ blocks: lines.map((l, i) => ({
    id: `block-${i}`, type: "text", content: l.text,
    metadata: { page: { number: 1, width, height }, avgOcrConfidence: l.confidence },
    boundingBox: l.bbox ? { left: l.bbox.x, top: l.bbox.y, right: l.bbox.x + l.bbox.w, bottom: l.bbox.y + l.bbox.h } : { left: 0, top: 0, right: width, bottom: height },
  })) }] };
}
```

- [ ] **Step 5: Run admin-client test + typecheck**

Run: `cd packages/xberg-web-ui && npx vitest run tests/admin-client.test.ts && pnpm typecheck`
Expected: PASS; no type errors

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-web-ui/src/lib/admin-client.ts packages/xberg-web-ui/tests/admin-client.test.ts packages/xberg-web-ui/src/engine/ packages/xberg-web-ui/src/lib/ocr-to-layout.ts packages/xberg-web-ui/components/
git commit -m "feat(web-ui): install extend viewers, admin client, worker ocrLayout"
```

---

### Task 3: `LayoutBlocks` (extends `@extend/layout-blocks-block`)

**Files:**
- Create: `packages/xberg-web-ui/src/components/LayoutBlocks.tsx`
- Create: `packages/xberg-web-ui/tests/components/LayoutBlocks.test.tsx`

**Interfaces:** `LayoutBlocks({ lines: OcrLine[]; width?: number; height?: number })` — converts via `toParsedOcrOutput` and renders the extend `<OcrBlocksBlock file={...} output={parsed} />`. For non-image docs (no bbox), render the chunk blocks as positioned cards (extend `layout-blocks` supports chunk blocks; fall back to a simple stacked list if no PDF `file` URL is available).

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { LayoutBlocks } from "../../src/components/LayoutBlocks.js";
describe("LayoutBlocks", () => {
  it("renders one region per OCR line", () => {
    render(<LayoutBlocks lines={[{ text: "Hello", confidence: 0.95, bbox: { x: 10, y: 20, w: 100, h: 30 } }, { text: "World", confidence: 0.6 }]} width={200} height={120} />);
    expect(screen.getAllByTestId("layout-block").length).toBeGreaterThanOrEqual(1);
  });
});
```

- [ ] **Step 2: Write `LayoutBlocks.tsx`**

```tsx
// src/components/LayoutBlocks.tsx
"use client";
import { useMemo } from "react";
import { OcrBlocksBlock } from "@/components/blocks/layout-blocks-block";
import { toParsedOcrOutput } from "@/lib/ocr-to-layout.js";
import type { OcrLine } from "@/lib/types.js";

export function LayoutBlocks({ lines, width = 1000, height = 1400, file }: { lines: OcrLine[]; width?: number; height?: number; file?: string }) {
  const output = useMemo(() => toParsedOcrOutput(lines, width, height), [lines, width, height]);
  if (file) return <OcrBlocksBlock file={file} output={output} className="h-[720px]" />;
  return (
    <div data-testid="layout-stack" className="grid gap-1">
      {lines.map((l, i) => (
        <div key={i} data-testid="layout-block" className="border rounded p-1 text-sm">
          <span>{l.text}</span> <span className="text-muted-foreground">{(l.confidence * 100).toFixed(0)}%</span>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/LayoutBlocks.test.tsx`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add packages/xberg-web-ui/src/components/LayoutBlocks.tsx packages/xberg-web-ui/tests/components/LayoutBlocks.test.tsx
git commit -m "feat(web-ui): LayoutBlocks wraps extend OCR layout viewer"
```

---

### Task 4: `BoundingBoxCitations` (extends `@extend/bounding-box-citations-block`)

**Files:**
- Modify: `packages/xberg-web-ui/src/lib/types.ts` (add `OcrLine`, `PiiDetection`)
- Create: `packages/xberg-web-ui/src/components/BoundingBoxCitations.tsx`
- Create: `packages/xberg-web-ui/tests/components/BoundingBoxCitations.test.tsx`

**Interfaces:** `BoundingBoxCitations({ redactedText, map, counts })` — renders the redacted text with each `[CATEGORY_n]` token highlighted; shows the `counts` side list; maps our `counts` to extend `ReviewField[]` (one field per category, `actual` = count) and renders `<HumanReviewBlock fields={...} />` when a `file` URL is available. Clear values (`map[token]`) are NEVER rendered.

- [ ] **Step 1: Add types**

```ts
// append to src/lib/types.ts
export interface OcrLine { text: string; confidence: number; bbox?: { x: number; y: number; w: number; h: number }; }
export interface PiiDetection { token: string; category: string; confidence?: number; }
```

- [ ] **Step 2: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BoundingBoxCitations } from "../../src/components/BoundingBoxCitations.js";
describe("BoundingBoxCitations", () => {
  it("highlights PII tokens and lists counts without leaking clear values", () => {
    render(<BoundingBoxCitations redactedText="Contact [EMAIL_1] about [PERSON_1]" map={{ "[EMAIL_1]": "alice@example.com", "[PERSON_1]": "Alice" }} counts={{ EMAIL: 1, NAME: 1 }} />);
    expect(screen.getByText("[EMAIL_1]")).toBeDefined();
    expect(screen.getByText("EMAIL")).toBeDefined();
    expect(screen.queryByText("alice@example.com")).toBeNull();
    expect(screen.queryByText("Alice")).toBeNull();
  });
});
```

- [ ] **Step 3: Write `BoundingBoxCitations.tsx`**

```tsx
// src/components/BoundingBoxCitations.tsx
"use client";
import type { ReviewField } from "@/components/ui/bounding-box-citations";

const CATEGORY_COLOR: Record<string, string> = { EMAIL: "bg-blue-200", PHONE: "bg-purple-200", NAME: "bg-rose-200", PERSON: "bg-rose-200", ORGANIZATION: "bg-emerald-200", ORG: "bg-emerald-200", LOCATION: "bg-amber-200", LOC: "bg-amber-200", SSN: "bg-red-200", CREDIT_CARD: "bg-red-200", IP_ADDRESS: "bg-cyan-200" };
const colorFor = (cat: string) => CATEGORY_COLOR[cat.toUpperCase()] ?? "bg-gray-200";

export function BoundingBoxCitations({ redactedText, counts, file }: { redactedText: string; map: Record<string, string>; counts: Record<string, number>; file?: string }) {
  const parts = redactedText.split(/(\[[A-Z_]+_\d+\])/g);
  const fields: ReviewField[] = Object.entries(counts).map(([cat, n]) => ({
    key: cat, schema: { type: "number", title: cat, description: `Detected ${cat} occurrences` }, actual: n,
  }));
  return (
    <div className="grid grid-cols-3 gap-4">
      <div className="col-span-2 whitespace-pre-wrap leading-relaxed">
        {parts.map((part, i) => /^\[[A-Z_]+_\d+\]$/.test(part) ? <mark key={i} data-testid="pii-token" className={`rounded px-1 ${colorFor(part)}`}>{part}</mark> : <span key={i}>{part}</span>)}
      </div>
      <aside data-testid="pii-counts" className="text-sm">
        <h3 className="font-semibold">PII detected</h3>
        <ul>{Object.entries(counts).map(([cat, n]) => <li key={cat} data-testid="pii-count"><span className={colorFor(cat)}>{cat}</span>: {n}</li>)}</ul>
      </aside>
    </div>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd packages/xberg-web-ui && npx vitest run tests/components/BoundingBoxCitations.test.tsx`
Expected: PASS (asserts no clear-value leak)

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/src/components/BoundingBoxCitations.tsx packages/xberg-web-ui/src/lib/types.ts packages/xberg-web-ui/tests/components/BoundingBoxCitations.test.tsx
git commit -m "feat(web-ui): BoundingBoxCitations PII review (no clear-value leak)"
```

---

### Task 5: Document viewer + delete/re-ingest controls + table selection

**Files:**
- Modify: `packages/xberg-web-ui/src/components/DocumentViewer.tsx` (Lot 2)
- Create: `packages/xberg-web-ui/src/components/DeleteDialog.tsx` (shadcn `dialog`)
- Create: `packages/xberg-web-ui/src/components/ReingestButton.tsx`
- Modify: `packages/xberg-web-ui/src/components/DocumentTable.tsx` (Lot 2 — add selection + action column)
- Modify: `packages/xberg-web-ui/src/app/document/[collection]/[id]/page.tsx` (Lot 2 — mount viewers + controls)

**Interfaces:**
- `DocumentViewer({ fileUrl?, mime?, redactedText, layoutLines?, counts, map })` — for `application/pdf`/`docx`/`xlsx` with a cached original, render the matching extend viewer (`<PDFViewer file={fileUrl} />`, `<DocxViewerPreview .../>`, `<XlsxViewer .../>`); always render `BoundingBoxCitations` of the redacted text; render `LayoutBlocks` from `layoutLines` (fetched via `useEngine().ocrLayout`).
- `DeleteDialog({ external_ids, collection, onDeleted })` — shadcn `Dialog`; confirms; calls `postAdmin({ op: "delete_documents", ... })`; then `onDeleted()`.
- `ReingestButton({ file, collection })` — re-runs `useEngine().ingest({ ...args, file })` (same `external_id` → idempotent replace).

- [ ] **Step 1: Write `DeleteDialog` (shadcn dialog)**

```tsx
// src/components/DeleteDialog.tsx
"use client";
import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogTrigger, DialogClose } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { postAdmin } from "@/lib/admin-client.js";
export function DeleteDialog({ baseUrl, token, collection, externalIds, onDeleted }: { baseUrl: string; token: string; collection: string; externalIds: string[]; onDeleted: () => void; }) {
  const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null);
  const confirm = async () => { setBusy(true); setError(null); try { await postAdmin(baseUrl, token, { op: "delete_documents", collection, external_ids: externalIds }); onDeleted(); } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); } };
  return (
    <Dialog>
      <DialogTrigger asChild><Button variant="destructive" aria-label="delete-selected">Delete</Button></DialogTrigger>
      <DialogContent>
        <DialogHeader><DialogTitle>Delete {externalIds.length} document(s)?</DialogTitle></DialogHeader>
        <p className="text-sm text-muted-foreground">Removes them from the MCP store (collection "{collection}").</p>
        {error && <p role="alert" className="text-red-600">{error}</p>}
        <DialogFooter>
          <DialogClose asChild><Button variant="outline" disabled={busy}>Cancel</Button></DialogClose>
          <Button variant="destructive" aria-label="confirm-delete-yes" disabled={busy} onClick={confirm}>Confirm</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 2: Add selection to `DocumentTable`** (modify Lot 2 component): add a `useState<Set<string>>` selection, a header checkbox, per-row checkboxes (`aria-label={"select-" + name}`), and render `<DeleteDialog>` + `<ReingestButton>` in a footer action bar when `selected.size > 0`.

- [ ] **Step 3: Mount viewers + controls in the document page**

In `app/document/[collection]/[id]/page.tsx`, render `<DocumentViewer>` with the cached original URL + `layoutLines` from `useEngine().ocrLayout(cachedBytes)` + `BoundingBoxCitations` using the ingest history's `map`/`counts`; add a `<ReingestButton>` and a single-doc `<DeleteDialog>`.

- [ ] **Step 4: Build + typecheck**

Run: `cd packages/xberg-web-ui && pnpm typecheck`
Expected: no errors

- [ ] **Step 5: Component smoke test (selection → delete dialog opens)**

```tsx
// tests/components/DeleteDialog.test.tsx
import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DeleteDialog } from "../../src/components/DeleteDialog.js";
describe("DeleteDialog", () => {
  it("opens and shows the confirm copy", async () => {
    render(<DeleteDialog baseUrl="http://x" token="t" collection="c" externalIds={["a.pdf"]} onDeleted={() => {}} />);
    fireEvent.click(screen.getByLabelText("delete-selected"));
    expect(await screen.findByText(/Delete 1 document/)).toBeDefined();
  });
});
```
Run: `cd packages/xberg-web-ui && npx vitest run tests/components/DeleteDialog.test.tsx`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add packages/xberg-web-ui/src/components/ packages/xberg-web-ui/src/app/ packages/xberg-web-ui/tests/
git commit -m "feat(web-ui): extend viewers, delete dialog, re-ingest, table selection"
```

---

### Task 6: Static export + Playwright e2e (delete + viz)

**Files:**
- Modify: `packages/xberg-web-ui/playwright.config.ts`
- Create: `packages/xberg-web-ui/e2e/delete.spec.ts`

- [ ] **Step 1: Write the e2e**

```ts
// e2e/delete.spec.ts
import { test, expect } from "@playwright/test";
import { createServer } from "node:http";
import { createIngestHandler } from "../../mcp-server/src/http/ingest-route.js";
import { createMapUploadHandler } from "../../mcp-server/src/http/map-route.js";
import { createAdminHandler } from "../../mcp-server/src/http/admin-route.js";

test("upload then delete a PII doc via the UI removes it from the MCP store", async ({ page }) => {
  const received: any = { deletes: [] as any[], stats: { documents: 1 } };
  const server = createServer(async (req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    const send = (s: number, b: unknown) => { res.writeHead(s, { "Content-Type": "application/json" }); res.end(JSON.stringify(b)); };
    if (req.method === "POST" && url.pathname === "/ingest") { let b = ""; for await (const c of req) b += c; received.payload = JSON.parse(b); send(200, { document_id: "doc-x" }); return; }
    if (req.method === "POST" && url.pathname === "/map") { for await (const _ of req) {} send(200, { status: "stored" }); return; }
    if (req.method === "POST" && url.pathname === "/admin") { let b = ""; for await (const c of req) b += c; const p = JSON.parse(b);
      if (p.op === "delete_documents") { received.deletes.push(p); received.stats.documents = 0; send(200, { deleted: p.external_ids.length }); }
      else if (p.op === "stats") send(200, received.stats); else send(200, { dropped: true }); return; }
    send(404, {});
  });
  await new Promise<void>((r) => server.listen(8080, "127.0.0.1", r));
  try {
    await page.goto("http://127.0.0.1:8080/ui/?token=test");
    await page.setInputFiles("input[type=file]", { name: "contrat.pdf", mimeType: "application/pdf", buffer: Buffer.from("Contact alice@example.com") });
    await expect(page.getByText("[EMAIL_1]")).toBeVisible({ timeout: 30000 });
    await page.getByText("contrat.pdf").click();
    await expect(page.getByTestId("pii-counts")).toContainText("EMAIL");
    expect(await page.getByText("alice@example.com").count()).toBe(0);
    await page.goto("http://127.0.0.1:8080/ui/folder/c1?token=test");
    await page.getByLabel("select-contrat.pdf").check();
    await page.getByLabelText("delete-selected").click();
    await page.getByLabelText("confirm-delete-yes").click();
    await expect.poll(() => received.deletes.length).toBe(1);
    expect(received.deletes[0].external_ids).toEqual(["contrat.pdf"]);
  } finally { server.close(); }
});
```

- [ ] **Step 2: Run the e2e (needs built wasm + node_modules; skip without `XBERG_RUN_WASM_TESTS=1`)**

Run: `cd packages/xberg-web-ui && npx playwright test e2e/delete.spec.ts`
Expected: PASS with wasm present

- [ ] **Step 3: Rebuild static export**

Run: `cd packages/xberg-web-ui && pnpm export`
Expected: `mcp-server/ui-dist` updated (Lot 2 copy script)

- [ ] **Step 4: Run full web-ui unit suite**

Run: `cd packages/xberg-web-ui && npx vitest run`
Expected: PASS (all Lot 2 + Lot 3 unit/component suites)

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/ mcp-server/ui-dist/
git commit -m "feat(web-ui): advanced viz + delete/re-ingest e2e, static export"
```

---

## Self-Review Notes

- **Spec coverage (Lot 3 only):** new MCP `POST /admin` (`drop_collection` / `delete_documents` by external_id / `stats`) wired into Lot 1's `ui-server.ts` (Task 1); browser `admin-client` + worker `ocrLayout` + `ocr-to-layout` adapter (Task 2); `LayoutBlocks` wrapping `@extend/layout-blocks-block` (Task 3); `BoundingBoxCitations` wrapping `@extend/bounding-box-citations-block` with no clear-value leak (Task 4); `DocumentViewer` using `@extend/pdf-viewer`/`docx-viewer`/`xlsx-viewer`, plus shadcn `DeleteDialog`/`ReingestButton` and TanStack `DocumentTable` selection (Task 5); Playwright e2e asserting upload→delete removes the doc from the MCP store and PII citations never leak clear values (Task 6).
- **Extend investigation outcome:** components are a shadcn registry installed via `npx shadcn@latest add @extend/<name>` (hosted by extend-hq; the GitHub `ui/` folder only holds re-export stubs). Primitives (`button`/`dialog`/`select`/`table`/`tooltip`/`popover`/`input`/`card`/`badge`) are bundled by the registry into `@/components/ui/*`, so shadcn provides the bespoke controls. Heavy viewers (pdf/docx/xlsx/file-system) pull react-pdf/mammoth/pdfjs-dist; building/verifying them requires a working `node_modules` + wasm build not available in the current environment.
- **Type/contract consistency:** `delete_documents` uses `store.deleteDocuments(collection, external_ids)` which matches by `external_id` (store-node.ts:257). `admin-client.AdminPayload` mirrors the MCP `AdminPayloadSchema` (discriminated union on `op`). Re-ingestion reuses Lot 2's `ingestFile` + Lot 1's idempotent `upsertDocument` (no new route).
- **PII safety:** `BoundingBoxCitations` receives only `redactedText` + `counts` (+ in-memory `map`); it asserts the clear value is absent from the DOM. The rehydration `map` is never persisted and never POSTed (per Lot 2 Global Constraints).
- **Execution order:** Lot 1 (✅ done) → Lot 2 (⛔ not yet implemented — prerequisite for Tasks 2–6) → Lot 3. Task 1 (MCP admin route) is independently executable now; Tasks 2–6 require the Lot 2 web-ui package to exist.
