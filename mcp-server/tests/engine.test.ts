import { describe, it, expect, beforeAll } from "vitest";
import { initializeEngine, getEngine } from "../src/engine.js";

describe("engine initialization", () => {
  beforeAll(async () => {
    // Startup should succeed. First run downloads the embedder model
    // (transformers.js) and loads the ~100MB wasm, so allow a generous budget.
    await initializeEngine();
  }, 180_000);

  it("returns a singleton engine instance", () => {
    const eng = getEngine();
    expect(eng).toBeDefined();
    expect(typeof eng.extract).toBe("function");
    expect(typeof eng.query).toBe("function");
  });

  it("engine has all required methods", () => {
    const eng = getEngine();
    // Real exported method names from B's generated .d.ts (snake_case; there is
    // no `rehydrate` — rehydration is surfaced via `redact`'s rehydrationMap).
    ["extract", "ocr", "detect_pii", "redact", "ner", "ingest", "query"].forEach(
      (method) => {
        expect(typeof eng[method as keyof typeof eng]).toBe("function");
      },
    );
  });

  it("is a singleton (initializeEngine returns the same instance)", async () => {
    const a = getEngine();
    const b = await initializeEngine();
    expect(a).toBe(b);
  });
});
