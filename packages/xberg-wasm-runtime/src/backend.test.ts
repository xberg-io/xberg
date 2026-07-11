import { afterEach, describe, it, expect, vi } from "vitest";
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
	afterEach(() => vi.unstubAllGlobals());
	it("selects quantized CPU inference in Node", () => {
		expect(selectModelBackend()).toEqual({ device: "cpu", dtype: "q8" });
	});

	it("selects WebGPU with fp32 in a capable browser", () => {
		vi.stubGlobal("process", undefined);
		vi.stubGlobal("navigator", { gpu: {} });
		expect(selectModelBackend()).toEqual({ device: "webgpu", dtype: "fp32" });
	});

	it("selects quantized WASM in a browser without WebGPU", () => {
		vi.stubGlobal("process", undefined);
		vi.stubGlobal("navigator", {});
		expect(selectModelBackend()).toEqual({ device: "wasm", dtype: "q8" });
	});
});
