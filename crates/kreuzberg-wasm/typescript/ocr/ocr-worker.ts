/**
 * OCR Worker Script
 *
 * Runs ocrRecognize() in a worker thread so the main event loop stays responsive.
 * Supports both Node.js worker_threads and browser Web Workers.
 *
 * Protocol:
 * - Node.js: receives init data via workerData, auto-initializes WASM
 * - Browser: waits for { type: 'init', wasmGluePath, wasmBinary? } message
 * - Posts { type: 'ready' } when WASM is initialized
 * - Receives { type: 'ocr', id, imageData, tessdata, language }
 * - Posts { type: 'result', id, text } or { type: 'error', id, error }
 */

interface OcrWasm {
	ocrRecognize(imageBytes: Uint8Array, tessdata: Uint8Array, language: string): string;
	default(input?: BufferSource | WebAssembly.Module): Promise<void>;
}

let wasm: OcrWasm | null = null;

let postFn: (data: unknown) => void = () => {
	throw new Error("Worker message handler not initialized");
};

async function initWasm(wasmGluePath: string, wasmBinary?: ArrayBufferLike): Promise<void> {
	const glue = (await import(/* @vite-ignore */ wasmGluePath)) as OcrWasm;
	if (typeof glue.default === "function") {
		if (wasmBinary) {
			await glue.default(wasmBinary);
		} else {
			await glue.default();
		}
	}
	wasm = glue;
	postFn({ type: "ready" });
}

function onMessage(msg: Record<string, unknown>): void {
	switch (msg["type"]) {
		case "init":
			initWasm(msg["wasmGluePath"] as string, msg["wasmBinary"] as ArrayBufferLike | undefined).catch((e: unknown) => {
				postFn({ type: "init-error", error: e instanceof Error ? e.message : String(e) });
			});
			break;
		case "ocr": {
			const id = msg["id"] as number;
			if (!wasm) {
				postFn({ type: "error", id, error: "WASM OCR not initialized" });
				return;
			}
			try {
				const text = wasm.ocrRecognize(
					msg["imageData"] as Uint8Array,
					msg["tessdata"] as Uint8Array,
					msg["language"] as string,
				);
				postFn({ type: "result", id, text });
			} catch (e: unknown) {
				postFn({ type: "error", id, error: e instanceof Error ? e.message : String(e) });
			}
			break;
		}
	}
}

async function bootstrap(): Promise<void> {
	const isNodeEnv =
		typeof process !== "undefined" &&
		!!process.versions?.node &&
		typeof (globalThis as Record<string, unknown>).Deno === "undefined";

	if (isNodeEnv) {
		const { parentPort, workerData } = await import(/* @vite-ignore */ "node:worker_threads");
		if (!parentPort) throw new Error("ocr-worker must be run as a worker thread");

		postFn = (data: unknown) => parentPort.postMessage(data);
		parentPort.on("message", (msg: Record<string, unknown>) => onMessage(msg));

		// Node.js: init data arrives via workerData
		const wd = workerData as { wasmGluePath?: string; wasmBinary?: ArrayBufferLike } | null;
		if (wd?.wasmGluePath) {
			await initWasm(wd.wasmGluePath, wd.wasmBinary);
		}
	} else {
		// Browser Web Worker
		const self_ = globalThis as unknown as DedicatedWorkerGlobalScope;
		postFn = (data: unknown) => self_.postMessage(data);
		self_.onmessage = (e: MessageEvent) => onMessage(e.data as Record<string, unknown>);
		// Browser: waits for 'init' message from main thread
	}
}

bootstrap().catch((e: unknown) => {
	try {
		if (typeof process !== "undefined" && process.stderr) {
			process.stderr.write(`[ocr-worker] bootstrap failed: ${e}\n`);
			process.exit(1);
		}
	} catch {
		// Nothing we can do
	}
});
