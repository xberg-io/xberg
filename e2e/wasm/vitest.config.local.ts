// Local vitest configuration to handle WASM imports
import { defineConfig, mergeConfig } from "vitest/config";
import baseConfig from "./vitest.config";

export default mergeConfig(
	baseConfig,
	defineConfig({
		test: {
			// Use the default Node.js test environment with WASM support
			env: {
				// Mock out WASI imports
				env: JSON.stringify({}),
			},
		},
		resolve: {
			alias: {
				env: new URL("./wasi-polyfill.js", import.meta.url).pathname,
				wasi_snapshot_preview1: new URL("./wasi-polyfill.js", import.meta.url).pathname,
			},
		},
	}),
);
