/**
 * Runtime detection of the best available ONNX Runtime Web backend.
 *
 * Preference order (graceful degradation):
 *   1. "webgpu" — if `navigator.gpu` exists AND `requestAdapter()` actually
 *      resolves with a real adapter within `WEBGPU_ADAPTER_TIMEOUT_MS`
 *   2. "webgl"  — if a WebGL rendering context is obtainable from a canvas
 *   3. "wasm"   — always-available CPU fallback
 *
 * Every browser-API access is guarded so this returns "wasm" in Node
 * (where `navigator`, `document`, and `HtmlCanvasElement` are undefined)
 * without throwing.
 */
export type OnnxBackend = "webgpu" | "webgl" | "wasm";
export type ModelBackend = {
	device: "webgpu" | "wasm" | "cpu";
	dtype: "fp32" | "q8";
};

// `navigator.gpu`'s mere presence is not a reliable signal: sandboxed,
// headless, or otherwise constrained browser contexts can expose the API
// while `requestAdapter()` itself hangs forever rather than rejecting (no
// adapter available, but no error either). A model pipeline built on that
// promise then never resolves -- previously observed as `pipeline(...)`
// silently stalling with zero console output and zero network activity.
// Race the real adapter request against a timeout so a non-functional
// WebGPU implementation degrades to the WASM-CPU backend instead of
// hanging the caller indefinitely.
const WEBGPU_ADAPTER_TIMEOUT_MS = 3_000;

export async function selectModelBackend(config?: { forceWasmBackend?: boolean }): Promise<ModelBackend> {
	if (typeof process !== "undefined" && process.versions?.node) {
		return { device: "cpu", dtype: "q8" };
	}
	if (config?.forceWasmBackend) {
		return { device: "wasm", dtype: "q8" };
	}
	const gpu =
		typeof navigator !== "undefined" ? (navigator as Navigator & { gpu?: GPU }).gpu : undefined;
	if (!gpu) {
		return { device: "wasm", dtype: "q8" };
	}
	try {
		const adapter = await Promise.race([
			gpu.requestAdapter(),
			new Promise<null>((resolve) => {
				setTimeout(() => resolve(null), WEBGPU_ADAPTER_TIMEOUT_MS);
			}),
		]);
		if (adapter) return { device: "webgpu", dtype: "fp32" };
	} catch {
		// fall through to wasm
	}
	return { device: "wasm", dtype: "q8" };
}

export function detectBackend(): OnnxBackend {
	if (typeof window === "undefined" || typeof document === "undefined") {
		return "wasm";
	}

	try {
		const nav = globalThis.navigator as {
			gpu?: { requestAdapter?: () => Promise<unknown> };
		};
		// Synchronous check is intentional: the full WebGPU adapter request is
		// awaited lazily by callers. Its mere presence is a strong signal that
		// WebGPU is exposed, which is sufficient for backend selection here.
		if (nav?.gpu?.requestAdapter) {
			return "webgpu";
		}
	} catch {
		// fall through to webgl
	}

	try {
		if (typeof HTMLCanvasElement !== "undefined") {
			const canvas = document.createElement("canvas");
			const gl = canvas.getContext("webgl") ?? canvas.getContext("experimental-webgl");
			if (gl) return "webgl";
		}
	} catch {
		// fall through to wasm
	}

	return "wasm";
}
