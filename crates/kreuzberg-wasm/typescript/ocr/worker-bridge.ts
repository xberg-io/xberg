/**
 * OCR Worker Bridge
 *
 * Manages the OCR worker lifecycle and provides an async interface for running
 * ocrRecognize() off the main thread. Falls back to direct (blocking) invocation
 * when workers are not available (e.g. edge runtimes).
 */

import { isNode } from "../runtime.js";

interface PendingRequest {
	resolve: (text: string) => void;
	reject: (error: Error) => void;
}

/** Abstraction over Node.js Worker and browser Worker */
interface WorkerHandle {
	postMessage(data: unknown): void;
	terminate(): void | Promise<unknown>;
}

let workerHandle: WorkerHandle | null = null;
const pendingRequests: Map<number, PendingRequest> = new Map();
let nextRequestId = 0;
let workerReady = false;
let readyResolve: (() => void) | null = null;
let readyReject: ((error: Error) => void) | null = null;

/** Whether the worker failed to initialize and we should use the direct fallback. */
let useFallback = false;

/** Direct (blocking) fallback function, set by the enabler when creating the backend. */
let fallbackFn: ((imageData: Uint8Array, tessdata: Uint8Array, language: string) => string) | null = null;

async function cleanupWorker(): Promise<void> {
	if (workerHandle) {
		await workerHandle.terminate();
		workerHandle = null;
	}
	workerReady = false;
}

function handleWorkerMessage(msg: Record<string, unknown>): void {
	switch (msg["type"]) {
		case "ready":
			workerReady = true;
			readyResolve?.();
			readyResolve = null;
			readyReject = null;
			break;
		case "init-error":
			readyReject?.(new Error(msg["error"] as string));
			readyResolve = null;
			readyReject = null;
			break;
		case "result": {
			const id = msg["id"] as number;
			const pending = pendingRequests.get(id);
			if (pending) {
				pendingRequests.delete(id);
				pending.resolve(msg["text"] as string);
			}
			break;
		}
		case "error": {
			const id = msg["id"] as number;
			const pending = pendingRequests.get(id);
			if (pending) {
				pendingRequests.delete(id);
				pending.reject(new Error(msg["error"] as string));
			}
			break;
		}
	}
}

/**
 * Create and initialize the OCR worker.
 *
 * @param wasmGluePath - Absolute path (Node.js) or URL (browser) to the WASM glue JS module
 * @param wasmBinary - Pre-loaded WASM binary (Node.js file-system loaded bytes)
 * @param directFallback - Direct blocking function used when workers are unavailable
 */
export async function createOcrWorker(
	wasmGluePath: string,
	wasmBinary: Uint8Array | undefined,
	directFallback: (imageData: Uint8Array, tessdata: Uint8Array, language: string) => string,
): Promise<void> {
	fallbackFn = directFallback;

	if (workerHandle) return;

	const readyPromise = new Promise<void>((resolve, reject) => {
		readyResolve = resolve;
		readyReject = reject;
	});

	try {
		if (isNode()) {
			await createNodeWorker(wasmGluePath, wasmBinary);
		} else if (typeof Worker !== "undefined") {
			await createBrowserWorker(wasmGluePath, wasmBinary);
		} else {
			// No worker support — use direct fallback
			useFallback = true;
			return;
		}

		// Timeout prevents indefinite hang if the worker loads but never sends "ready"
		// (e.g. WASM module import fails silently inside the worker)
		const timeoutMs = 30_000;
		const timeout = new Promise<void>((_, reject) => {
			setTimeout(() => reject(new Error("OCR worker initialization timed out")), timeoutMs);
		});
		await Promise.race([readyPromise, timeout]);
	} catch {
		// Worker creation or init failed — fall back to direct calls
		await cleanupWorker();
		useFallback = true;
	}
}

async function createNodeWorker(wasmGluePath: string, wasmBinary: Uint8Array | undefined): Promise<void> {
	const { Worker } = await import(/* @vite-ignore */ "node:worker_threads");
	const nodePath = await import(/* @vite-ignore */ "node:path");
	const nodeUrl = await import(/* @vite-ignore */ "node:url");

	const __dirname = nodePath.dirname(nodeUrl.fileURLToPath(import.meta.url));
	const workerPath = nodePath.join(__dirname, "ocr-worker.js");

	const worker = new Worker(workerPath, {
		workerData: { wasmGluePath, wasmBinary },
	});

	worker.on("message", (msg: Record<string, unknown>) => handleWorkerMessage(msg));
	worker.on("error", (err: Error) => {
		// Reject all pending requests
		for (const pending of pendingRequests.values()) {
			pending.reject(err);
		}
		pendingRequests.clear();
		readyReject?.(err);
	});

	workerHandle = {
		postMessage: (data: unknown) => worker.postMessage(data),
		terminate: () => worker.terminate(),
	};
}

async function createBrowserWorker(wasmGluePath: string, wasmBinary: Uint8Array | undefined): Promise<void> {
	const workerUrl = new URL("./ocr-worker.js", import.meta.url);
	const worker = new Worker(workerUrl, { type: "module" });

	worker.onmessage = (e: MessageEvent) => handleWorkerMessage(e.data as Record<string, unknown>);
	worker.onerror = (e: ErrorEvent) => {
		const err = new Error(e.message);
		for (const pending of pendingRequests.values()) {
			pending.reject(err);
		}
		pendingRequests.clear();
		readyReject?.(err);
	};

	workerHandle = {
		postMessage: (data: unknown) => worker.postMessage(data),
		terminate: () => worker.terminate(),
	};

	// Browser worker needs an init message (no workerData equivalent)
	worker.postMessage({
		type: "init",
		wasmGluePath,
		wasmBinary,
	});
}

/**
 * Run OCR in the worker thread. Returns a Promise that resolves with the recognized text.
 * Falls back to direct (blocking) call if workers are unavailable.
 */
export function runOcrInWorker(imageData: Uint8Array, tessdata: Uint8Array, language: string): Promise<string> {
	if (useFallback || !workerHandle || !workerReady) {
		if (fallbackFn) {
			try {
				const text = fallbackFn(imageData, tessdata, language);
				return Promise.resolve(text);
			} catch (e: unknown) {
				return Promise.reject(e instanceof Error ? e : new Error(String(e)));
			}
		}
		return Promise.reject(new Error("OCR worker not initialized and no fallback available"));
	}

	const id = nextRequestId++;
	return new Promise<string>((resolve, reject) => {
		pendingRequests.set(id, { resolve, reject });
		workerHandle!.postMessage({
			type: "ocr",
			id,
			imageData,
			tessdata,
			language,
		});
	});
}

/**
 * Check whether OCR is using the worker thread or the direct fallback.
 */
export function isUsingWorker(): boolean {
	return workerHandle !== null && workerReady && !useFallback;
}

/**
 * Terminate the OCR worker and clean up resources.
 */
export async function terminateOcrWorker(): Promise<void> {
	if (workerHandle) {
		await workerHandle.terminate();
		workerHandle = null;
	}
	workerReady = false;
	useFallback = false;
	fallbackFn = null;

	// Reject any still-pending requests
	for (const pending of pendingRequests.values()) {
		pending.reject(new Error("OCR worker terminated"));
	}
	pendingRequests.clear();
}
