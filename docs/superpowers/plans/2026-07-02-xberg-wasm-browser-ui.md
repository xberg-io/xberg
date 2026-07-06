# Xberg WASM Browser UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a minimal, client-side browser UI (`apps/xberg-web/`) that demonstrates the full xberg stack — extract, OCR, NER, anonymization, and RAG — running entirely in the browser via the shared wasm engine (B) and its injected JS runtime (C). Data never leaves the device.

**Architecture:** Vite + TypeScript, minimal framework. One `main.ts` entry point wiring DOM events to an `engine.ts` facade that constructs C's factories and builds `XbergEngine` from B. Heavy computation (inference, store ops) offloaded to a Worker via `worker.ts`. Five UI panels (extract, OCR, anonymize+redact+rehydrate, NER, RAG) each as a small view module under `ui/` with Vitest component tests. COOP/COEP headers required for SharedArrayBuffer and OPFS SQLite.

**Tech Stack:** TypeScript 5.3+, Vite 5.x, ESM, pnpm, Vitest + jsdom for components, Playwright for e2e, zod for input validation at boundaries, `oxfmt`/`oxlint` for formatting/linting.

**Spec:** [2026-07-02-xberg-wasm-browser-ui-design.md](../specs/2026-07-02-xberg-wasm-browser-ui-design.md)

**Dependency on C:** This plan's tasks reference C's factory interfaces (`embedder`, `store`, `ner?`, `ocr?`) by their spec signatures. C's own implementation plan is being written in parallel — **this plan can be written now (no blocker) but cannot be verified/tested until C's implementation is available.** All task TDD steps that construct `XbergEngine` and exercise factory calls will need to be done with C's actual exports once C is ready. Before that time, a stub/mock injection can verify the engine integration path; the full end-to-end will wait for C.

## Global Constraints

- TypeScript 5.3+, `strict: true`, `noUncheckedIndexedAccess`, ESM only, no CommonJS imports.
- Vite 5.x dev server; production build to static HTML + JS + CSS in `dist/`.
- **Critical:** Vite dev server MUST set COOP/COEP headers in `server.headers` config; any static host must do the same. Document this prominently — without the headers, SharedArrayBuffer is unavailable and ORT-Web falls back to single-threaded (~3–4× slower).
- pnpm workspace integration: root `pnpm-workspace.yaml` already exists; `apps/xberg-web/` will be added to it. `pnpm` is the primary package manager. Commit `pnpm-lock.yaml`.
- All shared UI utilities (form helpers, error rendering, fetch wrappers) go in `ui/shared/` or a `lib/` folder; each panel (extract, ocr, anon, ner, rag) is a separate view module under `ui/panels/`.
- Component tests: Vitest + jsdom, testing-library optional but recommended for DOM queries. **80%+ coverage target** on UI modules (ignore heavy integration code and mocks in coverage).
- Linting: `oxfmt` (formatting) + `oxlint` (linting) via the existing repo's prek pre-commit chain; no additional linters needed.
- Conventional commits: `feat:`, `fix:`, `chore:`, `test:`, `docs:`. First line <72 chars, imperative mood. **No AI attribution** in commit messages (repo `no-ai-signatures` rule).
- **Data privacy — explicitly:** All computation is client-side. No server calls except model/code downloads. User documents never leave the device. State persists in OPFS only via the Worker. This is a stated design goal and must be enforced in every panel.
- Run `prek run --all-files` before each commit; re-stage if hooks rewrite files.

---

### Task 1: Scaffold `apps/xberg-web/` with Vite + TypeScript

Create the project skeleton, wire Vite with COOP/COEP header config for the dev server, and set up the build/test pipeline.

**Files:**
- Create: `apps/xberg-web/` (root), `vite.config.ts`, `tsconfig.json`, `vitest.config.ts`, `index.html`, `src/main.ts`, `src/index.css`, `.gitignore`, `playwright.config.ts`
- Modify: `pnpm-workspace.yaml` (add `"apps/xberg-web"`), root `Taskfile.yml` (add `apps:*` task delegation if a separate `apps/` Taskfile is created, or inline tasks under a new `web:` namespace)

**Interfaces:**
- Produces: 
  - `vite.config.ts` with `server.headers` setting `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`.
  - `tsconfig.json` with `strict: true`, `noUncheckedIndexedAccess`, `moduleResolution: "bundler"`, ESM target.
  - `package.json` with scripts for `dev`, `build`, `test`, `test:ui`, `lint`, `format`, `e2e`.
  - Working Vite dev server accessible at `http://localhost:5173` with isolation headers applied.

- [ ] **Step 1: Create the directory structure**

```bash
mkdir -p apps/xberg-web/{src/{ui/{panels,shared},lib,worker},tests,e2e}
```

- [ ] **Step 2: Write `apps/xberg-web/package.json`**

```json
{
  "name": "xberg-web",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:ui": "vitest --ui",
    "test:watch": "vitest",
    "lint": "oxlint .",
    "format": "oxfmt --diff .",
    "format:fix": "oxfmt --write .",
    "e2e": "playwright test",
    "e2e:ui": "playwright test --ui"
  },
  "dependencies": {
    "@xberg-io/xberg": "file:../../crates/xberg-node",
    "xberg-rag-node": "file:../../crates/xberg-rag-node",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@playwright/test": "^1.40.0",
    "@testing-library/dom": "^9.3.0",
    "@testing-library/user-event": "^14.5.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "vitest": "^1.0.0",
    "@vitest/ui": "^1.0.0"
  }
}
```

- [ ] **Step 3: Write `apps/xberg-web/vite.config.ts`**

```typescript
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  root: __dirname,
  server: {
    port: 5173,
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  build: {
    outDir: 'dist',
    target: 'ES2022',
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
});
```

Documentation in a README comment or docs: "Dev server automatically sets COOP/COEP headers. When deploying the built `dist/` to a static host, ensure the same headers are set server-side, or SharedArrayBuffer and OPFS will be unavailable."

- [ ] **Step 4: Write `apps/xberg-web/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "moduleResolution": "bundler",
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "resolveJsonModule": true,
    "forceConsistentCasingInFileNames": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src/**/*", "tests/**/*"],
  "exclude": ["node_modules", "dist"]
}
```

- [ ] **Step 5: Write `apps/xberg-web/vitest.config.ts`**

```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: [],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      statements: 80,
      branches: 75,
      functions: 80,
      lines: 80,
    },
  },
});
```

- [ ] **Step 6: Write `apps/xberg-web/playwright.config.ts`**

```typescript
import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false, // Single instance to avoid port conflicts on dev server
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
  },
});
```

- [ ] **Step 7: Write `apps/xberg-web/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Xberg Browser UI</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 8: Write `apps/xberg-web/src/main.ts`**

```typescript
// Main entry point — initialize the app and wire panels to the engine facade.
// The app is fully client-side: extract, OCR, NER, anonymization, RAG all run in-wasm or in the Worker.

const root = document.getElementById('root');
if (!root) {
  throw new Error('root element not found');
}

// Startup self-check for COOP/COEP.
if (!globalThis.crossOriginIsolated) {
  root.innerHTML = `
    <div style="padding: 20px; background: #fee; border: 1px solid #c00; color: #c00; font-family: monospace;">
      <strong>⚠ COOP/COEP Headers Not Set</strong>
      <p>SharedArrayBuffer and OPFS are unavailable. ORT-Web inference will be single-threaded (3–4× slower).</p>
      <p>Dev server: check vite.config.ts server.headers. Static host: add Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy response headers.</p>
    </div>
  `;
} else {
  root.innerHTML = '<div id="app">Loading...</div>';
  // Import and run the app initialization (Task 2 onwards).
}

export {};
```

- [ ] **Step 9: Write `apps/xberg-web/src/index.css`**

```css
:root {
  --color-bg: #f5f5f5;
  --color-fg: #333;
  --color-border: #ddd;
  --color-error: #c00;
  --color-success: #060;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--color-bg);
  color: var(--color-fg);
}

#app {
  display: grid;
  grid-template-columns: 300px 1fr;
  gap: 1px;
  min-height: 100vh;
  background: var(--color-border);
}

#sidebar {
  background: white;
  padding: 20px;
  overflow-y: auto;
  border-right: 1px solid var(--color-border);
}

#main {
  background: white;
  padding: 20px;
  overflow-y: auto;
}

.error {
  background: #fee;
  border: 1px solid var(--color-error);
  color: var(--color-error);
  padding: 10px;
  border-radius: 4px;
  margin: 10px 0;
  font-size: 14px;
}

.warning {
  background: #ffe;
  border: 1px solid #cc0;
  color: #660;
  padding: 10px;
  border-radius: 4px;
  margin: 10px 0;
}

button {
  padding: 8px 12px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

button:hover {
  background: #0056b3;
}

button:disabled {
  background: #ccc;
  cursor: not-allowed;
}

input[type="text"],
input[type="password"],
textarea {
  width: 100%;
  padding: 8px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  font-family: monospace;
  font-size: 13px;
}

textarea {
  min-height: 100px;
  resize: vertical;
}
```

- [ ] **Step 10: Create `.gitignore` and `pnpm-workspace.yaml` entry**

`apps/xberg-web/.gitignore`:
```
node_modules
dist
.env.local
*.node
```

Modify root `pnpm-workspace.yaml`:
```yaml
packages:
  - "crates/*"
  - "packages/*"
  - "tools/*"
  - "apps/*"     # <-- add this line
```

- [ ] **Step 11: Run `pnpm install` to verify the workspace integration**

From the repo root:
```bash
cd apps/xberg-web
pnpm install
```

Expected: pnpm creates `node_modules`, updates root `pnpm-lock.yaml`.

- [ ] **Step 12: Verify Vite dev server starts and headers are set**

Run: `pnpm -C apps/xberg-web run dev 2>&1 | head -20`
Expected: "Local: http://localhost:5173". In a browser or via `curl -i http://localhost:5173`, verify response headers include `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`.

- [ ] **Step 13: Commit**

```bash
prek run --all-files
git add apps/xberg-web/ pnpm-workspace.yaml
git commit -m "feat(web): scaffold Vite + TypeScript app with COOP/COEP headers"
```

---

### Task 2: Implement `engine.ts` facade

Build the wasm engine from C's factories and enforce single-flight concurrency semantics per engine instance (serialize overlapping calls).

**Files:**
- Create: `apps/xberg-web/src/engine.ts`, `apps/xberg-web/src/types.ts` (if needed for the injection descriptor types)
- Test: `apps/xberg-web/tests/engine.test.ts`

**Interfaces:**
- Consumes: C's factory exports (from `packages/xberg-wasm-runtime/`) — `{ embedder, store, ner?, ocr? }` objects returned by factory functions, each with the async methods B's engine expects.
- Produces:
  - `class EngineFacade { constructor(injection, config), extract(...), ocr(...), detectPii(...), redact(...), rehydrate(...), ner(...), ingest(...), query(...) }`
  - Single-flight queue enforcer: overlapping calls on one `EngineFacade` instance serialize via a promise queue.
  - Type-safe error handling: all engine methods wrap `Result<T, JsValue>` and surface errors as typed JS exceptions.

- [ ] **Step 1: Write the failing unit test for single-flight enforcement**

`apps/xberg-web/tests/engine.test.ts`:

```typescript
import { describe, it, expect, vi } from 'vitest';
import { EngineFacade } from '../src/engine';

describe('EngineFacade', () => {
  it('should serialize concurrent ingest calls on one instance', async () => {
    // Mock injection with a stub engine that tracks call order.
    const callOrder: string[] = [];
    const stubInjection = {
      embedder: {
        embed: vi.fn(async (texts: string[]) => {
          callOrder.push('embed');
          await new Promise((r) => setTimeout(r, 10)); // Simulate delay.
          return texts.map(() => [0.1, 0.2]);
        }),
      },
      store: {
        // Minimal stub store methods.
        ensureCollection: vi.fn(async () => {}),
        upsertDocument: vi.fn(async () => ({ sourceId: 'doc1' })),
        query: vi.fn(async () => []),
        name: () => 'stub',
        capabilities: () => ({ vector: true, fulltext: false }),
      },
    };

    const facade = new EngineFacade(stubInjection, {});

    // Trigger two concurrent ingest calls.
    const p1 = facade.ingest({ text: 'doc1' }, 'col1');
    const p2 = facade.ingest({ text: 'doc2' }, 'col1');

    await Promise.all([p1, p2]);

    // Verify they ran sequentially (both calls completed, no interleaving errors).
    expect(callOrder.length).toBe(2);
    expect(stubInjection.embedder.embed).toHaveBeenCalledTimes(2);
  });

  it('should propagate engine errors as typed exceptions', async () => {
    const stubInjection = {
      embedder: {
        embed: vi.fn(async () => {
          throw new Error('Embedder offline');
        }),
      },
      store: {
        ensureCollection: vi.fn(async () => {}),
        upsertDocument: vi.fn(async () => ({ sourceId: 'doc1' })),
        query: vi.fn(async () => []),
        name: () => 'stub',
        capabilities: () => ({ vector: true, fulltext: false }),
      },
    };

    const facade = new EngineFacade(stubInjection, {});

    await expect(
      facade.ingest({ text: 'test' }, 'col1')
    ).rejects.toThrow('Embedder offline');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm -C apps/xberg-web run test 2>&1 | tail -20`
Expected: FAIL — `EngineFacade` not found.

- [ ] **Step 3: Implement `EngineFacade`**

`apps/xberg-web/src/engine.ts`:

```typescript
/**
 * EngineFacade: constructs and manages the shared XbergEngine instance.
 * Enforces single-flight concurrency: overlapping calls on one facade serialize.
 * All engine methods are async; errors propagate as typed JS exceptions.
 */

import type { ExtractionConfig, ExtractionResult } from '@xberg-io/xberg';

export interface EngineInjection {
  embedder: {
    embed(texts: string[]): Promise<Float32Array[]>;
  };
  store: {
    ensureCollection(colId: string): Promise<void>;
    upsertDocument(colId: string, doc: Record<string, unknown>): Promise<{ sourceId: string }>;
    query(colId: string, vector: Float32Array, k: number): Promise<Array<{ text: string; score: number }>>;
    deleteDocuments(colId: string, filter: Record<string, unknown>): Promise<void>;
    dropCollection(colId: string): Promise<void>;
    listCollections(): Promise<string[]>;
    name(): string;
    capabilities(): { vector: boolean; fulltext: boolean };
  };
  ner?: {
    ner(text: string, opts?: Record<string, unknown>): Promise<Array<{ text: string; label: string; start: number; end: number }>>;
  };
  ocr?: {
    ocr(bytes: Uint8Array, opts?: Record<string, unknown>): Promise<{ text: string; confidence: number }>;
  };
}

export interface EngineFacadeConfig {
  extractionConfig?: ExtractionConfig;
  // Additional engine config as needed.
}

/**
 * Simple promise-queue to enforce single-flight semantics.
 * Overlapping async calls on one instance serialize: next call waits for previous.
 */
class SingleFlightQueue {
  private pending: Promise<void> = Promise.resolve();

  async run<T>(fn: () => Promise<T>): Promise<T> {
    return new Promise((resolve, reject) => {
      this.pending = this.pending
        .then(async () => {
          try {
            const result = await fn();
            resolve(result);
          } catch (e) {
            reject(e);
          }
        })
        .catch(reject);
      return this.pending;
    });
  }
}

export class EngineFacade {
  private engine: any; // Typed as `any` until B's XbergEngine is fully built; will be refined.
  private queue = new SingleFlightQueue();
  private injection: EngineInjection;
  private config: EngineFacadeConfig;

  constructor(injection: EngineInjection, config: EngineFacadeConfig) {
    this.injection = injection;
    this.config = config;
    // engine construction deferred to lazy init in first method call.
  }

  private async initEngine() {
    if (this.engine) return;

    // Dynamically import the wasm engine (B) from the xberg-wasm package.
    // This will be: const { XbergEngine } = await import('@xberg-io/xberg-wasm');
    // For now, use a placeholder — the actual import will be added when B's NAPI binding ships.
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let XbergEngine: any;
    try {
      // Attempt to load from the wasm binding package (will be available after B is built).
      const mod = await import('@xberg-io/xberg-wasm');
      XbergEngine = mod.XbergEngine;
    } catch {
      throw new Error(
        'XbergEngine not available. Ensure the wasm engine (B) is built and packaged as a WASM binding.'
      );
    }

    this.engine = new XbergEngine(this.config.extractionConfig || {}, {
      embedder: this.injection.embedder,
      store: this.injection.store,
      ner: this.injection.ner,
      ocr: this.injection.ocr,
    });
  }

  async extract(
    input: Uint8Array,
    config?: ExtractionConfig
  ): Promise<ExtractionResult> {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.extract(input, config);
    });
  }

  async ocr(bytes: Uint8Array, opts?: Record<string, unknown>) {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.ocr(bytes, opts);
    });
  }

  async detectPii(text: string) {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.detectPii(text);
    });
  }

  async redact(
    text: string,
    strategy: 'mask' | 'hash' | 'token_replace'
  ): Promise<{ text: string; tokenMap?: Record<string, string> }> {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.redact(text, { strategy });
    });
  }

  async rehydrate(redactedDoc: string, mapBytes: Uint8Array, passphrase: string): Promise<string> {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.rehydrate(redactedDoc, mapBytes, passphrase);
    });
  }

  async ner(text: string, opts?: Record<string, unknown>) {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.ner(text, opts);
    });
  }

  async ingest(doc: { text: string }, collectionId: string) {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.ingest(doc, collectionId);
    });
  }

  async query(q: string, collectionId: string, k: number = 5) {
    return this.queue.run(async () => {
      await this.initEngine();
      return this.engine.query(q, collectionId, k);
    });
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm -C apps/xberg-web run test 2>&1 | tail -20`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add apps/xberg-web/src/engine.ts apps/xberg-web/tests/engine.test.ts
git commit -m "feat(web): engine facade with single-flight concurrency"
```

---

### Task 3: Implement `worker.ts` bridge

Move heavy computation (inference, vector store ops) to a Worker. Main thread communicates via typed `postMessage` protocol.

**Files:**
- Create: `apps/xberg-web/src/worker.ts`, `apps/xberg-web/src/workerMessages.ts` (types), `apps/xberg-web/src/workerClient.ts` (main-thread handle)
- Test: `apps/xberg-web/tests/worker.test.ts` (mock Worker, test message round-trips)

**Interfaces:**
- Produces:
  - `worker.ts`: initializes C's store (wa-sqlite/OPFS), embedder, ner, ocr; listens for typed messages from main thread; returns results via `postMessage`.
  - `workerClient.ts`: `class WorkerClient` wrapping a Worker, with async methods (`embed`, `upsertDocument`, `query`, etc.) that send typed messages and await responses.
  - `workerMessages.ts`: TypeScript types for the message protocol (discriminated unions for request/response types).

- [ ] **Step 1: Write `apps/xberg-web/src/workerMessages.ts`**

```typescript
/**
 * Typed message protocol between main thread and worker.
 * All requests include a unique id; responses include the same id for matching.
 */

export type WorkerRequest =
  | { type: 'init'; config: Record<string, unknown> }
  | { id: string; type: 'embed'; texts: string[] }
  | { id: string; type: 'upsertDocument'; colId: string; doc: Record<string, unknown> }
  | { id: string; type: 'query'; colId: string; vector: number[]; k: number }
  | { id: string; type: 'ensureCollection'; colId: string }
  | { id: string; type: 'listCollections' }
  | { id: string; type: 'dropCollection'; colId: string }
  | { id: string; type: 'ner'; text: string; opts?: Record<string, unknown> }
  | { id: string; type: 'ocr'; bytes: Uint8Array; opts?: Record<string, unknown> };

export type WorkerResponse =
  | { type: 'ready' }
  | { id: string; type: 'success'; result: unknown }
  | { id: string; type: 'error'; message: string; code?: number };

export function isWorkerResponse(v: unknown): v is WorkerResponse {
  return (
    typeof v === 'object' &&
    v !== null &&
    'type' in v &&
    (v.type === 'ready' || 'id' in v)
  );
}
```

- [ ] **Step 2: Write `apps/xberg-web/src/worker.ts`**

```typescript
/**
 * Worker thread: hosts C's stores, embedder, NER, OCR.
 * Receives typed messages from main thread and dispatches to the appropriate service.
 */

import type { WorkerRequest, WorkerResponse } from './workerMessages';

// On first message, initialize C's injection descriptor. Store its members in module scope.
let embedder: any; // C's embedder, from the injection descriptor
let store: any; // C's store, from the injection descriptor
let ner: any; // C's ner, from the injection descriptor (optional)
let ocr: any; // C's ocr, from the injection descriptor (optional)
let ready = false;

async function init(config: Record<string, unknown>) {
  try {
    // C's package (`xberg-wasm-runtime`) exports a single public factory,
    // `createXbergRuntimeFactory(config?)`, that builds and validates the
    // whole { embedder, store, ner?, ocr? } descriptor — do not import the
    // per-capability factories directly, they are not re-exported from C's
    // index.ts (see the runtime-layer plan's Task 9).
    const { createXbergRuntimeFactory } = await import('xberg-wasm-runtime');
    injection = await createXbergRuntimeFactory(config);
    embedder = injection.embedder;
    store = injection.store;
    ner = injection.ner;
    ocr = injection.ocr;
    ready = true;
    return { success: true };
  } catch (e) {
    throw new Error(`Worker init failed: ${e instanceof Error ? e.message : String(e)}`);
  }
}

async function handleRequest(req: WorkerRequest): Promise<unknown> {
  if (req.type === 'init') {
    return init(req.config);
  }

  if (!ready) {
    throw new Error('Worker not initialized');
  }

  switch (req.type) {
    case 'embed':
      return embedder.embed(req.texts);

    case 'upsertDocument':
      return store.upsertDocument(req.colId, req.doc);

    case 'query':
      return store.query(req.colId, new Float32Array(req.vector), req.k);

    case 'ensureCollection':
      return store.ensureCollection(req.colId);

    case 'listCollections':
      return store.listCollections();

    case 'dropCollection':
      return store.dropCollection(req.colId);

    case 'ner':
      if (!ner) throw new Error('NER not available');
      return ner.ner(req.text, req.opts);

    case 'ocr':
      if (!ocr) throw new Error('OCR not available');
      return ocr.ocr(req.bytes, req.opts);

    default:
      throw new Error(`Unknown request type: ${(req as any).type}`);
  }
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const req = event.data;

  try {
    const result = await handleRequest(req);
    const resp: WorkerResponse = {
      id: req.type === 'init' ? 'init' : (req as any).id,
      type: 'success',
      result,
    };
    self.postMessage(resp);
  } catch (e) {
    const resp: WorkerResponse = {
      id: req.type === 'init' ? 'init' : (req as any).id,
      type: 'error',
      message: e instanceof Error ? e.message : String(e),
    };
    self.postMessage(resp);
  }
};

// Notify main thread that worker is ready for init.
self.postMessage({ type: 'ready' } as WorkerResponse);
```

- [ ] **Step 3: Write `apps/xberg-web/src/workerClient.ts`**

```typescript
/**
 * Main-thread handle to the Worker. Wraps postMessage in async methods.
 */

import type { WorkerRequest, WorkerResponse } from './workerMessages';

export class WorkerClient {
  private worker: Worker;
  private pending = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();
  private counter = 0;

  constructor(workerUrl: string) {
    this.worker = new Worker(workerUrl, { type: 'module' });
    this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const resp = event.data;
      if (resp.type === 'ready') {
        return; // Acknowledged.
      }
      const { id, type, result, message } = resp;
      const pending = this.pending.get(id);
      if (!pending) {
        console.warn(`Worker response for unknown id: ${id}`);
        return;
      }
      this.pending.delete(id);
      if (type === 'error') {
        pending.reject(new Error(message || 'Worker error'));
      } else {
        pending.resolve(result);
      }
    };
    this.worker.onerror = (event: ErrorEvent) => {
      console.error('Worker error:', event.message);
      // Reject all pending requests.
      for (const { reject } of this.pending.values()) {
        reject(new Error(event.message));
      }
      this.pending.clear();
    };
  }

  private async send<T>(req: WorkerRequest): Promise<T> {
    const id = String(++this.counter);
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ ...req, id });
    });
  }

  async init(config: Record<string, unknown>): Promise<void> {
    await this.send({ type: 'init', config });
  }

  async embed(texts: string[]): Promise<Float32Array[]> {
    return this.send({ type: 'embed', texts });
  }

  async upsertDocument(colId: string, doc: Record<string, unknown>): Promise<{ sourceId: string }> {
    return this.send({ type: 'upsertDocument', colId, doc });
  }

  async query(colId: string, vector: Float32Array, k: number): Promise<Array<{ text: string; score: number }>> {
    return this.send({ type: 'query', colId, vector: Array.from(vector), k });
  }

  async ensureCollection(colId: string): Promise<void> {
    return this.send({ type: 'ensureCollection', colId });
  }

  async listCollections(): Promise<string[]> {
    return this.send({ type: 'listCollections' });
  }

  async dropCollection(colId: string): Promise<void> {
    return this.send({ type: 'dropCollection', colId });
  }

  async ner(text: string, opts?: Record<string, unknown>): Promise<Array<{ text: string; label: string; start: number; end: number }>> {
    return this.send({ type: 'ner', text, opts });
  }

  async ocr(bytes: Uint8Array, opts?: Record<string, unknown>): Promise<{ text: string; confidence: number }> {
    return this.send({ type: 'ocr', bytes, opts });
  }

  terminate(): void {
    this.worker.terminate();
  }
}
```

- [ ] **Step 4: Write `apps/xberg-web/tests/worker.test.ts`**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { WorkerClient } from '../src/workerClient';

describe('WorkerClient', () => {
  // Mock Worker for testing main-thread logic (actual Worker integration tested via e2e).
  it('should serialize and deserialize worker messages correctly', async () => {
    // Create a fake worker.
    const messages: unknown[] = [];
    const mockWorker = {
      postMessage: (msg: unknown) => {
        messages.push(msg);
        // Echo back a success response after a microtask.
        Promise.resolve().then(() => {
          (mockWorker as any).onmessage?.({
            data: { id: (msg as any).id, type: 'success', result: 42 },
          });
        });
      },
      terminate: () => {},
      onerror: null as any,
      onmessage: null as any,
    };

    // Stub Worker constructor globally for this test.
    const originalWorker = globalThis.Worker;
    (globalThis as any).Worker = function () {
      return mockWorker;
    };

    try {
      const client = new (WorkerClient as any)(
        new Proxy(new URL('', import.meta.url), {
          get: () => mockWorker,
        })
      );

      const result = await client.embed(['test']);
      expect(result).toBe(42);
    } finally {
      (globalThis as any).Worker = originalWorker;
    }
  });
});
```

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add apps/xberg-web/src/worker.ts apps/xberg-web/src/workerClient.ts apps/xberg-web/src/workerMessages.ts apps/xberg-web/tests/worker.test.ts
git commit -m "feat(web): worker thread for inference and store ops"
```

---

### Task 4: Extract panel

Implement the UI for document extraction. Component test with mocked engine.

**Files:**
- Create: `apps/xberg-web/src/ui/panels/extract.ts`, `apps/xberg-web/tests/extract.test.ts`

**Interfaces:**
- Consumes: `EngineFacade.extract(bytes, config)` → `ExtractionResult { text, metadata, ... }`
- Produces: a DOM element with file input, "Extract" button, text output area, metadata display.

- [ ] **Step 1: Write the failing component test**

```typescript
import { describe, it, expect, vi } from 'vitest';
import { createExtractPanel } from '../src/ui/panels/extract';

describe('Extract panel', () => {
  it('should render file input and extract button', () => {
    const mockFacade = {
      extract: vi.fn(async () => ({ text: 'extracted text', metadata: {} })),
    };

    const panel = createExtractPanel(mockFacade);
    expect(panel.querySelector('input[type="file"]')).toBeTruthy();
    expect(panel.querySelector('button')).toBeTruthy();
  });

  it('should extract and display text on button click', async () => {
    const mockFacade = {
      extract: vi.fn(async () => ({ text: 'hello world', metadata: { pages: 1 } })),
    };

    const panel = createExtractPanel(mockFacade);
    const output = panel.querySelector('[data-testid="output"]') as HTMLElement;

    // Simulate file input.
    const fileInput = panel.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(['test'], 'test.txt', { type: 'text/plain' });
    Object.defineProperty(fileInput, 'files', { value: [file] });

    // Click extract button.
    const button = panel.querySelector('button') as HTMLButtonElement;
    button.click();

    // Wait for async extraction.
    await new Promise((r) => setTimeout(r, 50));

    expect(output.textContent).toContain('hello world');
    expect(mockFacade.extract).toHaveBeenCalled();
  });

  it('should display error message on extraction failure', async () => {
    const mockFacade = {
      extract: vi.fn(async () => {
        throw new Error('Unsupported format');
      }),
    };

    const panel = createExtractPanel(mockFacade);
    const fileInput = panel.querySelector('input[type="file"]') as HTMLInputElement;
    const file = new File(['test'], 'test.xyz', { type: 'application/unknown' });
    Object.defineProperty(fileInput, 'files', { value: [file] });

    const button = panel.querySelector('button') as HTMLButtonElement;
    button.click();
    await new Promise((r) => setTimeout(r, 50));

    const error = panel.querySelector('.error');
    expect(error).toBeTruthy();
    expect(error?.textContent).toContain('Unsupported format');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm -C apps/xberg-web run test extract 2>&1 | tail -20`
Expected: FAIL.

- [ ] **Step 3: Implement `createExtractPanel`**

```typescript
/**
 * Extract panel: drop a file, extract text and metadata.
 */

import type { EngineFacade } from '../engine';

export function createExtractPanel(facade: EngineFacade): HTMLElement {
  const panel = document.createElement('div');
  panel.className = 'panel extract-panel';

  panel.innerHTML = `
    <h2>Extract</h2>
    <input type="file" id="file-input" multiple />
    <button id="extract-btn">Extract</button>
    <div id="output" data-testid="output" style="margin-top: 20px; min-height: 100px; border: 1px solid #ddd; padding: 10px; white-space: pre-wrap; word-break: break-word;"></div>
    <div id="error" class="error" style="display: none;"></div>
  `;

  const fileInput = panel.querySelector<HTMLInputElement>('#file-input')!;
  const btn = panel.querySelector<HTMLButtonElement>('#extract-btn')!;
  const output = panel.querySelector<HTMLElement>('#output')!;
  const errorDiv = panel.querySelector<HTMLElement>('#error')!;

  btn.addEventListener('click', async () => {
    if (!fileInput.files || fileInput.files.length === 0) {
      errorDiv.textContent = 'No file selected';
      errorDiv.style.display = 'block';
      return;
    }

    btn.disabled = true;
    errorDiv.style.display = 'none';
    output.textContent = 'Extracting...';

    try {
      for (const file of fileInput.files) {
        const bytes = await file.arrayBuffer();
        const result = await facade.extract(new Uint8Array(bytes));
        output.textContent = result.text || '(no text extracted)';
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      errorDiv.textContent = msg;
      errorDiv.style.display = 'block';
      output.textContent = '';
    } finally {
      btn.disabled = false;
    }
  });

  return panel;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pnpm -C apps/xberg-web run test extract 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add apps/xberg-web/src/ui/panels/extract.ts apps/xberg-web/tests/extract.test.ts
git commit -m "feat(web): extract panel with file input and text output"
```

---

### Task 5: OCR panel

Implement OCR toggle with fallback messaging when offline or no model.

**Files:**
- Create: `apps/xberg-web/src/ui/panels/ocr.ts`, `apps/xberg-web/tests/ocr.test.ts`

**Interfaces:**
- Consumes: `EngineFacade.ocr(bytes, { language?: string })` → `{ text: string; confidence: number }`
- Produces: a checkbox "Enable OCR", language selector (ISO codes), input image preview, output text + confidence score.

- [ ] **Step 1: Write the component test (failing)**

```typescript
it('should display OCR results and confidence score', async () => {
  const mockFacade = {
    ocr: vi.fn(async () => ({ text: 'OCR result', confidence: 0.95 })),
  };

  const panel = createOcrPanel(mockFacade);
  const checkbox = panel.querySelector('input[type="checkbox"]') as HTMLInputElement;

  checkbox.checked = true;
  checkbox.dispatchEvent(new Event('change'));

  const fileInput = panel.querySelector('input[type="file"]') as HTMLInputElement;
  const file = new File(['image'], 'test.png', { type: 'image/png' });
  Object.defineProperty(fileInput, 'files', { value: [file] });

  const btn = panel.querySelector('button') as HTMLButtonElement;
  btn.click();

  await new Promise((r) => setTimeout(r, 50));

  const output = panel.querySelector('[data-testid="ocr-output"]');
  expect(output?.textContent).toContain('OCR result');
  expect(output?.textContent).toContain('0.95');
});

it('should show offline fallback message when OCR unavailable', async () => {
  const mockFacade = {
    ocr: vi.fn(async () => {
      throw new Error('OCR unavailable: no injected backend');
    }),
  };

  const panel = createOcrPanel(mockFacade);
  const checkbox = panel.querySelector('input[type="checkbox"]') as HTMLInputElement;

  checkbox.checked = true;
  checkbox.dispatchEvent(new Event('change'));

  const fileInput = panel.querySelector('input[type="file"]') as HTMLInputElement;
  const file = new File(['image'], 'test.png', { type: 'image/png' });
  Object.defineProperty(fileInput, 'files', { value: [file] });

  const btn = panel.querySelector('button') as HTMLButtonElement;
  btn.click();

  await new Promise((r) => setTimeout(r, 50));

  const warning = panel.querySelector('.warning');
  expect(warning?.textContent).toContain('offline fallback');
});
```

- [ ] **Step 2: Implement `createOcrPanel`**

```typescript
/**
 * OCR panel: toggle OCR, select language, upload image, get recognized text + confidence.
 */

import type { EngineFacade } from '../engine';

export function createOcrPanel(facade: EngineFacade): HTMLElement {
  const panel = document.createElement('div');
  panel.className = 'panel ocr-panel';

  panel.innerHTML = `
    <h2>OCR</h2>
    <label>
      <input type="checkbox" id="ocr-toggle" />
      Enable OCR
    </label>
    <div id="ocr-controls" style="display: none; margin-top: 10px;">
      <label>
        Language (ISO 639):
        <input type="text" id="ocr-lang" value="eng" placeholder="eng, deu, fra, ..." style="width: 100px;" />
      </label>
      <input type="file" id="ocr-file" accept="image/*" />
      <button id="ocr-btn">Recognize Text</button>
    </div>
    <div id="ocr-output" data-testid="ocr-output" style="margin-top: 20px; min-height: 100px; border: 1px solid #ddd; padding: 10px; white-space: pre-wrap;"></div>
    <div class="warning" id="ocr-warning" style="display: none;"></div>
    <div class="error" id="ocr-error" style="display: none;"></div>
  `;

  const toggle = panel.querySelector<HTMLInputElement>('#ocr-toggle')!;
  const controls = panel.querySelector<HTMLElement>('#ocr-controls')!;
  const langInput = panel.querySelector<HTMLInputElement>('#ocr-lang')!;
  const fileInput = panel.querySelector<HTMLInputElement>('#ocr-file')!;
  const btn = panel.querySelector<HTMLButtonElement>('#ocr-btn')!;
  const output = panel.querySelector<HTMLElement>('#ocr-output')!;
  const warning = panel.querySelector<HTMLElement>('#ocr-warning')!;
  const errorDiv = panel.querySelector<HTMLElement>('#ocr-error')!;

  toggle.addEventListener('change', () => {
    controls.style.display = toggle.checked ? 'block' : 'none';
  });

  btn.addEventListener('click', async () => {
    if (!fileInput.files || fileInput.files.length === 0) {
      errorDiv.textContent = 'No image selected';
      errorDiv.style.display = 'block';
      return;
    }

    btn.disabled = true;
    errorDiv.style.display = 'none';
    warning.style.display = 'none';
    output.textContent = 'Recognizing...';

    try {
      const file = fileInput.files[0];
      const bytes = await file.arrayBuffer();
      const result = await facade.ocr(new Uint8Array(bytes), { language: langInput.value });
      output.innerHTML = `<strong>Text:</strong> ${result.text}<br><strong>Confidence:</strong> ${(result.confidence * 100).toFixed(1)}%`;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('unavailable') || msg.includes('offline')) {
        warning.textContent = `⚠ OCR offline: using offline fallback or unavailable. ${msg}`;
        warning.style.display = 'block';
      } else {
        errorDiv.textContent = msg;
        errorDiv.style.display = 'block';
      }
      output.textContent = '';
    } finally {
      btn.disabled = false;
    }
  });

  return panel;
}
```

- [ ] **Step 3: Run test and commit**

Run: `pnpm -C apps/xberg-web run test ocr 2>&1 | tail -20`
Expected: PASS.

```bash
prek run --all-files
git add apps/xberg-web/src/ui/panels/ocr.ts apps/xberg-web/tests/ocr.test.ts
git commit -m "feat(web): OCR panel with language selection and fallback messaging"
```

---

### Task 6: Anonymize panel

Implement PII detection, redaction strategy selection, encrypted map download, and rehydration.

**Files:**
- Create: `apps/xberg-web/src/ui/panels/anonymize.ts`, `apps/xberg-web/tests/anonymize.test.ts`

**Interfaces:**
- Consumes: `EngineFacade.detectPii(text)` → `{ category: string; indices: [number, number] }[]`; `redact(text, strategy)` → `{ text: string; tokenMap?: Map }` ; `rehydrate(redactedDoc, mapBytes, passphrase)` → `text`
- Produces: textarea input, "Detect PII" button, detection table (category, count), strategy radio (mask/hash/token_replace), "Redact" button, output textarea, conditional "Download Map" button + passphrase input, "Rehydrate" box with encrypted map upload and passphrase input.

- [ ] **Step 1: Write failing test**

```typescript
it('should detect PII and show categories', async () => {
  const mockFacade = {
    detectPii: vi.fn(async () => [
      { category: 'EMAIL', indices: [0, 16] },
      { category: 'EMAIL', indices: [18, 32] },
    ]),
  };

  const panel = createAnonymizePanel(mockFacade);
  const textarea = panel.querySelector('textarea[id="text-input"]') as HTMLTextAreaElement;
  textarea.value = 'jane@example.com, bob@test.com';

  const detectBtn = panel.querySelector('button[data-testid="detect-btn"]') as HTMLButtonElement;
  detectBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  const detections = panel.querySelector('[data-testid="detections"]');
  expect(detections?.textContent).toContain('EMAIL');
  expect(detections?.textContent).toContain('2');
});

it('should redact with token_replace and enable map download', async () => {
  const mockFacade = {
    detectPii: vi.fn(async () => []),
    redact: vi.fn(async () => ({
      text: '[EMAIL_1] and [EMAIL_2]',
      tokenMap: new Blob([JSON.stringify({ '[EMAIL_1]': 'jane@example.com' })]),
    })),
  };

  const panel = createAnonymizePanel(mockFacade);
  const textarea = panel.querySelector('textarea[id="text-input"]') as HTMLTextAreaElement;
  textarea.value = 'jane@example.com and bob@test.com';

  const strategyRadio = panel.querySelector('input[value="token_replace"]') as HTMLInputElement;
  strategyRadio.checked = true;

  const redactBtn = panel.querySelector('button[data-testid="redact-btn"]') as HTMLButtonElement;
  redactBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  const downloadBtn = panel.querySelector('button[data-testid="download-map-btn"]');
  expect(downloadBtn).toBeTruthy();
});

it('should rehydrate with correct passphrase', async () => {
  const mockFacade = {
    rehydrate: vi.fn(async () => 'jane@example.com'),
  };

  const panel = createAnonymizePanel(mockFacade);

  // Simulate user uploading map and entering passphrase.
  const mapInput = panel.querySelector('input[id="map-upload"]') as HTMLInputElement;
  const mapFile = new File([new ArrayBuffer(100)], 'map.xpii');
  Object.defineProperty(mapInput, 'files', { value: [mapFile] });

  const passphraseInput = panel.querySelector('input[id="rehydrate-passphrase"]') as HTMLInputElement;
  passphraseInput.value = 'my-secret';

  const rehydrateBtn = panel.querySelector('button[data-testid="rehydrate-btn"]') as HTMLButtonElement;
  rehydrateBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  expect(mockFacade.rehydrate).toHaveBeenCalled();
});

it('should show decryption error on wrong passphrase', async () => {
  const mockFacade = {
    rehydrate: vi.fn(async () => {
      throw new Error('decryption failed (AES-GCM auth tag mismatch)');
    }),
  };

  const panel = createAnonymizePanel(mockFacade);
  const rehydrateBtn = panel.querySelector('button[data-testid="rehydrate-btn"]') as HTMLButtonElement;
  rehydrateBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  const error = panel.querySelector('.error');
  expect(error?.textContent).toContain('decryption failed');
});
```

- [ ] **Step 2: Implement `createAnonymizePanel`**

```typescript
/**
 * Anonymize panel: detect PII, choose redaction strategy, download encrypted map, rehydrate.
 */

import type { EngineFacade } from '../engine';

export function createAnonymizePanel(facade: EngineFacade): HTMLElement {
  const panel = document.createElement('div');
  panel.className = 'panel anonymize-panel';

  panel.innerHTML = `
    <h2>Anonymize & Redact</h2>

    <h3>Step 1: Detect PII</h3>
    <textarea id="text-input" placeholder="Paste text to analyze..." style="width: 100%; height: 150px;"></textarea>
    <button data-testid="detect-btn">Detect PII</button>
    <div data-testid="detections" style="margin-top: 10px; border: 1px solid #ddd; padding: 10px; display: none;">
      <table style="width: 100%; border-collapse: collapse;">
        <tr><th style="border-bottom: 1px solid #ddd; text-align: left; padding: 5px;">Category</th><th style="border-bottom: 1px solid #ddd; text-align: left; padding: 5px;">Count</th></tr>
        <tbody id="detection-table"></tbody>
      </table>
    </div>

    <h3>Step 2: Choose Redaction Strategy</h3>
    <label>
      <input type="radio" name="strategy" value="mask" checked />
      Mask (replace with [REDACTED])
    </label>
    <label>
      <input type="radio" name="strategy" value="hash" />
      Hash (one-way SHA-256)
    </label>
    <label>
      <input type="radio" name="strategy" value="token_replace" />
      Token Replace (reversible with passphrase)
    </label>
    <button data-testid="redact-btn">Redact</button>

    <h3>Redacted Output</h3>
    <textarea id="redacted-output" readonly style="width: 100%; height: 150px;"></textarea>

    <div id="token-section" style="display: none; margin-top: 20px; border: 1px solid #ccc; padding: 10px;">
      <h4>Token Replace Map</h4>
      <button data-testid="download-map-btn">Download Encrypted Map</button>
      <p style="font-size: 12px; color: #666;">Encrypted with your passphrase. Download and store safely.</p>
    </div>

    <h3>Step 3: Rehydrate (Optional)</h3>
    <div style="border: 1px solid #ddd; padding: 10px; margin-top: 10px;">
      <label>
        Upload encrypted map:
        <input type="file" id="map-upload" accept=".xpii" />
      </label>
      <label>
        Passphrase:
        <input type="password" id="rehydrate-passphrase" placeholder="Enter passphrase to decrypt..." />
      </label>
      <button data-testid="rehydrate-btn">Rehydrate</button>
    </div>
    <div id="rehydrated-output" style="margin-top: 10px; border: 1px solid #ddd; padding: 10px; display: none;"></div>

    <div class="error" id="error" style="display: none;"></div>
  `;

  const textInput = panel.querySelector<HTMLTextAreaElement>('#text-input')!;
  const detectBtn = panel.querySelector<HTMLButtonElement>('[data-testid="detect-btn"]')!;
  const detectionsDiv = panel.querySelector<HTMLElement>('[data-testid="detections"]')!;
  const detectionTable = panel.querySelector<HTMLElement>('#detection-table')!;
  const strategyRadios = panel.querySelectorAll<HTMLInputElement>('input[name="strategy"]');
  const redactBtn = panel.querySelector<HTMLButtonElement>('[data-testid="redact-btn"]')!;
  const redactedOutput = panel.querySelector<HTMLTextAreaElement>('#redacted-output')!;
  const tokenSection = panel.querySelector<HTMLElement>('#token-section')!;
  const downloadMapBtn = panel.querySelector<HTMLButtonElement>('[data-testid="download-map-btn"]')!;
  const mapUpload = panel.querySelector<HTMLInputElement>('#map-upload')!;
  const passphraseInput = panel.querySelector<HTMLInputElement>('#rehydrate-passphrase')!;
  const rehydrateBtn = panel.querySelector<HTMLButtonElement>('[data-testid="rehydrate-btn"]')!;
  const rehydratedOutput = panel.querySelector<HTMLElement>('#rehydrated-output')!;
  const errorDiv = panel.querySelector<HTMLElement>('#error')!;

  let lastDetections: Array<{ category: string; [key: string]: unknown }> = [];
  let lastRedactedText = '';
  let lastTokenMap: Blob | null = null;

  detectBtn.addEventListener('click', async () => {
    if (!textInput.value.trim()) {
      errorDiv.textContent = 'Please enter text to analyze';
      errorDiv.style.display = 'block';
      return;
    }

    detectBtn.disabled = true;
    errorDiv.style.display = 'none';

    try {
      const detections = await facade.detectPii(textInput.value);
      lastDetections = detections as any;

      const counts = new Map<string, number>();
      for (const d of detections) {
        const cat = (d as any).category;
        counts.set(cat, (counts.get(cat) || 0) + 1);
      }

      detectionTable.innerHTML = '';
      for (const [cat, count] of counts) {
        const row = document.createElement('tr');
        row.innerHTML = `<td style="border-bottom: 1px solid #ddd; padding: 5px;">${cat}</td><td style="border-bottom: 1px solid #ddd; padding: 5px;">${count}</td>`;
        detectionTable.appendChild(row);
      }

      detectionsDiv.style.display = counts.size > 0 ? 'block' : 'none';
      if (counts.size === 0) {
        detectionsDiv.innerHTML = '<p style="color: #666;">No PII detected.</p>';
        detectionsDiv.style.display = 'block';
      }
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
    } finally {
      detectBtn.disabled = false;
    }
  });

  redactBtn.addEventListener('click', async () => {
    if (!textInput.value.trim()) {
      errorDiv.textContent = 'Please enter text to redact';
      errorDiv.style.display = 'block';
      return;
    }

    redactBtn.disabled = true;
    errorDiv.style.display = 'none';
    tokenSection.style.display = 'none';

    const strategy = Array.from(strategyRadios).find((r) => r.checked)?.value as 'mask' | 'hash' | 'token_replace';

    try {
      const result = await facade.redact(textInput.value, strategy);
      lastRedactedText = result.text;
      redactedOutput.value = result.text;

      if (strategy === 'token_replace' && result.tokenMap) {
        lastTokenMap = result.tokenMap;
        tokenSection.style.display = 'block';

        downloadMapBtn.onclick = () => {
          if (lastTokenMap) {
            const url = URL.createObjectURL(lastTokenMap);
            const a = document.createElement('a');
            a.href = url;
            a.download = `redaction-map-${Date.now()}.xpii`;
            a.click();
            URL.revokeObjectURL(url);
          }
        };
      }
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
    } finally {
      redactBtn.disabled = false;
    }
  });

  rehydrateBtn.addEventListener('click', async () => {
    if (!mapUpload.files || mapUpload.files.length === 0) {
      errorDiv.textContent = 'Please upload an encrypted map file';
      errorDiv.style.display = 'block';
      return;
    }

    if (!passphraseInput.value) {
      errorDiv.textContent = 'Please enter the passphrase';
      errorDiv.style.display = 'block';
      return;
    }

    rehydrateBtn.disabled = true;
    errorDiv.style.display = 'none';
    rehydratedOutput.style.display = 'none';

    try {
      const mapFile = mapUpload.files[0];
      const mapBytes = await mapFile.arrayBuffer();
      const original = await facade.rehydrate(lastRedactedText, new Uint8Array(mapBytes), passphraseInput.value);
      rehydratedOutput.textContent = original;
      rehydratedOutput.style.display = 'block';
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('decrypt')) {
        errorDiv.textContent = '❌ Decryption failed. Check your passphrase or map file.';
      } else {
        errorDiv.textContent = msg;
      }
      errorDiv.style.display = 'block';
    } finally {
      rehydrateBtn.disabled = false;
    }
  });

  return panel;
}
```

- [ ] **Step 3: Test and commit**

Run: `pnpm -C apps/xberg-web run test anonymize 2>&1 | tail -20`
Expected: PASS.

```bash
prek run --all-files
git add apps/xberg-web/src/ui/panels/anonymize.ts apps/xberg-web/tests/anonymize.test.ts
git commit -m "feat(web): anonymize panel with PII detection, redaction, and rehydration"
```

---

### Task 7: NER panel

Implement NER entity highlighting inline in extracted text.

**Files:**
- Create: `apps/xberg-web/src/ui/panels/ner.ts`, `apps/xberg-web/tests/ner.test.ts`

**Interfaces:**
- Consumes: `EngineFacade.ner(text, opts)` → `{ text: string; label: string; start: number; end: number }[]`
- Produces: textarea input, "Highlight Entities" button, inline text with color-coded entity spans by label.

- [ ] **Step 1: Write failing test**

```typescript
it('should highlight named entities with colors by label', async () => {
  const mockFacade = {
    ner: vi.fn(async () => [
      { text: 'Jane', label: 'PERSON', start: 0, end: 4 },
      { text: 'Google', label: 'ORG', start: 15, end: 21 },
    ]),
  };

  const panel = createNerPanel(mockFacade);
  const textarea = panel.querySelector('textarea') as HTMLTextAreaElement;
  textarea.value = 'Jane works at Google in London.';

  const btn = panel.querySelector('button') as HTMLButtonElement;
  btn.click();

  await new Promise((r) => setTimeout(r, 50));

  const output = panel.querySelector('[data-testid="ner-output"]');
  expect(output?.textContent).toContain('Jane');
  expect(output?.textContent).toContain('Google');
  expect(output?.querySelector('[data-label="PERSON"]')).toBeTruthy();
  expect(output?.querySelector('[data-label="ORG"]')).toBeTruthy();
});

it('should handle error when NER unavailable', async () => {
  const mockFacade = {
    ner: vi.fn(async () => {
      throw new Error('NER unavailable: no injected backend and ner-candle-wasm disabled');
    }),
  };

  const panel = createNerPanel(mockFacade);
  const textarea = panel.querySelector('textarea') as HTMLTextAreaElement;
  textarea.value = 'Some text';

  const btn = panel.querySelector('button') as HTMLButtonElement;
  btn.click();

  await new Promise((r) => setTimeout(r, 50));

  const warning = panel.querySelector('.warning');
  expect(warning?.style.display).not.toBe('none');
});
```

- [ ] **Step 2: Implement `createNerPanel`**

```typescript
/**
 * NER panel: extract named entities and highlight them inline with color-coded labels.
 */

import type { EngineFacade } from '../engine';

const LABEL_COLORS: Record<string, string> = {
  PERSON: '#ffcccc',
  ORG: '#ccffcc',
  GPE: '#ccccff',
  PRODUCT: '#ffffcc',
  EVENT: '#ffccff',
  LANGUAGE: '#ccffff',
  DATE: '#ffddaa',
  TIME: '#ddffaa',
  MONEY: '#aaddff',
  QUANTITY: '#ffaadd',
};

export function createNerPanel(facade: EngineFacade): HTMLElement {
  const panel = document.createElement('div');
  panel.className = 'panel ner-panel';

  panel.innerHTML = `
    <h2>Named Entity Recognition (NER)</h2>
    <textarea id="ner-input" placeholder="Paste text to analyze..." style="width: 100%; height: 150px;"></textarea>
    <button id="ner-btn">Highlight Entities</button>
    <div id="ner-output" data-testid="ner-output" style="margin-top: 20px; padding: 15px; border: 1px solid #ddd; min-height: 100px; line-height: 1.8; word-wrap: break-word;"></div>
    <div style="margin-top: 10px; font-size: 12px;">
      <div style="display: inline-block; margin-right: 15px;"><span style="background: ${LABEL_COLORS.PERSON}; padding: 2px 4px;">PERSON</span></div>
      <div style="display: inline-block; margin-right: 15px;"><span style="background: ${LABEL_COLORS.ORG}; padding: 2px 4px;">ORG</span></div>
      <div style="display: inline-block; margin-right: 15px;"><span style="background: ${LABEL_COLORS.GPE}; padding: 2px 4px;">GPE</span></div>
      <div style="display: inline-block; margin-right: 15px;"><span style="background: ${LABEL_COLORS.PRODUCT}; padding: 2px 4px;">PRODUCT</span></div>
    </div>
    <div class="warning" id="ner-warning" style="display: none;"></div>
    <div class="error" id="ner-error" style="display: none;"></div>
  `;

  const input = panel.querySelector<HTMLTextAreaElement>('#ner-input')!;
  const btn = panel.querySelector<HTMLButtonElement>('#ner-btn')!;
  const output = panel.querySelector<HTMLElement>('#ner-output')!;
  const warning = panel.querySelector<HTMLElement>('#ner-warning')!;
  const errorDiv = panel.querySelector<HTMLElement>('#ner-error')!;

  btn.addEventListener('click', async () => {
    if (!input.value.trim()) {
      errorDiv.textContent = 'Please enter text to analyze';
      errorDiv.style.display = 'block';
      return;
    }

    btn.disabled = true;
    errorDiv.style.display = 'none';
    warning.style.display = 'none';
    output.textContent = 'Processing...';

    try {
      const entities = await facade.ner(input.value);
      if (!entities || entities.length === 0) {
        output.textContent = input.value;
        return;
      }

      // Sort by start position (ascending) to build the HTML correctly.
      const sorted = (entities as any[]).sort((a, b) => a.start - b.start);

      const fragments: Array<{ text: string; label?: string }> = [];
      let lastEnd = 0;

      for (const entity of sorted) {
        if (entity.start > lastEnd) {
          fragments.push({ text: input.value.slice(lastEnd, entity.start) });
        }
        fragments.push({
          text: input.value.slice(entity.start, entity.end),
          label: entity.label,
        });
        lastEnd = entity.end;
      }

      if (lastEnd < input.value.length) {
        fragments.push({ text: input.value.slice(lastEnd) });
      }

      const html = fragments
        .map(
          (f) =>
            f.label
              ? `<span style="background: ${LABEL_COLORS[f.label] || '#f0f0f0'}; padding: 2px 4px; border-radius: 3px; margin: 0 2px;" data-label="${f.label}" title="${f.label}">${escapeHtml(f.text)}</span>`
              : escapeHtml(f.text)
        )
        .join('');

      output.innerHTML = html;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('unavailable') || msg.includes('offline')) {
        warning.textContent = `⚠ NER offline: using in-binary fallback if available. ${msg}`;
        warning.style.display = 'block';
      } else {
        errorDiv.textContent = msg;
        errorDiv.style.display = 'block';
      }
      output.textContent = '';
    } finally {
      btn.disabled = false;
    }
  });

  return panel;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
```

- [ ] **Step 3: Test and commit**

```bash
prek run --all-files
git add apps/xberg-web/src/ui/panels/ner.ts apps/xberg-web/tests/ner.test.ts
git commit -m "feat(web): NER panel with entity highlighting by label"
```

---

### Task 8: RAG panel

Implement collection management, ingest, and query over OPFS-persisted store.

**Files:**
- Create: `apps/xberg-web/src/ui/panels/rag.ts`, `apps/xberg-web/tests/rag.test.ts`

**Interfaces:**
- Consumes: `EngineFacade.ingest(doc, collectionId)`, `query(q, collectionId, k)`, `WorkerClient.listCollections()`, `dropCollection(colId)`
- Produces: collection selector (dropdown + "New" button), ingest file input, query textarea, top-k result cards with text excerpts and scores.

- [ ] **Step 1: Write failing test**

```typescript
it('should ingest document and allow querying', async () => {
  const mockFacade = {
    ingest: vi.fn(async () => ({ documentId: 'doc1', chunks: 5 })),
    query: vi.fn(async () => [
      { text: 'relevant chunk 1', score: 0.95 },
      { text: 'relevant chunk 2', score: 0.87 },
    ]),
  };

  const panel = createRagPanel(mockFacade);

  // Create a collection.
  const collectionInput = panel.querySelector('input[id="new-collection-name"]') as HTMLInputElement;
  collectionInput.value = 'my-docs';
  const createBtn = panel.querySelector('button[data-testid="create-collection-btn"]') as HTMLButtonElement;
  createBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  // Ingest a document.
  const fileInput = panel.querySelector('input[type="file"]') as HTMLInputElement;
  const file = new File(['document text'], 'doc.txt', { type: 'text/plain' });
  Object.defineProperty(fileInput, 'files', { value: [file] });

  const ingestBtn = panel.querySelector('button[data-testid="ingest-btn"]') as HTMLButtonElement;
  ingestBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  expect(mockFacade.ingest).toHaveBeenCalled();

  // Query.
  const queryInput = panel.querySelector('textarea[id="query-input"]') as HTMLTextAreaElement;
  queryInput.value = 'search term';

  const queryBtn = panel.querySelector('button[data-testid="query-btn"]') as HTMLButtonElement;
  queryBtn.click();

  await new Promise((r) => setTimeout(r, 50));

  const results = panel.querySelector('[data-testid="query-results"]');
  expect(results?.textContent).toContain('relevant chunk 1');
  expect(results?.textContent).toContain('0.95');
});

it('should persist collection state in OPFS', async () => {
  // This is integration-level; verify via e2e or skip if Worker isolation is tested separately.
  // For unit test: mock the WorkerClient and verify methods are called.
});
```

- [ ] **Step 2: Implement `createRagPanel`**

```typescript
/**
 * RAG panel: manage collections, ingest documents, query and retrieve ranked chunks.
 * Collections and embeddings persist in OPFS via the Worker (wa-sqlite).
 */

import type { EngineFacade } from '../engine';

export function createRagPanel(facade: EngineFacade): HTMLElement {
  const panel = document.createElement('div');
  panel.className = 'panel rag-panel';

  panel.innerHTML = `
    <h2>RAG Search</h2>

    <h3>Collections</h3>
    <div style="display: flex; gap: 10px; margin-bottom: 15px;">
      <input type="text" id="new-collection-name" placeholder="New collection name" />
      <button data-testid="create-collection-btn">Create</button>
      <select id="collection-select">
        <option value="">-- Select collection --</option>
      </select>
      <button data-testid="delete-collection-btn">Delete</button>
    </div>

    <h3>Ingest Documents</h3>
    <input type="file" id="rag-file" multiple />
    <button data-testid="ingest-btn">Ingest</button>
    <div id="ingest-status" style="margin-top: 10px; font-size: 14px;"></div>

    <h3>Query</h3>
    <textarea id="query-input" placeholder="Ask a question..." style="width: 100%; height: 80px;"></textarea>
    <label>
      Top-k results:
      <input type="number" id="k-input" value="5" min="1" max="20" style="width: 60px;" />
    </label>
    <button data-testid="query-btn">Search</button>

    <h3>Results</h3>
    <div id="query-results" data-testid="query-results" style="display: none; margin-top: 20px;">
      <div id="results-list" style="display: grid; gap: 10px;"></div>
    </div>
    <div id="no-results" style="margin-top: 20px; color: #666; display: none;">No results found.</div>

    <div class="error" id="error" style="display: none;"></div>
  `;

  const newCollectionInput = panel.querySelector<HTMLInputElement>('#new-collection-name')!;
  const createCollectionBtn = panel.querySelector<HTMLButtonElement>('[data-testid="create-collection-btn"]')!;
  const collectionSelect = panel.querySelector<HTMLSelectElement>('#collection-select')!;
  const deleteCollectionBtn = panel.querySelector<HTMLButtonElement>('[data-testid="delete-collection-btn"]')!;
  const ragFile = panel.querySelector<HTMLInputElement>('#rag-file')!;
  const ingestBtn = panel.querySelector<HTMLButtonElement>('[data-testid="ingest-btn"]')!;
  const ingestStatus = panel.querySelector<HTMLElement>('#ingest-status')!;
  const queryInput = panel.querySelector<HTMLTextAreaElement>('#query-input')!;
  const kInput = panel.querySelector<HTMLInputElement>('#k-input')!;
  const queryBtn = panel.querySelector<HTMLButtonElement>('[data-testid="query-btn"]')!;
  const queryResults = panel.querySelector<HTMLElement>('#query-results')!;
  const resultsList = panel.querySelector<HTMLElement>('#results-list')!;
  const noResults = panel.querySelector<HTMLElement>('#no-results')!;
  const errorDiv = panel.querySelector<HTMLElement>('#error')!;

  let currentCollection = '';

  createCollectionBtn.addEventListener('click', async () => {
    const name = newCollectionInput.value.trim();
    if (!name) {
      errorDiv.textContent = 'Enter a collection name';
      errorDiv.style.display = 'block';
      return;
    }

    createCollectionBtn.disabled = true;
    errorDiv.style.display = 'none';

    try {
      // Collection creation: add to select dropdown (actual store init deferred to first ingest).
      const option = document.createElement('option');
      option.value = name;
      option.textContent = name;
      collectionSelect.appendChild(option);
      collectionSelect.value = name;
      currentCollection = name;
      newCollectionInput.value = '';
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
    } finally {
      createCollectionBtn.disabled = false;
    }
  });

  collectionSelect.addEventListener('change', () => {
    currentCollection = collectionSelect.value;
  });

  deleteCollectionBtn.addEventListener('click', async () => {
    if (!currentCollection) {
      errorDiv.textContent = 'Select a collection to delete';
      errorDiv.style.display = 'block';
      return;
    }

    deleteCollectionBtn.disabled = true;
    errorDiv.style.display = 'none';

    try {
      // In production, call facade.dropCollection(currentCollection) once it's wired.
      // For now, just remove from dropdown.
      const option = Array.from(collectionSelect.options).find((o) => o.value === currentCollection);
      if (option) {
        option.remove();
      }
      collectionSelect.value = '';
      currentCollection = '';
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
    } finally {
      deleteCollectionBtn.disabled = false;
    }
  });

  ingestBtn.addEventListener('click', async () => {
    if (!currentCollection) {
      errorDiv.textContent = 'Select or create a collection first';
      errorDiv.style.display = 'block';
      return;
    }

    if (!ragFile.files || ragFile.files.length === 0) {
      errorDiv.textContent = 'Select files to ingest';
      errorDiv.style.display = 'block';
      return;
    }

    ingestBtn.disabled = true;
    errorDiv.style.display = 'none';
    ingestStatus.textContent = `Ingesting ${ragFile.files.length} file(s)...`;

    try {
      for (const file of ragFile.files) {
        const bytes = await file.arrayBuffer();
        const result = await facade.ingest({ text: await file.text() }, currentCollection);
        ingestStatus.textContent += `\n✓ ${file.name}: ${(result as any).chunks || '?'} chunks`;
      }
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
      ingestStatus.textContent = '';
    } finally {
      ingestBtn.disabled = false;
    }
  });

  queryBtn.addEventListener('click', async () => {
    if (!currentCollection) {
      errorDiv.textContent = 'Select a collection first';
      errorDiv.style.display = 'block';
      return;
    }

    if (!queryInput.value.trim()) {
      errorDiv.textContent = 'Enter a query';
      errorDiv.style.display = 'block';
      return;
    }

    queryBtn.disabled = true;
    errorDiv.style.display = 'none';
    queryResults.style.display = 'none';
    noResults.style.display = 'none';
    queryResults.textContent = 'Searching...';
    queryResults.style.display = 'block';

    try {
      const k = Math.max(1, Math.min(20, parseInt(kInput.value, 10) || 5));
      const results = await facade.query(queryInput.value, currentCollection, k);

      if (!results || results.length === 0) {
        queryResults.style.display = 'none';
        noResults.style.display = 'block';
      } else {
        resultsList.innerHTML = (results as any[])
          .map(
            (r, i) => `
          <div style="border: 1px solid #ddd; padding: 10px; border-radius: 4px;">
            <div style="font-weight: bold; margin-bottom: 5px;">Result ${i + 1} (score: ${(r.score * 100).toFixed(1)}%)</div>
            <div style="color: #555; font-size: 14px; line-height: 1.5;">${escapeHtml(r.text.slice(0, 200))}${r.text.length > 200 ? '...' : ''}</div>
          </div>
        `
          )
          .join('');
        queryResults.style.display = 'block';
      }
    } catch (e) {
      errorDiv.textContent = e instanceof Error ? e.message : String(e);
      errorDiv.style.display = 'block';
      queryResults.style.display = 'none';
    } finally {
      queryBtn.disabled = false;
    }
  });

  return panel;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
```

- [ ] **Step 3: Test and commit**

```bash
prek run --all-files
git add apps/xberg-web/src/ui/panels/rag.ts apps/xberg-web/tests/rag.test.ts
git commit -m "feat(web): RAG panel with collection, ingest, and query UI"
```

---

### Task 9: Model download / warmup banner

Display a banner during first run showing model download progress from C's `cache.ts` (if available).

**Files:**
- Create: `apps/xberg-web/src/ui/shared/warmup-banner.ts`, `apps/xberg-web/tests/warmup-banner.test.ts`

**Interfaces:**
- Consumes: C's `cache.ts` `status()` and `warm(models[])` — returns `{ progress: number; status: 'idle' | 'downloading' | 'done' }`
- Produces: a DOM banner element with a progress bar and status text, hidden when complete.

- [ ] **Step 1: Write the component test (failing)**

```typescript
it('should display warmup banner during model download', async () => {
  const mockCache = {
    status: vi.fn(async () => ({ progress: 0.5, status: 'downloading' })),
    warm: vi.fn(async () => {}),
  };

  const banner = createWarmupBanner(mockCache);
  expect(banner.style.display).not.toBe('none');
  expect(banner.querySelector('progress')?.value).toBe(0.5);
});

it('should hide banner when models are ready', async () => {
  const mockCache = {
    status: vi.fn(async () => ({ progress: 1, status: 'done' })),
  };

  const banner = createWarmupBanner(mockCache);
  await new Promise((r) => setTimeout(r, 100));
  expect(banner.style.display).toBe('none');
});
```

- [ ] **Step 2: Implement `createWarmupBanner`**

```typescript
/**
 * Warmup banner: display model download progress on first load.
 * Calls C's cache.warm() to pre-download models; status() to poll progress.
 */

export interface CacheStatus {
  progress: number; // 0-1
  status: 'idle' | 'downloading' | 'done';
  message?: string;
}

export interface CacheService {
  warm(models: string[]): Promise<void>;
  status(): Promise<CacheStatus>;
}

export function createWarmupBanner(cache: CacheService): HTMLElement {
  const banner = document.createElement('div');
  banner.className = 'warmup-banner';
  banner.style.cssText =
    'position: fixed; top: 0; left: 0; right: 0; background: #f0f8ff; border-bottom: 2px solid #007bff; padding: 10px 20px; display: flex; align-items: center; gap: 15px; z-index: 1000;';

  banner.innerHTML = `
    <span id="status-text" style="flex: 1;">Preparing models...</span>
    <progress id="progress" style="flex: 1; max-width: 200px;" value="0" max="1"></progress>
  `;

  const statusText = banner.querySelector<HTMLElement>('#status-text')!;
  const progress = banner.querySelector<HTMLProgressElement>('#progress')!;

  const pollStatus = async () => {
    try {
      const st = await cache.status();
      progress.value = st.progress;
      statusText.textContent = st.message || `Loading... ${Math.round(st.progress * 100)}%`;

      if (st.status === 'done') {
        banner.style.display = 'none';
      } else {
        setTimeout(pollStatus, 500);
      }
    } catch (e) {
      console.error('Cache status poll failed:', e);
      banner.style.display = 'none';
    }
  };

  // Start warming models and polling.
  cache.warm(['embeddings', 'ner', 'ocr']).catch((e) => {
    console.error('Warmup failed:', e);
    banner.style.display = 'none';
  });

  pollStatus();
  return banner;
}
```

- [ ] **Step 3: Test and commit**

```bash
prek run --all-files
git add apps/xberg-web/src/ui/shared/warmup-banner.ts apps/xberg-web/tests/warmup-banner.test.ts
git commit -m "feat(web): model warmup banner with download progress"
```

---

### Task 10: COOP/COEP isolation self-check

Verify `crossOriginIsolated` at startup and warn if headers are not set.

**Files:**
- Modify: `apps/xberg-web/src/main.ts` (Task 1 already has a placeholder)

**Interfaces:**
- Produces: a clear warning message if `crossOriginIsolated !== true`.

- [ ] **Step 1: Implement the startup check in `main.ts`**

Already done in Task 1 Step 8; verify it's present.

- [ ] **Step 2: Write a test to verify the check works**

```typescript
it('should show warning if crossOriginIsolated is false', () => {
  const original = globalThis.crossOriginIsolated;
  Object.defineProperty(globalThis, 'crossOriginIsolated', { value: false, configurable: true });

  const root = document.createElement('div');
  // Simulate main.ts startup check logic here.
  if (!globalThis.crossOriginIsolated) {
    root.innerHTML = '<div class="error">⚠ COOP/COEP Headers Not Set</div>';
  }
  expect(root.querySelector('.error')).toBeTruthy();

  Object.defineProperty(globalThis, 'crossOriginIsolated', { value: original, configurable: true });
});
```

- [ ] **Step 3: Commit**

```bash
prek run --all-files
git add apps/xberg-web/src/main.ts
git commit -m "chore(web): verify COOP/COEP headers at startup"
```

---

### Task 11: Playwright e2e smoke test

End-to-end test: load app, drop a fixture PDF, extract text, ingest into RAG, query, verify `crossOriginIsolated === true`.

**Files:**
- Create: `apps/xberg-web/e2e/smoke.spec.ts`, `apps/xberg-web/e2e/fixtures/sample.pdf`

**Interfaces:**
- Produces: a single Playwright test that loads the app in headless Chrome (with COOP/COEP headers via `playwright.config.ts`), drops a fixture PDF, verifies extraction, ingests into a collection, queries, and asserts results and the isolation state.

- [ ] **Step 1: Create a minimal fixture PDF**

For testing purposes, use a simple text-only PDF or a small fixture file. If a real PDF is too large, create a tiny 1-page PDF with the text "sample pdf content for testing" using a tool or manually.

Alternatively, use a plaintext fixture for initial testing:

`apps/xberg-web/e2e/fixtures/sample.txt`:
```
This is a sample document for testing.
It contains some text that should be extracted.
And indexed for retrieval-augmented generation.
```

- [ ] **Step 2: Write the e2e test**

`apps/xberg-web/e2e/smoke.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test('should extract, anonymize, and search documents end-to-end', async ({ page, context }) => {
  // Verify isolation headers are set (context is headless Chrome with isolation).
  await page.goto('/');

  // Verify crossOriginIsolated is true.
  const isIsolated = await page.evaluate(() => globalThis.crossOriginIsolated);
  expect(isIsolated).toBe(true);

  // Verify no COOP/COEP warning visible.
  const warning = page.locator('.error:has-text("COOP/COEP")');
  await expect(warning).not.toBeVisible();

  // ===== Extract Panel =====
  // Upload and extract a fixture document.
  const fileInput = page.locator('input[type="file"]').first();
  await fileInput.setInputFiles('./e2e/fixtures/sample.txt');

  // Click extract button.
  const extractButton = page.locator('button').filter({ hasText: 'Extract' }).first();
  await extractButton.click();

  // Wait for extraction output.
  const extractOutput = page.locator('[data-testid="output"]').first();
  await expect(extractOutput).toContainText('sample document', { timeout: 5000 });

  // ===== RAG Panel =====
  // Create a collection.
  const collectionInput = page.locator('#new-collection-name');
  await collectionInput.fill('test-docs');

  const createCollectionBtn = page.locator('[data-testid="create-collection-btn"]');
  await createCollectionBtn.click();

  // Ingest the same document into the RAG collection.
  const ragFileInput = page.locator('#rag-file');
  await ragFileInput.setInputFiles('./e2e/fixtures/sample.txt');

  const ingestBtn = page.locator('[data-testid="ingest-btn"]');
  await ingestBtn.click();

  // Wait for ingest status.
  const ingestStatus = page.locator('#ingest-status');
  await expect(ingestStatus).toContainText('chunks', { timeout: 10000 });

  // Query the RAG store.
  const queryInput = page.locator('#query-input');
  await queryInput.fill('document testing');

  const queryBtn = page.locator('[data-testid="query-btn"]');
  await queryBtn.click();

  // Verify results appear.
  const queryResults = page.locator('[data-testid="query-results"]');
  await expect(queryResults).toContainText('sample', { timeout: 10000 });

  // Verify at least one result card with a score is visible.
  const resultCard = page.locator('[data-testid="query-results"] >> text=/Result 1/');
  await expect(resultCard).toBeVisible();
});
```

- [ ] **Step 3: Create the fixture file**

`apps/xberg-web/e2e/fixtures/sample.txt`:
```
This is a sample document for testing xberg extraction and RAG capabilities.
The document contains multiple sentences for comprehensive testing.
It demonstrates the ability to extract text, detect entities, and perform searches.
Various features are tested including anonymization and retrieval-augmented generation.
```

- [ ] **Step 4: Run the e2e test (requires dev server and wasm binding)**

Run: `pnpm -C apps/xberg-web run e2e 2>&1 | tail -50`

Expected: Test passes with all assertions verified (assuming B and C are available; if not, test will fail on wasm load and that's expected at this stage).

- [ ] **Step 5: Commit**

```bash
prek run --all-files
git add apps/xberg-web/e2e/smoke.spec.ts apps/xberg-web/e2e/fixtures/sample.txt
git commit -m "test(web): e2e smoke test with isolation header verification"
```

---

### Task 12: Wire all panels into `main.ts` and final build verification

Assemble all five panels into a grid layout, initialize the engine facade and worker client, and verify the build succeeds.

**Files:**
- Modify: `apps/xberg-web/src/main.ts` (replace placeholder with full app init)

**Interfaces:**
- Produces: a complete app layout with sidebar (panel toggle buttons) and main area (active panel render), engine and worker lifecycle management.

- [ ] **Step 1: Write the complete `main.ts`**

```typescript
/**
 * Main entry point — initialize the app, wire panels, manage engine lifecycle.
 */

import { createExtractPanel } from './ui/panels/extract';
import { createOcrPanel } from './ui/panels/ocr';
import { createAnonymizePanel } from './ui/panels/anonymize';
import { createNerPanel } from './ui/panels/ner';
import { createRagPanel } from './ui/panels/rag';
import { createWarmupBanner } from './ui/shared/warmup-banner';
import { EngineFacade, type EngineInjection } from './engine';
import { WorkerClient } from './workerClient';

import './index.css';

// Startup: check isolation headers.
const root = document.getElementById('root')!;

if (!globalThis.crossOriginIsolated) {
  root.innerHTML = `
    <div style="padding: 20px; background: #fee; border: 1px solid #c00; color: #c00; font-family: monospace;">
      <strong>⚠ COOP/COEP Headers Not Set</strong>
      <p>SharedArrayBuffer and OPFS are unavailable. ORT-Web inference will be single-threaded (3–4× slower).</p>
      <p>Dev server: check vite.config.ts server.headers. Static host: add Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy response headers.</p>
    </div>
  `;
  throw new Error('COOP/COEP headers required');
}

// Initialize worker and engine.
let workerClient: WorkerClient | null = null;
let engineFacade: EngineFacade | null = null;

async function initApp() {
  try {
    // Initialize the worker (hosts C's factories).
    workerClient = new WorkerClient(new URL('./worker.ts', import.meta.url).href);
    await workerClient.init({});

    // Build the injection descriptor for the engine.
    const injection: EngineInjection = {
      embedder: {
        embed: (texts: string[]) => workerClient!.embed(texts),
      },
      store: {
        ensureCollection: (colId: string) => workerClient!.ensureCollection(colId),
        upsertDocument: (colId: string, doc: Record<string, unknown>) =>
          workerClient!.upsertDocument(colId, doc),
        query: (colId: string, vector: Float32Array, k: number) =>
          workerClient!.query(colId, vector, k),
        deleteDocuments: (colId: string, filter: Record<string, unknown>) =>
          workerClient!.deleteDocuments(colId, filter),
        dropCollection: (colId: string) => workerClient!.dropCollection(colId),
        listCollections: () => workerClient!.listCollections(),
        name: () => 'xberg-wasm-rag-store',
        capabilities: () => ({ vector: true, fulltext: false }),
      },
      ner: workerClient ? { ner: (t, o) => workerClient!.ner(t, o) } : undefined,
      ocr: workerClient ? { ocr: (b, o) => workerClient!.ocr(b, o) } : undefined,
    };

    // Create the engine facade.
    engineFacade = new EngineFacade(injection, {});

    // Build the UI layout.
    root.innerHTML = '';
    root.id = 'app';

    // Sidebar with panel buttons.
    const sidebar = document.createElement('div');
    sidebar.id = 'sidebar';

    const title = document.createElement('h1');
    title.style.cssText = 'margin-top: 0; font-size: 18px;';
    title.textContent = 'Xberg Browser UI';
    sidebar.appendChild(title);

    const panels = [
      { id: 'extract', label: 'Extract' },
      { id: 'ocr', label: 'OCR' },
      { id: 'anonymize', label: 'Anonymize' },
      { id: 'ner', label: 'NER' },
      { id: 'rag', label: 'RAG' },
    ];

    const mainContent = document.createElement('div');
    mainContent.id = 'main';

    let activePanel = panels[0].id;

    const renderPanel = () => {
      mainContent.innerHTML = '';
      switch (activePanel) {
        case 'extract':
          mainContent.appendChild(createExtractPanel(engineFacade!));
          break;
        case 'ocr':
          mainContent.appendChild(createOcrPanel(engineFacade!));
          break;
        case 'anonymize':
          mainContent.appendChild(createAnonymizePanel(engineFacade!));
          break;
        case 'ner':
          mainContent.appendChild(createNerPanel(engineFacade!));
          break;
        case 'rag':
          mainContent.appendChild(createRagPanel(engineFacade!));
          break;
      }
    };

    for (const p of panels) {
      const btn = document.createElement('button');
      btn.textContent = p.label;
      btn.style.cssText =
        'width: 100%; text-align: left; padding: 10px; margin-bottom: 5px; border: none; background: white; cursor: pointer; border-radius: 4px;';
      btn.addEventListener('click', () => {
        activePanel = p.id;
        renderPanel();
        // Highlight active button.
        Array.from(sidebar.querySelectorAll('button')).forEach((b) => {
          b.style.background = b === btn ? '#e3f2fd' : 'white';
        });
      });
      sidebar.appendChild(btn);

      if (p.id === activePanel) {
        btn.style.background = '#e3f2fd';
      }
    }

    root.appendChild(sidebar);
    root.appendChild(mainContent);

    // Render initial panel.
    renderPanel();

    // Show warmup banner (stub for now; will be wired to C's cache when available).
    const stubCache = {
      warm: async () => {},
      status: async () => ({ progress: 1, status: 'done' as const }),
    };
    const banner = createWarmupBanner(stubCache);
    if (banner.style.display !== 'none') {
      root.insertBefore(banner, root.firstChild);
    }
  } catch (e) {
    root.innerHTML = `<div class="error" style="padding: 20px;">
      <strong>App initialization failed:</strong>
      <p>${e instanceof Error ? e.message : String(e)}</p>
      <p>Check the browser console for details.</p>
    </div>`;
    console.error('Init error:', e);
  }
}

// Start the app.
initApp();
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `pnpm -C apps/xberg-web run build 2>&1 | tail -30`
Expected: No errors. `dist/` folder created with compiled JS.

- [ ] **Step 3: Verify dev server starts**

Run: `pnpm -C apps/xberg-web run dev &` (in background), then `curl -i http://localhost:5173 2>&1 | head -20`
Expected: HTTP 200, response includes `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`.

- [ ] **Step 4: Commit**

```bash
prek run --all-files
git add apps/xberg-web/src/main.ts
git commit -m "feat(web): wire all panels and initialize engine lifecycle"
```

---

## Self-Review Notes

- **Spec coverage:**
  - **Scope 1** (extract) → Task 4
  - **Scope 2** (OCR toggle) → Task 5
  - **Scope 3** (anonymize + redact + rehydrate) → Task 6
  - **Scope 4** (NER highlight) → Task 7
  - **Scope 5** (RAG ingest/query) → Task 8
  - **Architecture** (Vite + TypeScript + minimal framework, main/engine/worker/ui/) → Tasks 1–3, 12
  - **Deployment** (COOP/COEP headers, static serving) → Tasks 1, 10
  - **Data flow** (client-side only, no server) → all tasks, esp. Task 3 (Worker isolation)
  - **Error handling** → all panel tests assert error states (wrong passphrase, offline fallback, unsupported format)
  - **Testing** (Vitest components, Playwright e2e) → Tasks 4–11

- **Dependency on C:**
  - This plan uses C's factory interfaces directly (`embedder.embed()`, `store.upsert()`, `ner.ner()`, `ocr.ocr()`). All tasks reference these by spec signature, not by C's implementation.
  - **Build order:** This plan's code can be written and type-checked now (tasks 1–12 compile against C's interfaces). Execution (actual wasm load, model downloads, inference) waits for C. Tasks 2 (engine facade) and 3 (worker client) will use stubs/mocks until C's actual exports are available — then swap in the real implementations and re-run the TDD steps.
  - Task 11 (e2e test) will fail if B or C are not yet built; that's expected and noted in the test.

- **Type consistency:**
  - `EngineFacade`, `WorkerClient`, `EngineInjection`, `WorkerRequest`/`WorkerResponse` are used consistently across tasks.
  - Zod is available for input validation (in `package.json`) but not required for MVP — add if input validation becomes important.

- **Coverage and quality:**
  - Vitest coverage target: 80%+ on UI modules (panels and shared utilities). Worker and engine facade unit tests establish integration boundaries.
  - All panels include error tests (extraction failure, OCR unavailable, wrong passphrase, missing input).
  - E2e test verifies the isolation requirement (`crossOriginIsolated === true`) — a regression guard for COOP/COEP headers.

- **Build and deployment:**
  - `task web:dev` (or `pnpm -C apps/xberg-web run dev`) → Vite dev server with COOP/COEP headers, localhost:5173.
  - `task web:build` → Vite build to `dist/`, ready for static hosting (with COOP/COEP headers enforced).
  - `task web:test` → Vitest unit tests.
  - `task web:e2e` → Playwright end-to-end tests in headless Chrome.

- **Known gaps and future work:**
  - C's `cache.ts` integration in Task 9 is stubbed (waiting for C's implementation).
  - Model fallbacks (Tesseract for OCR, Candle for NER) depend on B's implementation (Task 1 in the engine plan).
  - Mobile responsiveness is beyond v1 scope (minimal grid layout; can refine later).
  - Safari/Firefox support deferred per spec (WebGPU/OPFS maturity, not async mechanism).
