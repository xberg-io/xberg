import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    globals: true,
    environment: "node",
    coverage: {
      provider: "v8",
      reporter: ["text", "json"],
      thresholds: { lines: 80, functions: 80, branches: 75 },
    },
  },
});
