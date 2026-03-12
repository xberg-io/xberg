import { defineConfig } from "tsup";

export default defineConfig({
	entry: {
		index: "typescript/index.ts",
		runtime: "typescript/runtime.ts",
		"adapters/wasm-adapter": "typescript/adapters/wasm-adapter.ts",
		"ocr/registry": "typescript/ocr/registry.ts",
		"ocr/tesseract-wasm-backend": "typescript/ocr/tesseract-wasm-backend.ts",
		// ocr-worker must be at dist root (not dist/ocr/) because worker-bridge.ts
		// references it via "./ocr-worker.js" relative to import.meta.url, and
		// worker-bridge gets bundled into dist/index.js.
		"ocr-worker": "typescript/ocr/ocr-worker.ts",
	},
	// ESM only - CJS is not supported due to top-level await in WASM initialization
	// Modern Node.js (>= 14), Deno, and browsers all support ESM natively
	format: ["esm"],
	bundle: true,
	// Disable tsup's dts bundling - it generates hashed filenames (types-xxx.d.ts)
	// that change on every build. We generate stable .d.ts files using tsc instead.
	dts: false,
	splitting: false,
	sourcemap: true,
	clean: true,
	shims: false,
	platform: "node",
	target: "es2022",
	external: [
		"@kreuzberg/core",
		"tesseract-wasm",
		// WASM module - keep external to avoid bundling
		// The wasm-pack generated module should not be bundled
		"../pkg/kreuzberg_wasm.js",
		"./pkg/kreuzberg_wasm.js",
		"./kreuzberg_wasm.js",
		/\.wasm$/,
		/@kreuzberg\/wasm-.*/,
		"./index.js",
		"../index.js",
		// PDFium module - keep external for runtime resolution
		// In Node.js, loaded from filesystem; in browser, loaded via dynamic import
		"../pdfium.js",
		"./pdfium.js",
		// Node.js built-in modules — preserve node: prefix for Deno compatibility.
		// Without this, tsup strips the node: prefix (e.g. "node:worker_threads" → "worker_threads"),
		// which breaks Deno where the node: prefix is required.
		/^node:/,
		"worker_threads",
	],
});
