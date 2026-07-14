import { env } from "@huggingface/transformers";
import type { CacheConfig } from "./types.js";

export function defaultNodeCachePath(): string {
	const home = process.env.USERPROFILE ?? process.env.HOME ?? ".";
	if (process.platform === "win32") {
		return `${process.env.LOCALAPPDATA ?? `${home}/AppData/Local`}/xberg`;
	}
	return `${home}/.cache/xberg`;
}

export function configureTransformersEnvironment(config?: CacheConfig): void {
	// Browser: point onnxruntime-web at self-hosted runtime files when the
	// host app provides a same-origin location for them. transformers.js
	// defaults `wasmPaths` to the jsdelivr CDN, which breaks on
	// crossOriginIsolated pages: ORT's threaded runtime spawns its pthread
	// worker pool via `new Worker(new URL(import.meta.url))`, and with CDN
	// wasmPaths that URL is cross-origin -> SecurityError swallowed by the
	// Emscripten bootstrap -> pipeline() hangs forever with no console or
	// network signal. Same-origin wasmPaths is the fix, not an optimization.
	if (config?.wasmPaths) {
		const onnxWasm = (
			env.backends?.onnx as
				| { wasm?: { wasmPaths?: string | { mjs?: string; wasm?: string } } }
				| undefined
		)?.wasm;
		if (onnxWasm) {
			// Prefer the explicit { mjs, wasm } object form (absolute URLs) over
			// the string-prefix form: ORT checks the object override BEFORE its
			// `import.meta.url`-relative resolution, which is the only hook that
			// survives webpack bundling (webpack rewrites ORT's internal dynamic
			// import so the string prefix is never consulted -- observed as zero
			// requests to the self-hosted directory despite wasmPaths being set).
			const origin = (globalThis as { location?: { href: string } }).location?.href;
			if (origin) {
				const base = new URL(config.wasmPaths, origin);
				onnxWasm.wasmPaths = {
					mjs: new URL("ort-wasm-simd-threaded.jsep.mjs", base).href,
					wasm: new URL("ort-wasm-simd-threaded.jsep.wasm", base).href,
				};
			} else {
				onnxWasm.wasmPaths = config.wasmPaths;
			}
			console.debug(`[runtime-env] ORT wasmPaths ->`, onnxWasm.wasmPaths);
		}
	}
	if (typeof process === "undefined" || !process.versions?.node) return;
	env.cacheDir = config?.nodeCachePath ?? defaultNodeCachePath();
}
