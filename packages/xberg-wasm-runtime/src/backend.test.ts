import { afterEach, beforeEach, describe, it, expect, vi } from "vitest";
import { detectBackend, selectModelBackend } from "./backend.js";

describe("detectBackend", () => {
	afterEach(() => vi.unstubAllGlobals());
	it("falls back to wasm in node (no browser gpu/webgl)", () => {
		expect(detectBackend()).toBe("wasm");
	});
	it("returns a valid backend union", () => {
		expect(["webgpu", "webgl", "wasm"]).toContain(detectBackend());
	});

	it("selects WebGPU when the browser exposes an adapter API", () => {
		vi.stubGlobal("window", {});
		vi.stubGlobal("document", { createElement: vi.fn() });
		vi.stubGlobal("navigator", { gpu: { requestAdapter: vi.fn() } });
		expect(detectBackend()).toBe("webgpu");
	});

	it("falls back to WebGL when WebGPU is unavailable", () => {
		vi.stubGlobal("window", {});
		vi.stubGlobal("navigator", {});
		vi.stubGlobal("HTMLCanvasElement", function HTMLCanvasElement() {});
		vi.stubGlobal("document", {
			createElement: () => ({ getContext: (kind: string) => (kind === "webgl" ? {} : null) }),
		});
		expect(detectBackend()).toBe("webgl");
	});

	it("falls back to WASM when browser graphics APIs fail", () => {
		vi.stubGlobal("window", {});
		vi.stubGlobal("navigator", {
			get gpu() {
				throw new Error("blocked");
			},
		});
		vi.stubGlobal("HTMLCanvasElement", function HTMLCanvasElement() {});
		vi.stubGlobal("document", {
			createElement: () => {
				throw new Error("blocked");
			},
		});
		expect(detectBackend()).toBe("wasm");
	});
});

describe("selectModelBackend", () => {
	// Vitest's own worker RPC relies on the real global `process` (it calls
	// process.nextTick on later ticks, including mid-await inside these
	// tests). Stubbing `process` itself to `undefined` crashes the test
	// worker as soon as any tick runs during an awaited call. Instead, only
	// hide the one field selectModelBackend actually reads, leaving the real
	// `process` object (and its nextTick) intact throughout.
	let originalNodeVersion: string | undefined;
	beforeEach(() => {
		originalNodeVersion = process.versions.node;
		// @ts-expect-error -- test-only: simulate a browser process.versions shape
		delete process.versions.node;
	});
	afterEach(() => {
		vi.unstubAllGlobals();
		process.versions.node = originalNodeVersion as string;
	});

	it("selects quantized CPU inference in Node", async () => {
		process.versions.node = originalNodeVersion as string;
		expect(await selectModelBackend()).toEqual({ device: "cpu", dtype: "q8" });
	});

	it("selects WebGPU with fp32 when requestAdapter resolves with a real adapter", async () => {
		vi.stubGlobal("navigator", { gpu: { requestAdapter: vi.fn().mockResolvedValue({}) } });
		expect(await selectModelBackend()).toEqual({ device: "webgpu", dtype: "fp32" });
	});

	it("selects quantized WASM in a browser without WebGPU", async () => {
		vi.stubGlobal("navigator", {});
		expect(await selectModelBackend()).toEqual({ device: "wasm", dtype: "q8" });
	});

	it("falls back to WASM when requestAdapter resolves with no adapter", async () => {
		vi.stubGlobal("navigator", { gpu: { requestAdapter: vi.fn().mockResolvedValue(null) } });
		expect(await selectModelBackend()).toEqual({ device: "wasm", dtype: "q8" });
	});

	it("falls back to WASM when requestAdapter throws", async () => {
		vi.stubGlobal("navigator", {
			gpu: { requestAdapter: vi.fn().mockRejectedValue(new Error("blocked")) },
		});
		expect(await selectModelBackend()).toEqual({ device: "wasm", dtype: "q8" });
	});

	it("falls back to WASM when requestAdapter never settles (present-but-non-functional GPU)", async () => {
		// Only fake setTimeout/clearTimeout -- faking the full timer set (the
		// default) also replaces process.nextTick, which Vitest's own worker
		// RPC relies on, and crashes the test worker.
		vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
		vi.stubGlobal("navigator", { gpu: { requestAdapter: vi.fn(() => new Promise(() => {})) } });
		const promise = selectModelBackend();
		await vi.advanceTimersByTimeAsync(3_000);
		expect(await promise).toEqual({ device: "wasm", dtype: "q8" });
		vi.useRealTimers();
	});

	it("respects forceWasmBackend without calling requestAdapter", async () => {
		const requestAdapter = vi.fn();
		vi.stubGlobal("navigator", { gpu: { requestAdapter } });
		expect(await selectModelBackend({ forceWasmBackend: true })).toEqual({ device: "wasm", dtype: "q8" });
		expect(requestAdapter).not.toHaveBeenCalled();
	});
});
