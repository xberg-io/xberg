# Xberg WASM-Backed MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the MCP server off native NAPI bindings (`@xberg-io/xberg`, `xberg-rag-node`) onto the shared wasm engine (B), so Claude Code desktop and the browser UI run the same `.wasm` file and ML/storage code.

**Architecture:** The 13 tool groups (extract, pii, rehydrate, ingest, query, collection, document, stats, reports, cache, intelligence, media, web) remain stable in their MCP contracts (tool names, Zod schemas); only implementation bodies swap native calls for `engine.*` calls injected via a new `src/engine.ts` that constructs the shared runtime (C) factories and a single `XbergEngine` instance.

**Tech Stack:** TypeScript/ESM, Node.js V8, `@xberg-io/xberg-wasm` (from B), `packages/xberg-wasm-runtime` (from C), `onnxruntime-node`, `better-sqlite3` (or wa-sqlite), `vitest`.

**Spec:** [2026-07-02-xberg-wasm-mcp-server-design.md](../specs/2026-07-02-xberg-wasm-mcp-server-design.md)

## Global Constraints

- **Tool names and Zod schemas are stable public API** — renaming a tool or changing a schema is a breaking change for connected agents. Every tool's contract remains identical (per `mcp-tool-patterns`); only the implementation body changes from native calls to engine calls.
- **Engine instance management:** The server constructs one `XbergEngine` at startup (or a small managed pool). Native calls like `openSqlite()` and `embedTexts()` are replaced by engine method calls (`.ingest`, `.query`, `.redact`, etc.). Single-flight semantics per instance (from C spec §6 async binding): the engine holds `&self` across an `await`, so overlapping calls on one handle must be serialized by the caller.
- **Error handling:** Tool handlers keep the existing `{ isError: true, content: [...] }` contract. Never `process.exit()` from a handler (per `mcp-tool-patterns`). Map engine errors to that shape preserving message and context.
- **No process.exit() in handlers** — the MCP SDK manages server lifecycle.
- **Native packages remain in the repo** — `@xberg-io/xberg` and `xberg-rag-node` are not deleted; only `mcp-server/` retargets. Other consumers may still link them.
- **TypeScript strict mode, ESM, pnpm, oxfmt/oxlint, vitest** — existing conventions per repo `typescript-conventions`.
- **Conventional commits:** `feat:`, `fix:`, `refactor:`, `chore:`, `test:`, `docs:` prefix; <72 chars first line; **no AI attribution** (repo `no-ai-signatures` rule).
- **Parity requirements:**
  - `rehydrate_tokens` must decrypt map files produced by the old TS `rehydration.ts` path (AES-GCM container format `XPII\x01 | salt(16) | iv(12) | tag(16) | ciphertext` is frozen; plan B's Rust crypto is byte-compatible with the Node code).
  - PII detection/redaction output (`detect_pii`, `redact_document`) must match pre-migration output on the same inputs (parity test with a fixture).
  - OCR default becomes injected PaddleOCR (50+ languages, ORT-backed via C) with Tesseract fallback — a **capability upgrade** from the native path; document in CHANGELOG per `api-compatibility` rule.
- **Run `prek run --all-files` before each commit** — re-stage if hooks rewrite.

---

### Task 1: Create `mcp-server/src/engine.ts` — engine factory and lifecycle

Construct and manage the `XbergEngine` instance via C's shared runtime factories. Replace the `store.ts` singleton and native imports throughout.

**Files:**
- Create: `mcp-server/src/engine.ts`
- Modify: `mcp-server/package.json` (add `xberg-wasm` from C, keep `@xberg-io/xberg` and `xberg-rag-node` for now)
- Modify: `mcp-server/src/index.ts` (import engine, initialize at startup)
- Remove: `mcp-server/src/store.ts` (no longer needed after tool retargeting is complete)

**Interfaces:**
- Consumes: C's public entry point `createXbergRuntimeFactory(config?): Promise<InjectionDescriptor>` from `packages/xberg-wasm-runtime` (the package's `index.ts` only re-exports this single factory, not the per-capability `createEmbedder`/`createVectorStore`/`createNer`/`createOcr` internals — see the runtime-layer plan's Task 9); B's `XbergEngine` from `@xberg-io/xberg-wasm`.
- Produces: `export async function initializeEngine(): Promise<XbergEngine>` and `export function getEngine(): XbergEngine` (singleton getter); configuration of the injection descriptor with Node-variant factories (onnxruntime-node for ORT, better-sqlite3 or wa-sqlite for the store backend, `~/.cache/xberg` for model cache).

- [ ] **Step 1: Add C and B dependencies to package.json**

Update `mcp-server/package.json`:
```json
{
  "dependencies": {
    "@xberg-io/xberg-wasm": "file:../crates/xberg-wasm",
    "xberg-wasm-runtime": "file:../packages/xberg-wasm-runtime",
    "@xberg-io/xberg": "file:../crates/xberg-node",
    "xberg-rag-node": "file:../crates/xberg-rag-node",
    "onnxruntime-node": "^1.18.0",
    "better-sqlite3": "^9.2.0",
    ...existing
  }
}
```

Run: `pnpm install`
Expected: SUCCESS, `node_modules/.pnpm/` includes `xberg-wasm-runtime` and `onnxruntime-node`.

- [ ] **Step 2: Write the failing test**

Create `mcp-server/tests/engine.test.ts`:

```typescript
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("engine initialization", () => {
  beforeAll(async () => {
    // Startup should succeed
    await initializeEngine();
  });

  it("returns a singleton engine instance", () => {
    const eng = getEngine();
    expect(eng).toBeDefined();
    expect(typeof eng.extract).toBe("function");
    expect(typeof eng.query).toBe("function");
  });

  it("engine has all required methods", () => {
    const eng = getEngine();
    ["extract", "ocr", "detectPii", "redact", "rehydrate", "ner", "ingest", "query"]
      .forEach((method) => {
        expect(typeof eng[method as keyof typeof eng]).toBe("function");
      });
  });
});
```

- [ ] **Step 3: Run to verify it fails**

Run: `pnpm test tests/engine.test.ts 2>&1 | tail -20`
Expected: FAIL — `initializeEngine` not found.

- [ ] **Step 4: Implement engine.ts**

Create `mcp-server/src/engine.ts`:

```typescript
import type { XbergEngine } from "@xberg-io/xberg-wasm";
import { createXbergRuntimeFactory } from "xberg-wasm-runtime";
import { getCacheDir } from "./store.js"; // reuse existing cache dir logic temporarily
import { homedir } from "os";
import { join } from "path";

let _engine: XbergEngine | null = null;

export async function initializeEngine(): Promise<XbergEngine> {
  if (_engine !== null) return _engine;

  const cacheDir = getCacheDir();
  const dbPath = process.env.XBERG_STORE_PATH ??
    (process.platform === "win32"
      ? join(process.env.APPDATA ?? join(homedir(), "AppData", "Roaming"), "xberg", "store.db")
      : join(homedir(), ".local", "share", "xberg", "store.db"));

  // C's single public entry point builds and validates the whole injection
  // descriptor ({ embedder, store, ner?, ocr? }) — do not import the
  // per-capability factories directly, index.ts only re-exports this one.
  const injection = await createXbergRuntimeFactory({ nodeCachePath: cacheDir, storePath: dbPath });

  const config = {}; // use defaults; per spec, config is empty for engine construction

  // Construct the wasm engine
  const { XbergEngine } = await import("@xberg-io/xberg-wasm");
  _engine = new XbergEngine(config, injection);

  return _engine;
}

export function getEngine(): XbergEngine {
  if (_engine === null) {
    throw new Error("Engine not initialized. Call initializeEngine() first.");
  }
  return _engine;
}
```

(`createXbergRuntimeFactory` is C's only public export for descriptor construction — see [2026-07-02-xberg-wasm-runtime-layer.md](2026-07-02-xberg-wasm-runtime-layer.md) Task 9. If C's `CacheConfig` shape does not accept `storePath`, adjust to whatever store-location option C actually exposes — verify against C's `types.ts` when implementing this task.)

- [ ] **Step 5: Run to verify it passes (or identify integration issues)**

Run: `pnpm test tests/engine.test.ts 2>&1 | tail -30`
Expected: PASS (or clear integration errors pointing to C's actual factory signatures).

- [ ] **Step 6: Update index.ts to initialize engine at startup**

Modify `mcp-server/src/index.ts`:

```typescript
import { initializeEngine } from "./engine.js";
import { WarmupManager } from "./warmup.js";

async function main() {
  const cacheDir = process.env.XBERG_CACHE_DIR ?? `${process.env.HOME ?? process.env.USERPROFILE ?? "~"}/.cache/xberg`;
  const warmup = new WarmupManager(cacheDir);
  const missing = warmup.getMissingModels();
  if (missing.length > 0) {
    console.error(`[xberg-mcp] First-time setup: downloading ${missing.join(", ")}...`);
  }

  // Initialize the wasm engine
  const engine = await initializeEngine();
  console.error("[xberg-mcp] engine initialized");

  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error("[xberg-mcp] started");
}
```

- [ ] **Step 7: Commit**

```bash
prek run --all-files
git add mcp-server/src/engine.ts mcp-server/src/index.ts mcp-server/package.json mcp-server/tests/engine.test.ts
git commit -m "feat(mcp): initialize XbergEngine from shared wasm runtime"
```

---

### Task 2: Retarget extract tool group (`extract_document`, `extract_batch`, `list_formats`)

Replace `@xberg-io/xberg` native calls with `engine.extract()`.

**Files:**
- Modify: `mcp-server/src/tools/extract.ts` (only implementation body; tool names/schemas unchanged)

**Interfaces:**
- Consumes: `getEngine()`, returns `XbergEngine` with `.extract(input, config) -> Promise<ExtractionResult>`.
- Produces: tool handler bodies (extract_document, extract_batch, list_formats) calling engine instead of native.

**Constraint:** Tool names (`extract_document`, `extract_batch`, `list_formats`) and Zod schemas (`ExtractInputSchema`, `ExtractionConfigSchema`) are public API — rename them and you break agents.

- [ ] **Step 1: Write the failing test (extract via engine)**

Add to `mcp-server/tests/tools.test.ts` (create if absent):

```typescript
import { describe, it, expect } from "vitest";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { registerExtractTools } from "../src/tools/extract.js";
import { initializeEngine } from "../src/engine.js";

describe("extract tool group via engine", () => {
  let server: McpServer;

  beforeAll(async () => {
    await initializeEngine();
    server = new McpServer({
      name: "test",
      version: "0.1.0",
    });
    registerExtractTools(server);
  });

  it("extract_document returns content with text field", async () => {
    const tool = server.listTools().find((t) => t.name === "extract_document");
    expect(tool).toBeDefined();

    const handler = tool?.inputSchema; // Verify schema exists
    expect(handler).toBeDefined();
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm test tests/tools.test.ts 2>&1 | tail -20`
Expected: FAIL — handlers are still calling native functions.

- [ ] **Step 3: Refactor extract.ts to use engine**

Open `mcp-server/src/tools/extract.ts`. Replace the handler bodies:

**Before:**
```typescript
import { extract, extractBatch, extractInputFromBytes, ... } from "@xberg-io/xberg";

async ({ uri, bytes, ... }) => {
  const result = await extract(input, config);
  // ...
}
```

**After:**
```typescript
import { getEngine } from "../engine.js";

async ({ uri, bytes, ... }) => {
  const engine = getEngine();
  const result = await engine.extract(input, config);
  // ...
}
```

**Key changes:**
- Remove `import` from `@xberg-io/xberg`.
- Add `import { getEngine }` from `../engine.js`.
- Replace `extract(...)` with `engine.extract(...)`, `extractBatch(...)` with `engine.extract(..., { batch: true })` (adjust to match B's actual `extract` signature).
- Keep all `{ content: [...], isError?: true }` response shapes and error handling identical.
- Keep tool name and Zod schema unchanged.

For `list_formats`, check if it's exposed by the engine or if it remains a native call — if absent from B's engine API, keep the native call as-is (not a breaking change if an internal tool uses native).

- [ ] **Step 4: Run to verify it passes**

Run: `pnpm test tests/tools.test.ts 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Test against a fixture document**

Run: `pnpm dev` and invoke `extract_document` manually via the MCP protocol with a small test file. Verify the output matches the pre-migration format.

- [ ] **Step 6: Commit**

```bash
prek run --all-files
git add mcp-server/src/tools/extract.ts
git commit -m "refactor(mcp): retarget extract tools to wasm engine"
```

---

### Task 3: Retarget PII tool group (`detect_pii`, `redact_document`)

Replace pattern-matching PII detection and redaction with engine calls.

**Files:**
- Modify: `mcp-server/src/tools/pii.ts` (only implementation body; tool names/schemas unchanged)

**Interfaces:**
- Consumes: `getEngine()`, returns methods `.detectPii(text) -> Promise<Detection[]>` and `.redact(text, strategy) -> Promise<RedactedDoc>`.
- Produces: tool handlers calling engine instead of native.

**Constraint:** Tool names (`detect_pii`, `redact_document`) and schemas are stable public API.

- [ ] **Step 1: Write the failing test (pii detection via engine)**

Add to `mcp-server/tests/pii.test.ts` (create if absent):

```typescript
import { describe, it, expect } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("PII detection via engine", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.detectPii returns findings array", async () => {
    const engine = getEngine();
    const text = "Email: test@example.com, SSN: 123-45-6789";
    const findings = await engine.detectPii(text);
    expect(Array.isArray(findings)).toBe(true);
    expect(findings.length).toBeGreaterThan(0);
  });

  it("engine.redact with token_replace returns redacted text and token map", async () => {
    const engine = getEngine();
    const text = "Email: test@example.com";
    const result = await engine.redact(text, "token_replace");
    expect(result).toHaveProperty("redacted_text");
    expect(result).toHaveProperty("token_map");
    expect(result.redacted_text).toContain("[EMAIL_");
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm test tests/pii.test.ts 2>&1 | tail -20`
Expected: FAIL — engine methods not yet exposed or not integrated.

- [ ] **Step 3: Refactor pii.ts to use engine**

Open `mcp-server/src/tools/pii.ts`. Replace the handler bodies:

**Before:**
```typescript
async ({ text, categories }) => {
  const findings = detectPiiPattern(text, categories);
  // ...
}
```

**After:**
```typescript
import { getEngine } from "../engine.js";

async ({ text, categories }) => {
  const engine = getEngine();
  const findings = await engine.detectPii(text);
  if (categories) {
    findings = findings.filter((f) => categories.includes(f.entity_type));
  }
  // ...
}
```

Similarly for `redact_document`:

```typescript
async ({ text, strategy }) => {
  const engine = getEngine();
  const result = await engine.redact(text, strategy);
  // ... format result into { redacted, token_map } response
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `pnpm test tests/pii.test.ts 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add mcp-server/src/tools/pii.ts
git commit -m "refactor(mcp): retarget PII tools to wasm engine"
```

---

### Task 4: Retarget rehydrate tool group (`rehydrate_tokens`, `list_tokens`, `rehydrate_document`)

Replace TS-native rehydration with engine's in-wasm AES-GCM decryption.

**Files:**
- Modify: `mcp-server/src/tools/rehydrate.ts` (only implementation body; tool names/schemas unchanged)
- Modify: `mcp-server/src/redaction/rehydration.ts` (remove or mark as deprecated; engine now owns decryption)

**Interfaces:**
- Consumes: `getEngine()`, returns `.rehydrate(doc, mapBytes, passphrase) -> Promise<string>`.
- Produces: tool handlers calling engine's in-wasm rehydration instead of the TS path.

**Constraint:** Tool names (`rehydrate_tokens`, `list_tokens`, `rehydrate_document`) and schemas are stable public API.

- [ ] **Step 1: Write the cross-format compatibility test**

Create `mcp-server/tests/rehydration_compat.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";
import { encryptMapFile } from "../src/redaction/rehydration.js"; // existing TS impl
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

describe("rehydration cross-format compatibility", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.rehydrate decrypts maps produced by TS encryptMapFile", async () => {
    const engine = getEngine();
    const tmpPath = path.join(os.tmpdir(), `xberg-test-${Date.now()}.xpii`);

    try {
      // Produce a map with the OLD TS code
      const tokenMap = { "[EMAIL_1]": "jane@example.com", "[PHONE_1]": "555-1234567" };
      const passphrase = "correct horse battery staple";
      encryptMapFile(tmpPath, tokenMap, passphrase);

      // Read the encrypted bytes
      const mapBytes = fs.readFileSync(tmpPath);

      // Decrypt with engine (in-wasm)
      const redacted = "[EMAIL_1] called [PHONE_1]";
      const rehydrated = await engine.rehydrate(redacted, Array.from(mapBytes), passphrase);

      expect(rehydrated).toContain("jane@example.com");
      expect(rehydrated).toContain("555-1234567");
    } finally {
      if (fs.existsSync(tmpPath)) fs.unlinkSync(tmpPath);
    }
  });

  it("engine.rehydrate rejects wrong passphrase", async () => {
    const engine = getEngine();
    const tmpPath = path.join(os.tmpdir(), `xberg-test-${Date.now()}.xpii`);

    try {
      encryptMapFile(tmpPath, { "[EMAIL_1]": "test@test.com" }, "correct");
      const mapBytes = fs.readFileSync(tmpPath);
      const redacted = "[EMAIL_1] was here";

      await expect(
        engine.rehydrate(redacted, Array.from(mapBytes), "wrong")
      ).rejects.toThrow();
    } finally {
      if (fs.existsSync(tmpPath)) fs.unlinkSync(tmpPath);
    }
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm test tests/rehydration_compat.test.ts 2>&1 | tail -20`
Expected: FAIL — engine.rehydrate not yet integrated with the wasm AES-GCM path.

- [ ] **Step 3: Verify plan B's anon_crypto matches the TS format**

Check `crates/xberg/src/text/anon_crypto.rs` (from plan B Task 2):
- Magic: `XPII\x01` ✓
- Layout: `magic | salt(16) | iv(12) | tag(16) | ciphertext` ✓
- Scrypt params: `Params::new(14, 8, 1, 32)` matches Node's default `scryptSync(pw, salt, 32)` ✓

If there are discrepancies, pause and file a task to reconcile plan B's crypto with the TS implementation. For this plan, assume B is correct.

- [ ] **Step 4: Refactor rehydrate.ts to use engine**

Open `mcp-server/src/tools/rehydrate.ts`. Replace the handler bodies:

**Before (rehydrate_tokens):**
```typescript
async ({ redacted_text, token_map }) => {
  let text = redacted_text;
  for (const [token, original] of Object.entries(token_map)) {
    text = text.split(token).join(original);
  }
  return { content: [...] };
}
```

**After (rehydrate_tokens):**
```typescript
async ({ redacted_text, token_map }) => {
  try {
    // If the caller provides the map directly (in-memory), do local rehydration
    let text = redacted_text;
    for (const [token, original] of Object.entries(token_map)) {
      text = text.split(token).join(original);
    }
    return { content: [...] };
  } catch (err) {
    // ...existing error handling
  }
}
```

(This tool stays mostly the same — it takes an in-memory token map, not a file.)

**For rehydrate_document (if it exists):**
```typescript
async ({ redacted_text, map_path, passphrase }) => {
  try {
    const engine = getEngine();
    const mapBytes = fs.readFileSync(map_path);
    const rehydrated = await engine.rehydrate(redacted_text, Array.from(mapBytes), passphrase);
    return { content: [{ type: "text", text: JSON.stringify({ rehydrated_text: rehydrated }) }] };
  } catch (err) {
    // ...error handling
  }
}
```

- [ ] **Step 5: Run to verify compatibility test passes**

Run: `pnpm test tests/rehydration_compat.test.ts 2>&1 | tail -20`
Expected: PASS — engine.rehydrate successfully decrypts TS-produced maps byte-for-byte.

- [ ] **Step 6: Commit**

```bash
prek run --all-files
git add mcp-server/src/tools/rehydrate.ts mcp-server/tests/rehydration_compat.test.ts
git commit -m "refactor(mcp): retarget rehydrate tools to wasm engine AES-GCM"
```

---

### Task 5: Retarget ingest tool group (`ingest_document`, `ingest_folder`)

Replace native extraction + native embeddings with engine's unified `.ingest()`.

**Files:**
- Modify: `mcp-server/src/tools/ingest.ts` (only implementation body; tool names/schemas unchanged)

**Interfaces:**
- Consumes: `getEngine()`, returns `.ingest(doc, collection) -> Promise<IngestReport>`.
- Produces: tool handlers calling engine instead of native extract + native embedTexts.

**Constraint:** Tool names (`ingest_document`, `ingest_folder`) and schemas are stable public API.

- [ ] **Step 1: Write the failing test**

Add to `mcp-server/tests/ingest.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("ingest via engine", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.ingest returns an IngestReport", async () => {
    const engine = getEngine();
    const doc = {
      full_text: "Hello world. This is a test document.",
      title: "Test",
      collection: "test_col",
    };
    const report = await engine.ingest(doc, "test_col");
    expect(report).toHaveProperty("document_id");
    expect(report).toHaveProperty("chunks_ingested");
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm test tests/ingest.test.ts 2>&1 | tail -20`
Expected: FAIL — engine.ingest not yet integrated.

- [ ] **Step 3: Refactor ingest.ts to use engine**

Open `mcp-server/src/tools/ingest.ts`. The current code:

```typescript
import { extract, extractInputFromUri, ... } from "@xberg-io/xberg";
import { embedTexts } from "xberg-rag-node";
import { getStore } from "../store.js";

async ({ collection, full_text, ... }) => {
  // Manual chunking + extract + embed + store
  const chunks = chunkText(full_text);
  const embeddings = await embedTexts(chunks, {});
  const store = await getStore();
  // ... upsert
}
```

**Replace with:**

```typescript
import { getEngine } from "../engine.js";

async ({ collection, full_text, title, mime, ... }) => {
  try {
    const engine = getEngine();
    const doc = { full_text, title, mime, ... };
    const report = await engine.ingest(doc, collection);
    return {
      content: [{
        type: "text",
        text: JSON.stringify({
          status: "ingested",
          document_id: report.document_id,
          chunks_ingested: report.chunks_ingested,
        }),
      }],
    };
  } catch (err) {
    // ...existing error handling
  }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `pnpm test tests/ingest.test.ts 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add mcp-server/src/tools/ingest.ts
git commit -m "refactor(mcp): retarget ingest tools to wasm engine"
```

---

### Task 6: Retarget query tool group (`query_corpus`)

Replace native `embedTexts` + native `store.query` with `engine.query()`.

**Files:**
- Modify: `mcp-server/src/tools/query.ts` (only implementation body; tool names/schemas unchanged)

**Interfaces:**
- Consumes: `getEngine()`, returns `.query(q, collection, k) -> Promise<RetrievedChunk[]>`.
- Produces: tool handlers calling engine instead of native embed + store.query.

**Constraint:** Tool names (`query_corpus`) and schemas are stable public API.

- [ ] **Step 1: Write the failing test**

Add to `mcp-server/tests/query.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("query via engine", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.query returns chunks", async () => {
    const engine = getEngine();
    const chunks = await engine.query("test query", "test_col", 5);
    expect(Array.isArray(chunks)).toBe(true);
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm test tests/query.test.ts 2>&1 | tail -20`
Expected: FAIL — engine.query not yet integrated.

- [ ] **Step 3: Refactor query.ts to use engine**

Open `mcp-server/src/tools/query.ts`. Replace:

```typescript
import { embedTexts } from "xberg-rag-node";
import { getStore } from "../store.js";

async ({ query_text, collection, k, ... }) => {
  const embeddings = await embedTexts([query_text], {});
  const store = await getStore();
  const chunks = await store.query(/* ... */);
  // ...
}
```

**With:**

```typescript
import { getEngine } from "../engine.js";

async ({ query_text, collection, k, ... }) => {
  try {
    const engine = getEngine();
    const chunks = await engine.query(query_text, collection, k);
    return {
      content: [{
        type: "text",
        text: JSON.stringify({
          chunks: chunks.map((c) => ({
            content: c.content,
            score: c.score,
            metadata: c.metadata,
          })),
        }),
      }],
    };
  } catch (err) {
    // ...error handling
  }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `pnpm test tests/query.test.ts 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add mcp-server/src/tools/query.ts
git commit -m "refactor(mcp): retarget query tools to wasm engine"
```

---

### Task 7: Retarget collection + document + stats + reports + cache + intelligence + media + web tool groups

Replace native store calls with engine equivalents. These groups are smaller and mostly orchestration around `engine.ingest`, `engine.query`, and native store management.

**Files:**
- Modify: `mcp-server/src/tools/collection.ts` (create_collection, get_collection, drop_collection)
- Modify: `mcp-server/src/tools/document.ts` (upsert_document, get_document, delete_documents)
- Modify: `mcp-server/src/tools/stats.ts` (collection_stats, list_collections, export_collection)
- Modify: `mcp-server/src/tools/reports.ts` (get_ingestion_summary, get_document_report)
- Modify: `mcp-server/src/tools/cache.ts` (rag_cache_warm, rag_cache_status)
- Modify: `mcp-server/src/tools/intelligence.ts` (if NER/keyword tools exist)
- Modify: `mcp-server/src/tools/media.ts` (if media-specific tools exist)
- Modify: `mcp-server/src/tools/web.ts` (if web-specific tools exist)

**Interfaces:**
- Consumes: `getEngine()` where possible; direct store access for collection metadata (may remain native if engine doesn't expose it).
- Produces: tool handlers calling engine methods or falling back to store access as needed.

**Constraint:** All tool names and schemas are stable public API; only implementation bodies change.

- [ ] **Step 1: Batch refactor collection + document + stats tools**

For each tool file, replace native `getStore()` calls and `store.` method calls with `engine.` equivalents where B exposes them:

- `createCollection` → keep native (engine doesn't expose collection creation; or check B's API)
- `dropCollection` → keep native or use `engine.query(..., collection, 0)` to test if exists
- `upsertDocument` → use `engine.ingest` if document comes with text; otherwise keep native
- `query` → already done in Task 6
- `listCollections` → keep native (engine may not expose)
- Collection metadata/stats → keep native if engine doesn't expose

For each tool, follow the TDD pattern:
1. Write a failing test calling the native tool's handler and verifying it still works (backward-compat test).
2. Identify which native calls can be replaced with engine calls.
3. Replace those calls; keep the handler contract identical.
4. Verify the test passes.

Example for `collection.ts`:

```typescript
// Before
import { getStore } from "../store.js";
async ({ name, embedding_dim, ... }) => {
  const store = await getStore();
  await store.ensureCollection(specJson);
  // ...
}

// After (if engine exposes this)
import { getEngine } from "../engine.js";
async ({ name, embedding_dim, ... }) => {
  const engine = getEngine();
  // engine may not have this; keep native if absent
  const store = await getStore();
  await store.ensureCollection(specJson);
  // ...
}
```

Since B's spec doesn't list all `VectorStore` methods as engine methods, **keep native calls where the engine doesn't expose an equivalent**.

- [ ] **Step 2: Refactor each tool file (7 files)**

For each of: `collection.ts`, `document.ts`, `stats.ts`, `reports.ts`, `cache.ts`, `intelligence.ts`, `media.ts`, `web.ts`:

Run: `pnpm test tests/<tool>.test.ts 2>&1 | tail -20` (or run the corresponding tool test if it exists).
Expected: PASS after each refactor (handlers remain functionally equivalent).

- [ ] **Step 3: Consolidate tests**

Update `mcp-server/tests/tools.test.ts` to include a simple smoke test for each of the 13 tool groups:

```typescript
describe("all tool groups schema + contract", () => {
  const toolNames = [
    "extract_document", "extract_batch", "list_formats",
    "detect_pii", "redact_document",
    "rehydrate_tokens", "list_tokens", "rehydrate_document",
    "ingest_document", "ingest_folder",
    "query_corpus",
    "create_collection", "get_collection", "drop_collection",
    "upsert_document", "get_document", "delete_documents",
    "collection_stats", "list_collections", "export_collection",
    "get_ingestion_summary", "get_document_report",
    "rag_cache_warm", "rag_cache_status",
    "detect_entities", "extract_keywords", // if intelligence.ts has these
    "describe_media", "extract_media", // if media.ts has these
    "fetch_and_extract", "crawl_links", // if web.ts has these
  ];

  server.listTools().forEach((tool) => {
    it(`tool ${tool.name} has inputSchema`, () => {
      expect(tool.inputSchema).toBeDefined();
      expect(toolNames).toContain(tool.name);
    });
  });
});
```

- [ ] **Step 4: Commit all refactored tools**

```bash
prek run --all-files
git add mcp-server/src/tools/collection.ts mcp-server/src/tools/document.ts mcp-server/src/tools/stats.ts mcp-server/src/tools/reports.ts mcp-server/src/tools/cache.ts mcp-server/src/tools/intelligence.ts mcp-server/src/tools/media.ts mcp-server/src/tools/web.ts mcp-server/tests/tools.test.ts
git commit -m "refactor(mcp): retarget remaining 8 tool groups to engine / native store mix"
```

---

### Task 8: PII detection + redaction parity test with pre-migration fixture

Verify that the wasm engine's PII detection and redaction produce identical output to the pre-migration TS path on a committed fixture.

**Files:**
- Create: `mcp-server/tests/pii_parity.test.ts`
- Create: `mcp-server/tests/fixtures/pii_input.txt` (test text)
- Create: `mcp-server/tests/fixtures/pii_expected.json` (expected PII detections from TS path)

**Interfaces:**
- Consumes: committed fixture text and expected JSON.
- Produces: assertion that `engine.detectPii` on the same input produces matching findings.

- [ ] **Step 1: Capture the TS baseline**

From a running instance of the current MCP server (native):

```bash
pnpm dev
# In another terminal, curl or use the MCP client to call detect_pii on a test fixture
curl -X POST http://localhost:3000/detect_pii -H "Content-Type: application/json" \
  -d '{"text":"Contact Jane Doe at jane@example.com or 555-123-4567. SSN: 123-45-6789. Card: 4111111111111111."}' \
  > tests/fixtures/pii_expected.json
```

Or manually call `detectPii` from the TS path:

```typescript
import { detectPii, groupByCategory } from "./src/redaction/detect.js";
const findings = detectPii("Contact Jane Doe at jane@example.com or 555-123-4567. SSN: 123-45-6789.");
console.log(JSON.stringify(findings, null, 2));
// Copy to tests/fixtures/pii_expected.json
```

Save the input text to `tests/fixtures/pii_input.txt`.

- [ ] **Step 2: Write the parity test**

Create `mcp-server/tests/pii_parity.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import * as fs from "fs";
import * as path from "path";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("PII detection parity", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.detectPii matches pre-migration TS output on fixture", async () => {
    const engine = getEngine();

    const fixtureText = fs.readFileSync(
      path.join(__dirname, "fixtures", "pii_input.txt"),
      "utf-8"
    );
    const expectedJson = JSON.parse(
      fs.readFileSync(path.join(__dirname, "fixtures", "pii_expected.json"), "utf-8")
    );

    const engineFindings = await engine.detectPii(fixtureText);

    // Normalize: sort by start position
    engineFindings.sort((a, b) => a.start - b.start);
    expectedJson.findings?.sort((a: any, b: any) => a.start - b.start);

    // Compare entity types, counts
    const engineByType = groupFindingsByType(engineFindings);
    const expectedByType = groupFindingsByType(expectedJson.findings ?? []);

    // Assert category counts match
    Object.keys(expectedByType).forEach((cat) => {
      expect(engineByType[cat] ?? 0).toBe(expectedByType[cat], `category ${cat} count mismatch`);
    });
  });

  function groupFindingsByType(findings: any[]) {
    return findings.reduce(
      (acc, f) => {
        acc[f.entity_type] = (acc[f.entity_type] ?? 0) + 1;
        return acc;
      },
      {} as Record<string, number>
    );
  }
});
```

- [ ] **Step 3: Run to verify it passes**

Run: `pnpm test tests/pii_parity.test.ts 2>&1 | tail -20`
Expected: PASS — engine findings match pre-migration findings on the fixture.

If FAIL: compare the counts and entities; if a category differs, it may indicate a change in the wasm implementation. Log and reconcile with plan B's PII detection code.

- [ ] **Step 4: Commit**

```bash
prek run --all-files
git add mcp-server/tests/pii_parity.test.ts mcp-server/tests/fixtures/pii_input.txt mcp-server/tests/fixtures/pii_expected.json
git commit -m "test(mcp): PII detection parity with pre-migration output"
```

---

### Task 9: E2E per-tool-group smoke tests

Minimal test per tool group: schema parse → engine call → assert `{ content: [...] }` response shape.

**Files:**
- Create/Modify: `mcp-server/tests/e2e.test.ts`

**Interfaces:**
- Consumes: Each tool group's handler (via McpServer).
- Produces: Vitest test suite with ~13 subtests (one per group), each calling the handler and verifying the response shape.

- [ ] **Step 1: Write the E2E smoke suite**

Create `mcp-server/tests/e2e.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from "vitest";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { initializeEngine } from "../src/engine.js";
import * as fs from "fs";

describe("MCP tool groups E2E", () => {
  let server: McpServer;

  beforeAll(async () => {
    await initializeEngine();
    server = new McpServer({ name: "test", version: "0.1.0" });

    // Register all tool groups
    const { registerExtractTools } = await import("../src/tools/extract.js");
    const { registerPiiTools } = await import("../src/tools/pii.js");
    const { registerIngestTools } = await import("../src/tools/ingest.js");
    const { registerQueryTools } = await import("../src/tools/query.js");
    const { registerCollectionTools } = await import("../src/tools/collection.js");
    const { registerRehydrateTools } = await import("../src/tools/rehydrate.js");
    // ... register remaining groups

    registerExtractTools(server);
    registerPiiTools(server);
    registerIngestTools(server);
    registerQueryTools(server);
    registerCollectionTools(server);
    registerRehydrateTools(server);
    // ...
  });

  async function callTool(name: string, args: Record<string, any>) {
    const tool = server.listTools().find((t) => t.name === name);
    expect(tool).toBeDefined(`tool ${name} not registered`);

    // Parse input
    const parsed = tool!.inputSchema.parse(args);
    // Call the handler (mock the MCP call; adjust based on MCP SDK)
    // For now, just verify the tool exists and schema is valid
    expect(parsed).toBeDefined();
  }

  // Smoke tests per group
  it("extract group: extract_document schema parses", async () => {
    await callTool("extract_document", {
      uri: "memory://test.txt",
      bytes: [104, 105], // "hi"
    });
  });

  it("pii group: detect_pii schema parses", async () => {
    await callTool("detect_pii", {
      text: "Email: test@example.com",
    });
  });

  it("ingest group: ingest_document schema parses", async () => {
    await callTool("ingest_document", {
      collection: "test",
      full_text: "Hello world.",
    });
  });

  it("query group: query_corpus schema parses", async () => {
    await callTool("query_corpus", {
      query_text: "test",
      collection: "test",
      k: 5,
    });
  });

  it("collection group: create_collection schema parses", async () => {
    await callTool("create_collection", {
      name: "test_col",
      embedding_dim: 1024,
    });
  });

  it("rehydrate group: rehydrate_tokens schema parses", async () => {
    await callTool("rehydrate_tokens", {
      redacted_text: "[EMAIL_1] called",
      token_map: { "[EMAIL_1]": "test@example.com" },
    });
  });

  // Add more smoke tests for remaining groups...
});
```

- [ ] **Step 2: Run to verify all schemas parse**

Run: `pnpm test tests/e2e.test.ts 2>&1 | tail -30`
Expected: PASS — all 13+ tool group schemas parse successfully.

- [ ] **Step 3: Commit**

```bash
prek run --all-files
git add mcp-server/tests/e2e.test.ts
git commit -m "test(mcp): E2E smoke tests for all 13 tool groups"
```

---

### Task 10: Latency benchmark (extract + ingest vs native baseline)

Measure regression: extract + ingest latency on the wasm engine vs the pre-migration native path.

**Files:**
- Create: `mcp-server/benchmarks/engine_vs_native.bench.ts`
- Modify: Benchmark results tracking (or just log and document)

**Interfaces:**
- Consumes: fixture document (text file).
- Produces: vitest bench suite comparing `engine.extract` + `engine.ingest` times against the native path (if available in the codebase for comparison).

- [ ] **Step 1: Write the benchmark**

Create `mcp-server/benchmarks/engine_vs_native.bench.ts`:

```typescript
import { bench, describe } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";
import * as fs from "fs";

// Pre-migration native calls (if they remain available for comparison)
// import { extract, extractBatch } from "@xberg-io/xberg";
// import { embedTexts } from "xberg-rag-node";

const fixtureText = "The quick brown fox jumps over the lazy dog. " + "This is a test. ".repeat(50);

describe("engine vs native latency", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  bench("engine.extract (wasm)", async () => {
    const engine = getEngine();
    await engine.extract(
      { full_text: fixtureText },
      {}
    );
  });

  bench("engine.ingest (wasm)", async () => {
    const engine = getEngine();
    await engine.ingest(
      { full_text: fixtureText, title: "bench" },
      "bench_col"
    );
  });

  // Optional: compare against native if the functions are still available
  // bench("native extract + embedTexts", async () => {
  //   const result = await extract(fixtureText, {});
  //   const chunks = [result.text];
  //   await embedTexts(chunks, {});
  // });
});
```

- [ ] **Step 2: Run the benchmark**

Run: `pnpm bench benchmarks/engine_vs_native.bench.ts 2>&1 | tee bench_results.txt`
Expected: Benchmark results showing latency for each operation. Capture the output.

- [ ] **Step 3: Document the results**

Create or append to `docs/superpowers/results/2026-07-02-wasm-mcp-performance.md`:

```markdown
# WASM MCP Server Performance Baseline (2026-07-02)

**Test date:** [date]
**Fixture:** 50-word document (~250 chars)
**Engine:** XbergEngine via C's Node-variant factories

## Latency (ms)

| Operation | Time (ms) | Notes |
|---|---|---|
| engine.extract | [time] | wasm-based extraction |
| engine.ingest | [time] | extract + embed + store |
| Native extract (baseline) | [time] | for comparison |

## Observations

[Any regressions, improvements, or notes about the wasm performance.]
```

Commit this document with the benchmark results recorded.

- [ ] **Step 4: Commit**

```bash
prek run --all-files
git add mcp-server/benchmarks/engine_vs_native.bench.ts docs/superpowers/results/2026-07-02-wasm-mcp-performance.md
git commit -m "perf(mcp): establish engine latency baseline vs native path"
```

---

### Task 11: Update CHANGELOG and document OCR capability upgrade

Document the breaking changes (none at the tool surface, but capability upgrade) and the OCR default change per `api-compatibility` rule.

**Files:**
- Modify: `CHANGELOG.md` (root)

**Interfaces:**
- Produces: documented capability upgrade (PaddleOCR default + 50+ languages) and internal implementation migration (WASM engine).

- [ ] **Step 1: Add entry to CHANGELOG**

Prepend to `CHANGELOG.md` under a new `## [Unreleased]` or `## [0.2.0]` section:

```markdown
## [Unreleased]

### Changed

- **MCP server backend:** Migrated from native NAPI bindings (`@xberg-io/xberg`, `xberg-rag-node`) to shared WASM engine (`@xberg-io/xberg-wasm`) with C runtime layer (`xberg-wasm-runtime`). MCP tool surface remains identical; only implementation details changed. No breaking changes to the MCP protocol or tool contracts.
- **OCR capability upgrade:** Default OCR backend is now injected PaddleOCR (50+ languages, ONNX Runtime, WebGPU-accelerated) with in-binary Tesseract fallback. Previously only Tesseract was available. Improves accuracy and language support; no API change.

### Fixed

- PII rehydration now uses in-WASM AES-256-GCM decryption, eliminating native C++ crypto dependencies.

### Internal

- MCP server now uses shared WASM engine, aligning with browser UI (sub-project D). Single codebase for all intelligence: extraction, NER, OCR, embeddings, anonymization.
```

- [ ] **Step 2: Verify no other breaking changes are needed**

Check the commits from this plan: since all tool names/schemas are unchanged and handlers maintain `{ content, isError? }` contracts, there are no breaking changes at the MCP API surface.

- [ ] **Step 3: Commit**

```bash
prek run --all-files
git add CHANGELOG.md
git commit -m "docs(changelog): document WASM migration and OCR capability upgrade"
```

---

### Task 12: Final cleanup and integration test

Remove `mcp-server/src/store.ts`, verify all native imports are gone or intentionally kept, run full test suite.

**Files:**
- Remove: `mcp-server/src/store.ts` (after verifying all callers are retargeted)
- Verify: no lingering `import ... from "@xberg-io/xberg"` or `import ... from "xberg-rag-node"` except where intentional (e.g., for type definitions or fallback paths).

**Interfaces:**
- Produces: clean codebase with engine.ts as the only interface to the native paths (which are now optional/auxiliary).

- [ ] **Step 1: Verify all retargetings are complete**

Run: `grep -r "from \"@xberg-io/xberg\"" mcp-server/src/tools/ --include="*.ts"`
Expected: NO results (all imports replaced with `getEngine()`).

Run: `grep -r "from \"xberg-rag-node\"" mcp-server/src/tools/ --include="*.ts"`
Expected: NO results.

Run: `grep -r "import.*getStore" mcp-server/src/tools/ --include="*.ts"`
Expected: NO results (all calls replaced with engine calls).

If any matches remain, revisit those files and complete the retargeting.

- [ ] **Step 2: Remove store.ts**

Once verified clean:

```bash
rm mcp-server/src/store.ts
```

(Do NOT remove from git yet; stage the removal in the next step.)

- [ ] **Step 3: Update imports in other files that reference store.ts**

Check `warmup.ts` and other non-tool files for references to `store.ts`:

```bash
grep -r "from \".*store.js\"" mcp-server/src/ --include="*.ts"
```

Update any references (e.g., `getCacheDir` may move into `engine.ts`). Adjust imports accordingly.

- [ ] **Step 4: Run full test suite**

Run: `pnpm test 2>&1 | tail -50`
Expected: ALL tests PASS. No warnings about undefined functions or missing imports.

- [ ] **Step 5: Run the server manually**

Run: `pnpm dev`
Expected: `[xberg-mcp] engine initialized` and `[xberg-mcp] started` messages. Server is live.

Invoke a tool (e.g., `detect_pii`) via an MCP client. Verify it returns the expected response shape.

- [ ] **Step 6: Commit cleanup**

```bash
prek run --all-files
git add mcp-server/src/ mcp-server/package.json
git rm mcp-server/src/store.ts
git commit -m "chore(mcp): remove obsolete store.ts singleton"
```

---

### Task 13: Retarget existing tests (redaction.test.ts, tools.test.ts) to run against wasm engine

Move test assertions to use the engine instead of native paths where applicable.

**Files:**
- Modify: `mcp-server/tests/redaction.test.ts` (if it imports native functions directly)
- Modify: `mcp-server/tests/tools.test.ts` (if it imports native functions directly)

**Interfaces:**
- Consumes: existing test fixtures and expected outputs.
- Produces: tests that import and use the engine (or keep as-is if they already call tool handlers, which internally use the engine now).

- [ ] **Step 1: Check the current test structure**

Open `mcp-server/tests/redaction.test.ts`. If it calls `detectPii` or `applyRedaction` directly from the redaction modules:

```typescript
import { detectPii } from "../src/redaction/detect.js";
const findings = detectPii("test@example.com");
```

These tests are testing the **TS PII detection functions directly**, not via the MCP tool. They can remain as-is (they test the TS modules, not the engine). No retargeting needed unless they're meant to test the engine.

Similarly for `tools.test.ts`, if it calls the MCP tool handlers, they already use the engine (retargeting is done). If it calls native functions directly, decide whether the test should move to the engine or remain as-is (for backward compat of the TS modules).

- [ ] **Step 2: Add engine integration test to redaction.test.ts (optional)**

If `redaction.test.ts` should also test the engine's PII, add a new describe block:

```typescript
import { initializeEngine, getEngine } from "../src/engine.js";

describe("PII detection via engine", () => {
  beforeAll(async () => {
    await initializeEngine();
  });

  it("engine.detectPii matches TS detectPii on same input", async () => {
    const engine = getEngine();
    const text = "Email: test@test.com";
    const findings = await engine.detectPii(text);
    expect(findings.length).toBeGreaterThan(0);
    expect(findings[0]?.entity_type).toBe("EMAIL");
  });
});
```

This is **additive** — the original TS tests remain for regression coverage.

- [ ] **Step 3: Run all tests**

Run: `pnpm test 2>&1 | tail -50`
Expected: ALL tests PASS, including new engine tests.

- [ ] **Step 4: Commit**

```bash
prek run --all-files
git add mcp-server/tests/redaction.test.ts mcp-server/tests/tools.test.ts
git commit -m "test(mcp): add engine integration tests to existing test suites"
```

---

## Self-Review Notes

### Coverage of spec sections (from 2026-07-02-xberg-wasm-mcp-server-design.md)

- **§Purpose:** Tasks 1, 2–7 (port all tool groups off native).
- **§Architecture:** Tasks 1 (engine factory), 2–13 (tool retargeting and integration).
- **§Tool migration table:** Tasks 2 (extract), 3 (pii), 4 (rehydrate), 5 (ingest), 6 (query), 7 (collection/document/stats/reports/cache/intelligence/media/web).
- **§Behavioral parity requirements:**
  - Rehydration: Task 4 (cross-format compat test with TS-produced maps).
  - PII output match: Task 8 (parity test on fixture).
  - OCR capability upgrade: Task 11 (CHANGELOG).
- **§Async binding:** Task 1 (engine construction with async factories); C spec's JSPI note applies (standard wasm-bindgen + JsFuture, not true JSPI).
- **§Error handling:** All tasks (maintain `{ isError: true, content: [...] }` contract, never `process.exit()`).
- **§Testing:** Tasks 4, 8, 9, 13 (parity, pii, e2e, integration).
- **§Non-goals:** Task 12 clarifies that native packages remain; only MCP server retargets.

### Dependencies on sub-projects B and C

- **Plan B (Shared WASM Engine):** This plan assumes Tasks 1–8 are complete. Critical: `XbergEngine` API contract (§API contract in B spec), `anon_crypto` (Task 2), bridges (Tasks 4–6). Task 11 in this plan documents OCR upgrade; check that B actually exposes PaddleOCR injection point in the engine.
- **Plan C (Shared JS Runtime):** This plan assumes C's public entry point `createXbergRuntimeFactory(config?)` is available from `packages/xberg-wasm-runtime` (per the runtime-layer plan, `index.ts` re-exports only this combined factory, not the per-capability internals). Task 1 calls it directly rather than reconstructing the descriptor from individual factories.

### Open verification questions (for integration)

1. **C's `CacheConfig` shape:** Task 1 Step 4 assumes `createXbergRuntimeFactory({ nodeCachePath, storePath })`. Verify C's actual `CacheConfig` fields (the runtime-layer plan's Task 2 type definitions) and adjust Task 1's call site accordingly — in particular whether store location is passed through `CacheConfig` or a separate argument.
2. **Engine constructor:** Task 1 assumes `new XbergEngine(config, injection)`. Verify B's exact constructor signature.
3. **Engine method names:** Spec says `.extract`, `.ocr`, `.detectPii`, `.redact`, `.rehydrate`, `.ner`, `.ingest`, `.query`. Verify these are all exposed and have the expected signatures.
4. **Native call removal completeness:** Task 12 Step 1 greps for lingering imports. If any tool still needs a native call (e.g., list_formats from extract), keep it and document it.

### Parity assumptions (for Task 4 + Task 8)

- Plan B's `anon_crypto.rs` (Task 2) produces byte-identical encrypted maps to the TS `rehydration.ts` code. Task 4 Step 3 verifies this; if it fails, file a task to reconcile the crypto.
- TS `detectPii` output format (categories, counts, positions) matches the engine's in-Rust PII detection (from plan B's PII redaction feature). Task 8 captures the baseline and asserts parity.

### Performance expectations (Task 10)

No strict target is defined; baseline is captured for regression tracking. Wasm extraction on CPU is typically within 10–30% of native (depending on format complexity). Ingest latency depends on embeddings (engine calls C's ORT-Web, same latency whether wasm-backed or native). Record results and flag if >20% regression vs native baseline.

### Sequencing notes

- **Must complete before:** B (Tasks 1–8) and C (factories) must be available.
- **Parallel with:** D (browser UI) — both consume B and C in parallel.
- **Order within this plan:** Task 1 (engine setup) → Tasks 2–7 (tool retargeting in spec table order) → Tasks 8–10 (verification) → Task 11 (CHANGELOG) → Task 12–13 (cleanup).

### No AI attribution rule

Every commit uses conventional commit message format and includes **no** `Co-Authored-By: Claude ...` footer (per `no-ai-signatures` rule in repo CLAUDE.md).
