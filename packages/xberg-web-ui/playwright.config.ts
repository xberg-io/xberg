// playwright.config.ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  // These specs drive the real (non-mocked) wasm engine -- bge-m3 embedding
  // and Candle GLiNER2 NER inference are CPU-bound and expensive. Running
  // more than one at once makes every one of them slower by fighting for
  // the same cores, which can push an otherwise-sufficient per-test timeout
  // past its limit. Force serial execution so timeouts reflect one test's
  // real cost, not N tests' combined cost.
  workers: 1,
  use: { headless: true },
});
