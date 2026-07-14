import { env } from "@huggingface/transformers";

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

// onnxruntime-web's threaded WASM backend pre-spawns a pthread-style Worker
// pool as part of session creation, before any model file is fetched. In
// some sandboxed/automated browser contexts that bootstrap itself can hang
// indefinitely -- confirmed NOT caused by WebGPU detection (see above, that
// case now resolves to "wasm" quickly), missing COOP/COEP (crossOriginIsolated
// was true), network egress (fetch to the model host worked directly), or
// basic Worker creation (a bare postMessage round-trip worked). The hang
// happens with zero console output and zero network requests, before
// onnxruntime-web logs anything -- consistent with the worker-pool handshake
// itself stalling. Race pipeline() against a timeout and, on timeout, retry
// once forced onto onnxruntime-web's single-threaded WASM path (no worker
// pool to bootstrap) so a stalled threaded backend degrades instead of
// hanging the caller forever. Threaded WASM stays the default everywhere
// this actually works -- this is a fallback, not a blanket disable.
const PIPELINE_INIT_TIMEOUT_MS = 30_000;

/**
 * Create a transformers.js pipeline, falling back to onnxruntime-web's
 * single-threaded WASM path if the (preferred) threaded backend's own
 * worker-pool bootstrap doesn't complete within `PIPELINE_INIT_TIMEOUT_MS`.
 */
export async function createPipelineWithFallback<T>(
	createPipeline: (backend: ModelBackend) => Promise<T>,
	backend: ModelBackend,
	label: string,
): Promise<T> {
	if (backend.device !== "wasm" && backend.device !== "webgpu") {
		return createPipeline(backend);
	}

	const timeout = new Promise<"timeout">((resolve) => {
		setTimeout(() => resolve("timeout"), PIPELINE_INIT_TIMEOUT_MS);
	});

	const result = await Promise.race([createPipeline(backend), timeout]);
	if (result !== "timeout") {
		return result;
	}

	console.warn(
		`[backend] ${label} pipeline init exceeded ${PIPELINE_INIT_TIMEOUT_MS}ms on device=${backend.device}` +
			" -- retrying on single-threaded WASM (threaded backend's worker-pool bootstrap may be stalled)",
	);
	// Merely switching `device` back to "wasm" isn't enough to change anything
	// if that's already what was tried -- threading is a separate ORT config
	// axis. Force the single-threaded, non-proxied path (no worker pool to
	// bootstrap) so the retry can't hit the same stall.
	const wasmConfig = env.backends?.onnx?.wasm as { numThreads?: number; proxy?: boolean } | undefined;
	if (wasmConfig) {
		wasmConfig.numThreads = 1;
		wasmConfig.proxy = false;
	}
	return createPipeline({ device: "wasm", dtype: "q8" });
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
