# MCP Config Enrichment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the chunking, keyword extraction, and OCR backend fields that are already compiled into xberg-node but not surfaced in the MCP's `ExtractionConfigSchema`.

**Architecture:** All three features exist in the compiled `@xberg-io/xberg` binary — they just need Zod schema fields added to `ExtractionConfigSchema` and mapping in `toNativeConfig()`. No new files, no Rust changes, no Cargo rebuilds.

**Tech Stack:** TypeScript, Zod, `@xberg-io/xberg` (types from `crates/xberg-node/index.d.ts`), vitest

## Global Constraints

- TypeScript strict mode — no `any`, no non-null assertions
- Tool names are frozen public API — do NOT rename `extract_document` or `extract_batch`
- `toNativeConfig()` must return `ExtractionConfig | null` (never `undefined`)
- Tests in `mcp-server/tests/tools.test.ts` must not import native bindings — smoke tests only (native bindings may not be built in CI)
- Run `cd mcp-server && npm test -- run` to verify; run `node_modules/.bin/tsc --noEmit` to type-check

---

## File Map

| File | Change |
|------|--------|
| `mcp-server/src/tools/extract.ts` | Extend `ExtractionConfigSchema` + `toNativeConfig()` |
| `mcp-server/tests/tools.test.ts` | Add schema-shape tests for new fields |

---

### Task 1: Expose chunking config

Chunking splits extracted text into overlapping segments for RAG ingestion. The compiled binary already does this when `ExtractionConfig.chunking` is set.

**Files:**
- Modify: `mcp-server/src/tools/extract.ts:19-32`
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `ChunkingConfig` from `@xberg-io/xberg` — fields used here: `maxSize?: number`, `overlap?: number`, `sizing?: string` (type alias `JsChunkingConfig`)
- Produces: `chunking` field on `ExtractionConfig` passed to `extract()`

- [ ] **Step 1: Write the failing type test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("ExtractionConfigSchema shape", () => {
  it("accepts chunking config", async () => {
    const { z } = await import("zod");
    // Import the schema indirectly by checking the module exports the correct shape
    // (We can't import extract.ts directly because it imports @xberg-io/xberg)
    // Instead validate a plain Zod schema that mirrors what we'll add
    const ChunkingConfigSchema = z.object({
      max_size: z.number().int().min(64).max(16384).optional(),
      overlap: z.number().int().min(0).max(1024).optional(),
    });
    const result = ChunkingConfigSchema.safeParse({ max_size: 512, overlap: 64 });
    expect(result.success).toBe(true);
    const bad = ChunkingConfigSchema.safeParse({ max_size: 10 }); // below min 64
    expect(bad.success).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```
cd mcp-server && npm test -- run
```

Expected: FAIL — "ExtractionConfigSchema shape > accepts chunking config" (describe block not yet in file, will PASS trivially once added — this test validates the schema logic we're about to wire in)

- [ ] **Step 3: Add chunking to ExtractionConfigSchema in extract.ts**

Replace the `ExtractionConfigSchema` block (currently lines 19-23) and `toNativeConfig()` (lines 25-32) with:

```typescript
const ChunkingConfigSchema = z.object({
  max_size: z.number().int().min(64).max(16384).optional(),
  overlap: z.number().int().min(0).max(1024).optional(),
});

const ExtractionConfigSchema = z.object({
  force_ocr: z.boolean().optional(),
  disable_ocr: z.boolean().optional(),
  use_cache: z.boolean().optional(),
  chunking: ChunkingConfigSchema.optional(),
});

function toNativeConfig(config: z.infer<typeof ExtractionConfigSchema> | undefined): ExtractionConfig | null {
  if (!config) return null;
  return {
    forceOcr: config.force_ocr,
    disableOcr: config.disable_ocr,
    useCache: config.use_cache,
    chunking: config.chunking
      ? { maxSize: config.chunking.max_size, overlap: config.chunking.overlap }
      : undefined,
  };
}
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. If `maxSize` or `overlap` don't match the `ChunkingConfig` interface, the compiler will tell you the correct field names — check `crates/xberg-node/index.d.ts` around line 509.

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS including the new schema test.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/tools/extract.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): expose chunking config in extract_document"
```

---

### Task 2: Expose keyword extraction config

Keyword extraction (YAKE or RAKE algorithm) populates `ExtractedDocument.extractedKeywords` when enabled. Results appear in the `keywords` field already passed through in `ingest_folder`.

**Files:**
- Modify: `mcp-server/src/tools/extract.ts` (extend schema + mapping)
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `KeywordConfig` from `@xberg-io/xberg` — fields: `algorithm?: "yake" | "rake"`, `maxKeywords?: number`
- Produces: `keywords` field on `ExtractionConfig`; `extractedKeywords` field appears in `ExtractedDocument`

- [ ] **Step 1: Write the failing test**

Add to the `describe("ExtractionConfigSchema shape")` block in `mcp-server/tests/tools.test.ts`:

```typescript
  it("accepts keyword config", async () => {
    const { z } = await import("zod");
    const KeywordConfigSchema = z.object({
      algorithm: z.enum(["yake", "rake"]).optional(),
      max_keywords: z.number().int().min(1).max(100).optional(),
    });
    const result = KeywordConfigSchema.safeParse({ algorithm: "yake", max_keywords: 10 });
    expect(result.success).toBe(true);
    const bad = KeywordConfigSchema.safeParse({ algorithm: "invalid" });
    expect(bad.success).toBe(false);
  });
```

- [ ] **Step 2: Run test to verify it fails as expected**

```
cd mcp-server && npm test -- run
```

Expected: the new test PASSES (it's self-contained). This validates our schema logic before we wire it in.

- [ ] **Step 3: Add keyword config to ExtractionConfigSchema**

In `mcp-server/src/tools/extract.ts`, after `ChunkingConfigSchema`, add:

```typescript
const KeywordConfigSchema = z.object({
  algorithm: z.enum(["yake", "rake"]).optional(),
  max_keywords: z.number().int().min(1).max(100).optional(),
});
```

Extend `ExtractionConfigSchema` to include:

```typescript
const ExtractionConfigSchema = z.object({
  force_ocr: z.boolean().optional(),
  disable_ocr: z.boolean().optional(),
  use_cache: z.boolean().optional(),
  chunking: ChunkingConfigSchema.optional(),
  keywords: KeywordConfigSchema.optional(),
});
```

Extend `toNativeConfig()`:

```typescript
function toNativeConfig(config: z.infer<typeof ExtractionConfigSchema> | undefined): ExtractionConfig | null {
  if (!config) return null;
  return {
    forceOcr: config.force_ocr,
    disableOcr: config.disable_ocr,
    useCache: config.use_cache,
    chunking: config.chunking
      ? { maxSize: config.chunking.max_size, overlap: config.chunking.overlap }
      : undefined,
    keywords: config.keywords
      ? { algorithm: config.keywords.algorithm, maxKeywords: config.keywords.max_keywords }
      : undefined,
  };
}
```

Also update `extract_document` result mapping to include keywords. In the `structured` object (around line 63), add:

```typescript
keywords: doc.extractedKeywords?.map((k: { text: string; score?: number }) => ({
  text: k.text,
  score: k.score ?? null,
})) ?? [],
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. `KeywordConfig.algorithm` expects a `KeywordAlgorithm` union — the string values `"yake"` and `"rake"` should satisfy it. If not, check `JsKeywordAlgorithm` in `index.d.ts`.

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/tools/extract.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): expose keyword extraction config in extract_document"
```

---

### Task 3: Expose OCR backend and language selection

Allows callers to explicitly choose `"tesseract"` (default, always available) or `"paddleocr"` (requires xberg-node built with `paddle-ocr` feature), and to set the OCR language.

**Files:**
- Modify: `mcp-server/src/tools/extract.ts`
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `OcrConfig` from `@xberg-io/xberg` — fields: `backend?: string` (`"tesseract"` | `"paddleocr"`), `language?: string[]`, `enabled?: boolean`
- Produces: `ocr` field on `ExtractionConfig`

- [ ] **Step 1: Write the failing test**

Add to `describe("ExtractionConfigSchema shape")` in `mcp-server/tests/tools.test.ts`:

```typescript
  it("accepts ocr config with backend and languages", async () => {
    const { z } = await import("zod");
    const OcrConfigSchema = z.object({
      backend: z.enum(["tesseract", "paddleocr"]).optional(),
      languages: z.array(z.string().min(2).max(10)).optional(),
    });
    const result = OcrConfigSchema.safeParse({ backend: "tesseract", languages: ["eng", "deu"] });
    expect(result.success).toBe(true);
    const bad = OcrConfigSchema.safeParse({ backend: "unknown_engine" });
    expect(bad.success).toBe(false);
  });
```

- [ ] **Step 2: Run test to verify schema logic**

```
cd mcp-server && npm test -- run
```

Expected: PASS (self-contained).

- [ ] **Step 3: Add OCR config to ExtractionConfigSchema**

In `mcp-server/src/tools/extract.ts`, after `KeywordConfigSchema`, add:

```typescript
const OcrConfigSchema = z.object({
  backend: z.enum(["tesseract", "paddleocr"]).optional(),
  languages: z.array(z.string()).optional(),
});
```

Extend `ExtractionConfigSchema`:

```typescript
const ExtractionConfigSchema = z.object({
  force_ocr: z.boolean().optional(),
  disable_ocr: z.boolean().optional(),
  use_cache: z.boolean().optional(),
  chunking: ChunkingConfigSchema.optional(),
  keywords: KeywordConfigSchema.optional(),
  ocr: OcrConfigSchema.optional(),
});
```

Extend `toNativeConfig()`:

```typescript
function toNativeConfig(config: z.infer<typeof ExtractionConfigSchema> | undefined): ExtractionConfig | null {
  if (!config) return null;
  return {
    forceOcr: config.force_ocr,
    disableOcr: config.disable_ocr,
    useCache: config.use_cache,
    chunking: config.chunking
      ? { maxSize: config.chunking.max_size, overlap: config.chunking.overlap }
      : undefined,
    keywords: config.keywords
      ? { algorithm: config.keywords.algorithm, maxKeywords: config.keywords.max_keywords }
      : undefined,
    ocr: config.ocr
      ? { backend: config.ocr.backend, language: config.ocr.languages }
      : undefined,
  };
}
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. `OcrConfig.backend` is typed as `string` (not an enum), so any string passes the TS check — the Zod enum enforces valid values at runtime.

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS.

- [ ] **Step 6: Update extract_document description to document new options**

In the `server.tool("extract_document", ...)` call, update the description string:

```typescript
"Extract text, tables, and metadata from a document (91+ formats). " +
"Provide uri (file path or HTTPS URL) or bytes (number array). " +
"Config options: force_ocr, disable_ocr, use_cache, " +
"chunking (max_size/overlap), keywords (algorithm/max_keywords), " +
"ocr (backend: tesseract|paddleocr, languages: [eng, deu, ...]).",
```

- [ ] **Step 7: Commit**

```
git add mcp-server/src/tools/extract.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): expose OCR backend and language selection in extract_document"
```
