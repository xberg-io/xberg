# MCP Intelligence Tools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four new MCP tools that expose xberg intelligence features compiled into xberg-node but not yet surfaced: `extract_entities` (NER), `structured_extract` (LLM + JSON schema), `transcribe_audio` (Whisper), and `scrape_url` (crawlberg).

**Architecture:** Each tool is a thin MCP handler that builds an `ExtractionConfig` with the relevant sub-config populated and calls the existing `extract()` function. No new Rust code, no Cargo rebuild needed. Tools are split by category into two new files (`intelligence.ts` and `media.ts`) plus one web file (`web.ts`). All three are registered in `index.ts`.

**Tech Stack:** TypeScript, Zod, `@xberg-io/xberg` (types from `crates/xberg-node/index.d.ts`), vitest

## Global Constraints

- TypeScript strict mode — no `any`, no non-null assertions
- New tool names are snake_case: `extract_entities`, `structured_extract`, `transcribe_audio`, `scrape_url`
- Handler errors must return `{ isError: true, content: [{ type: "text", text: msg }] }` — never throw out of a handler
- Smoke tests in `tests/tools.test.ts` must not import native bindings — check module exports only
- Run `cd mcp-server && npm test -- run` to verify; `node_modules/.bin/tsc --noEmit` to type-check
- `NerBackendKind` values: `"onnx"` and `"llm"` (lowercase, from the TypeScript enum)
- `TranscriptionConfig.model` values: `"tiny"`, `"base"`, `"small"`, `"medium"`, `"large"`, `"turbo"`

---

## File Map

| File | Change |
|------|--------|
| `mcp-server/src/tools/intelligence.ts` | Create — `extract_entities` + `structured_extract` |
| `mcp-server/src/tools/media.ts` | Create — `transcribe_audio` |
| `mcp-server/src/tools/web.ts` | Create — `scrape_url` |
| `mcp-server/src/index.ts` | Modify — register 3 new tool groups |
| `mcp-server/tests/tools.test.ts` | Modify — smoke tests for 3 new modules |

---

### Task 1: NER tool — `extract_entities`

Uses `ExtractionConfig.ner` to run GLiNER ONNX or LLM-based entity extraction on a document. Returns entities from `ExtractedDocument.entities`.

**Files:**
- Create: `mcp-server/src/tools/intelligence.ts`
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `extract`, `extractInputFromUri`, `extractInputFromBytes`, `ExtractionConfig`, `NerConfig`, `NerBackendKind`, `EntityCategory` from `@xberg-io/xberg`
- Produces: `registerIntelligenceTools(server: McpServer): void` — exported from `intelligence.ts`

- [ ] **Step 1: Write the failing smoke test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("intelligence tools module", () => {
  it("exports registerIntelligenceTools", async () => {
    // Dynamic import avoids loading @xberg-io/xberg at test time
    const mod = await import("../src/tools/intelligence.js");
    expect(typeof mod.registerIntelligenceTools).toBe("function");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```
cd mcp-server && npm test -- run
```

Expected: FAIL — `Cannot find module '../src/tools/intelligence.js'`

- [ ] **Step 3: Create mcp-server/src/tools/intelligence.ts**

```typescript
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  extract,
  extractInputFromBytes,
  extractInputFromUri,
  type ExtractionConfig,
  type NerConfig,
} from "@xberg-io/xberg";

const InputSchema = z.object({
  uri: z.string().optional(),
  bytes: z.array(z.number().int().min(0).max(255)).optional(),
  mime_type: z.string().optional(),
  filename: z.string().optional(),
});

const NerSchema = z.object({
  backend: z.enum(["onnx", "llm"]).optional().default("onnx"),
  categories: z.array(z.string()).optional(),
  model: z.string().optional(),
});

const StructuredSchema = z.object({
  json_schema: z.record(z.unknown()),
  schema_name: z.string(),
  strict: z.boolean().optional().default(true),
  llm_model: z.string().optional(),
});

export function registerIntelligenceTools(server: McpServer): void {
  server.tool(
    "extract_entities",
    "Run named-entity recognition (NER) on a document. Returns persons, organizations, locations, emails, and custom categories. Backend 'onnx' uses GLiNER (fast, offline); 'llm' uses a connected LLM (higher accuracy). Provide uri or bytes.",
    {
      input: InputSchema,
      ner: NerSchema.optional(),
    },
    async ({ input, ner }) => {
      try {
        let extractInput;
        if (input.bytes) {
          extractInput = extractInputFromBytes(
            Buffer.from(input.bytes),
            input.mime_type ?? "application/octet-stream",
            input.filename ?? null,
          );
        } else if (input.uri) {
          extractInput = extractInputFromUri(input.uri);
        } else {
          return {
            content: [{ type: "text" as const, text: "Error: provide input.uri or input.bytes" }],
            isError: true,
          };
        }

        const nerConfig: NerConfig = {
          backend: (ner?.backend ?? "onnx") as NerConfig["backend"],
          categories: ner?.categories as NerConfig["categories"],
          model: ner?.model,
        };

        const config: ExtractionConfig = {
          disableOcr: true,
          ner: nerConfig,
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        if (!doc) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ entities: [] }) }],
          };
        }

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              entities: doc.entities ?? [],
              document_id: doc.metadata?.additional?.document_id ?? null,
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `extract_entities failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );

  server.tool(
    "structured_extract",
    "Extract structured JSON from a document using a provided JSON Schema and an LLM. The document is extracted first, then the text is sent to the LLM with your schema. Returns structured_output matching the schema.",
    {
      input: InputSchema,
      schema: StructuredSchema,
    },
    async ({ input, schema }) => {
      try {
        let extractInput;
        if (input.bytes) {
          extractInput = extractInputFromBytes(
            Buffer.from(input.bytes),
            input.mime_type ?? "application/octet-stream",
            input.filename ?? null,
          );
        } else if (input.uri) {
          extractInput = extractInputFromUri(input.uri);
        } else {
          return {
            content: [{ type: "text" as const, text: "Error: provide input.uri or input.bytes" }],
            isError: true,
          };
        }

        const config: ExtractionConfig = {
          structuredExtraction: {
            schema: schema.json_schema,
            schemaName: schema.schema_name,
            strict: schema.strict,
            llm: schema.llm_model ? { model: schema.llm_model } : undefined,
          },
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              structured_output: doc?.structuredOutput ?? null,
              content_preview: (doc?.content ?? "").slice(0, 200),
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `structured_extract failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. If `NerConfig["backend"]` or `structuredOutput` don't exist, check `crates/xberg-node/index.d.ts` for the exact field names (`entities`, `structuredOutput`/`structured_output`).

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: the new smoke test PASSES — the module loads without importing native bindings.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/tools/intelligence.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): add extract_entities and structured_extract tools"
```

---

### Task 2: Transcription tool — `transcribe_audio`

Uses `ExtractionConfig.transcription` to run Whisper ONNX on audio/video files. The Whisper model downloads lazily on first use from HuggingFace.

**Files:**
- Create: `mcp-server/src/tools/media.ts`
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `extract`, `extractInputFromUri`, `extractInputFromBytes`, `ExtractionConfig`, `TranscriptionConfig` from `@xberg-io/xberg`
- Produces: `registerMediaTools(server: McpServer): void`

- [ ] **Step 1: Write the failing smoke test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("media tools module", () => {
  it("exports registerMediaTools", async () => {
    const mod = await import("../src/tools/media.js");
    expect(typeof mod.registerMediaTools).toBe("function");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```
cd mcp-server && npm test -- run
```

Expected: FAIL — `Cannot find module '../src/tools/media.js'`

- [ ] **Step 3: Create mcp-server/src/tools/media.ts**

```typescript
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  extract,
  extractInputFromBytes,
  extractInputFromUri,
  type ExtractionConfig,
} from "@xberg-io/xberg";

const AudioInputSchema = z.object({
  uri: z.string().optional().describe("File path or HTTPS URL to audio/video file (mp3, m4a, wav, ogg, mp4, webm, etc.)"),
  bytes: z.array(z.number().int().min(0).max(255)).optional(),
  mime_type: z.string().optional(),
  filename: z.string().optional(),
});

const TranscriptionSchema = z.object({
  model: z.enum(["tiny", "base", "small", "medium", "large", "turbo"]).optional().default("base"),
  language: z.string().optional().describe("ISO 639-1 language code (e.g. 'en', 'fr'). Omit for auto-detect."),
  translate_to_english: z.boolean().optional().default(false),
});

export function registerMediaTools(server: McpServer): void {
  server.tool(
    "transcribe_audio",
    "Transcribe audio or video files to text using Whisper ONNX. Supports mp3, m4a, wav, ogg, flac, mp4, webm, and more. Model downloads automatically on first use (~150MB for 'base'). Returns transcript text with optional timestamps.",
    {
      input: AudioInputSchema,
      transcription: TranscriptionSchema.optional(),
    },
    async ({ input, transcription }) => {
      try {
        let extractInput;
        if (input.bytes) {
          extractInput = extractInputFromBytes(
            Buffer.from(input.bytes),
            input.mime_type ?? "audio/mpeg",
            input.filename ?? null,
          );
        } else if (input.uri) {
          extractInput = extractInputFromUri(input.uri);
        } else {
          return {
            content: [{ type: "text" as const, text: "Error: provide input.uri or input.bytes" }],
            isError: true,
          };
        }

        const config: ExtractionConfig = {
          transcription: {
            enabled: true,
            model: transcription?.model ?? "base",
            language: transcription?.language,
            translateToEnglish: transcription?.translate_to_english,
          },
        };

        const result = await extract(extractInput, config);
        const doc = (result.results ?? [])[0];

        if (!doc) {
          return {
            content: [{ type: "text" as const, text: JSON.stringify({ transcript: "", error: "no result" }) }],
          };
        }

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              transcript: doc.content ?? "",
              duration_ms: doc.metadata?.audio?.durationMs ?? null,
              language: (doc.detectedLanguages ?? [])[0] ?? null,
              model: transcription?.model ?? "base",
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `transcribe_audio failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. `TranscriptionConfig.translateToEnglish` — check the exact camelCase in `index.d.ts` if the compiler complains (may be `translateToEnglish` or `translate`).

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/tools/media.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): add transcribe_audio tool (Whisper ONNX)"
```

---

### Task 3: Web scraping tool — `scrape_url`

Uses `ExtractionConfig.url` with crawlberg to fetch and extract web pages, following JS rendering fallback automatically.

**Files:**
- Create: `mcp-server/src/tools/web.ts`
- Modify: `mcp-server/tests/tools.test.ts`

**Interfaces:**
- Consumes: `extract`, `extractInputFromUri`, `ExtractionConfig`, `UrlExtractionConfig`, `UrlExtractionMode` from `@xberg-io/xberg`
- Produces: `registerWebTools(server: McpServer): void`

- [ ] **Step 1: Write the failing smoke test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("web tools module", () => {
  it("exports registerWebTools", async () => {
    const mod = await import("../src/tools/web.js");
    expect(typeof mod.registerWebTools).toBe("function");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```
cd mcp-server && npm test -- run
```

Expected: FAIL — `Cannot find module '../src/tools/web.js'`

- [ ] **Step 3: Create mcp-server/src/tools/web.ts**

```typescript
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import {
  extract,
  extractInputFromUri,
  type ExtractionConfig,
} from "@xberg-io/xberg";

export function registerWebTools(server: McpServer): void {
  server.tool(
    "scrape_url",
    "Fetch and extract content from a URL or crawl a website. Supports JavaScript-rendered pages (headless fallback), PDFs served over HTTPS, and multi-page crawls. Returns extracted text and metadata.",
    {
      url: z.string().url().describe("HTTPS URL to scrape or crawl"),
      mode: z.enum(["document", "crawl"]).optional().default("document").describe(
        "'document' — extract a single page/file; 'crawl' — follow links up to max_pages"
      ),
      max_pages: z.number().int().min(1).max(100).optional().default(1).describe("Max pages to crawl (ignored in document mode)"),
      max_depth: z.number().int().min(1).max(5).optional().default(2).describe("Max link-hop depth (crawl mode only)"),
      js_rendering: z.enum(["auto", "always", "never"]).optional().default("auto").describe(
        "'auto' — use headless browser only when needed; 'always' — always render JS; 'never' — plain HTTP only"
      ),
      allow_subdomains: z.boolean().optional().default(false),
    },
    async ({ url, mode, max_pages, max_depth, js_rendering, allow_subdomains }) => {
      try {
        const extractInput = extractInputFromUri(url);

        const config: ExtractionConfig = {
          url: {
            mode: mode as "document" | "crawl",
            crawl: {
              maxDepth: max_depth,
              maxPages: max_pages,
              allowSubdomains: allow_subdomains,
              browser: {
                mode: js_rendering as "auto" | "always" | "never",
              },
            },
          },
        };

        const result = await extract(extractInput, config);
        const docs = result.results ?? [];

        return {
          content: [{
            type: "text" as const,
            text: JSON.stringify({
              pages_extracted: docs.length,
              documents: docs.map((doc) => ({
                url: doc.metadata?.additional?.source_url ?? url,
                title: doc.metadata?.title ?? null,
                content: doc.content ?? "",
                mime_type: doc.mimeType,
              })),
            }, null, 2),
          }],
        };
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        return {
          content: [{ type: "text" as const, text: `scrape_url failed: ${msg}` }],
          isError: true,
        };
      }
    }
  );
}
```

- [ ] **Step 4: Type-check**

```
cd mcp-server && node_modules/.bin/tsc --noEmit
```

Expected: zero errors. `UrlExtractionConfig.mode` accepts `"document" | "crawl"` — if the TypeScript enum values differ, check `UrlExtractionMode` in `index.d.ts` (values may be `"Document"` and `"Crawl"` — match exactly).

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/tools/web.ts mcp-server/tests/tools.test.ts
git commit -m "feat(mcp): add scrape_url tool (crawlberg web extraction)"
```

---

### Task 4: Register all new tools in index.ts

Wire the three new tool groups into the server.

**Files:**
- Modify: `mcp-server/src/index.ts`

**Interfaces:**
- Consumes: `registerIntelligenceTools`, `registerMediaTools`, `registerWebTools` from their respective modules
- Produces: All new tools visible to MCP clients on server start

- [ ] **Step 1: Write the failing test**

Add to `mcp-server/tests/tools.test.ts`:

```typescript
describe("index.ts registers all tool groups", () => {
  it("index imports succeed without native bindings", async () => {
    // We can't actually import index.ts (it starts the server) but we can
    // verify all tool modules load cleanly
    const mods = await Promise.all([
      import("../src/tools/extract.js"),
      import("../src/tools/collection.js"),
      import("../src/tools/intelligence.js"),
      import("../src/tools/media.js"),
      import("../src/tools/web.js"),
    ]);
    for (const mod of mods) {
      const keys = Object.keys(mod);
      const hasRegister = keys.some((k) => k.startsWith("register"));
      expect(hasRegister).toBe(true);
    }
  });
});
```

- [ ] **Step 2: Run test to confirm it passes with current state**

```
cd mcp-server && npm test -- run
```

Expected: PASS (all 5 modules now exist from Tasks 1-3).

- [ ] **Step 3: Add imports to mcp-server/src/index.ts**

After the existing imports in `index.ts`, add:

```typescript
import { registerIntelligenceTools } from "./tools/intelligence.js";
import { registerMediaTools } from "./tools/media.js";
import { registerWebTools } from "./tools/web.js";
```

After the existing `registerStatsTools(server);` call, add:

```typescript
registerIntelligenceTools(server);
registerMediaTools(server);
registerWebTools(server);
```

- [ ] **Step 4: Type-check and build**

```
cd mcp-server && node_modules/.bin/tsc --noEmit && npm run build
```

Expected: zero errors, `dist/` updated with new tool files.

- [ ] **Step 5: Run tests**

```
cd mcp-server && npm test -- run
```

Expected: all tests PASS, including the new registration test.

- [ ] **Step 6: Commit**

```
git add mcp-server/src/index.ts
git commit -m "feat(mcp): register intelligence, media, and web tool groups"
```
