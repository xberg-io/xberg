import { defineConfig, configDefaults } from "vitest/config";

// Suites that construct the wasm `XbergEngine` (via initializeEngine) need the
// ~97MB wasm-pack binary (crates/xberg-wasm/pkg/nodejs/xberg_wasm_bg.wasm) plus
// a downloaded HuggingFace embedder model. Neither is available in the
// lightweight MCP CI job, which only commits the TypeScript bindings and builds
// the runtime. Set XBERG_SKIP_WASM_TESTS=1 there to run the pure-logic suites
// only (chunker, detect, eu-*, redaction); locally, where the binary is
// present, the full suite runs.
const skipWasm = process.env.XBERG_SKIP_WASM_TESTS === "1";

const wasmEngineTests = [
  "tests/collections.test.ts",
  "tests/e2e.test.ts",
  "tests/engine.test.ts",
  "tests/ingest.test.ts",
  "tests/pii.test.ts",
  "tests/pii_parity.test.ts",
  "tests/query.test.ts",
  "tests/rehydration_compat.test.ts",
  "tests/tools.test.ts",
];

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    exclude: [...configDefaults.exclude, ...(skipWasm ? wasmEngineTests : [])],
    environment: "node",
    globals: false,
  },
});
