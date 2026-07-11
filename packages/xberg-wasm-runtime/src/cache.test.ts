import { afterEach, describe, it, expect, vi } from "vitest";
import { CacheManager } from "./cache";
import { join } from "node:path";
import { existsSync, mkdirSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";

describe("cache manager", () => {
	afterEach(() => vi.unstubAllGlobals());
	it("reports initial cache status", async () => {
		const manager = new CacheManager();
		const status = await manager.status();
		expect(status).toHaveProperty("cached");
		expect(status).toHaveProperty("size");
		expect(Array.isArray(status.cached)).toBe(true);
	});

	it("tracks model availability", async () => {
		const manager = new CacheManager();
		const status = await manager.status();
		// No models cached initially (or may find system defaults)
		expect(typeof status.size).toBe("number");
		expect(status.size).toBeGreaterThanOrEqual(0);
	});

	it("accepts custom cache directory", async () => {
		const customDir = "/custom/cache/path";
		const manager = new CacheManager(customDir);
		// Verify it was set (via status call which uses the directory)
		const status = await manager.status();
		expect(status).toHaveProperty("cached");
	});

	it("setWasmPaths handles missing window gracefully", () => {
		const manager = new CacheManager();
		// In Node environment, window is undefined
		// setWasmPaths should not throw
		expect(() => manager.setWasmPaths("/some/path")).not.toThrow();
	});

	it("sets ORT WASM paths in a browser runtime", () => {
		const browserWindow = { ort: { env: { wasm: { wasmPaths: "" } } } };
		vi.stubGlobal("window", browserWindow);
		new CacheManager().setWasmPaths("/assets/ort/");
		expect(browserWindow.ort.env.wasm.wasmPaths).toBe("/assets/ort/");
	});

	it("reports an empty browser OPFS cache until browser status I/O is available", async () => {
		vi.stubGlobal("window", {});
		await expect(new CacheManager().status()).resolves.toEqual({ cached: [], size: 0 });
	});

	it("accepts an empty legacy model list without initializing pipelines", async () => {
		await expect(new CacheManager().warm([])).resolves.toEqual({ success: [], failed: [] });
	});

	it("reports transformers.js artifacts from its actual cache layout", async () => {
		const directory = join(tmpdir(), `xberg-status-${Date.now()}`);
		const embedderDirectory = join(directory, "Xenova", "bge-m3", "onnx");
		const nerDirectory = join(directory, "Xenova", "bert-base-NER", "onnx");
		mkdirSync(embedderDirectory, { recursive: true });
		mkdirSync(nerDirectory, { recursive: true });
		writeFileSync(join(embedderDirectory, "model_quantized.onnx"), Buffer.alloc(32));
		writeFileSync(join(nerDirectory, "model_quantized.onnx"), Buffer.alloc(64));

		const status = await new CacheManager(directory).status();
		expect(status.cached).toEqual(["Embedder (bge-m3)", "BERT NER"]);
		expect(status.size).toBe(96);
	});
});

describe("CacheManager.warm", () => {
	it.runIf(process.env.XBERG_RUN_MODEL_DOWNLOAD_TESTS === "1")(
		"downloads and caches embedding, NER, and OCR model artifacts",
		async () => {
			const dir = join(tmpdir(), `xberg-warm-${Date.now()}`);
			const mgr = new CacheManager(dir);
			const phases: string[] = [];
			const result = await mgr.warm({ onProgress: (p) => phases.push(p) });
			expect(result).toEqual({ success: ["embedding", "ner", "ocr"], failed: [] });
			expect(phases).toEqual(["embedding", "ner", "ocr"]);
			expect(existsSync(dir)).toBe(true);
			const files = walkFiles(dir);
			expect(files.length).toBeGreaterThanOrEqual(8);
			expect(files.reduce((total, file) => total + statSync(file).size, 0)).toBeGreaterThan(1_000_000);
		},
		600_000,
	);
});

function walkFiles(directory: string): string[] {
	return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
		const child = join(directory, entry.name);
		return entry.isDirectory() ? walkFiles(child) : [child];
	});
}
