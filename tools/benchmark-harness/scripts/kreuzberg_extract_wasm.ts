#!/usr/bin/env tsx
import * as fs from "node:fs";
import * as path from "node:path";
import * as readline from "node:readline";
// Import from the local build path to avoid pnpm workspace resolution issues.
// The @kreuzberg/wasm package is resolved through pnpm's virtual store which
// doesn't reliably provide the WASM binary and glue JS via import.meta.url.
import {
	type ExtractionConfig,
	enableOcr,
	extractFile,
	getWasmModule,
	initializePdfiumAsync,
	initWasm,
} from "../../../crates/kreuzberg-wasm/dist/index.js";

/** Default per-extraction timeout in milliseconds (5 minutes). */
const DEFAULT_TIMEOUT_MS = 300_000;

/** Parse EXTRACTION_TIMEOUT_MS from env, falling back to the default. */
const EXTRACTION_TIMEOUT_MS: number = (() => {
	const env = process.env["EXTRACTION_TIMEOUT_MS"];
	if (env) {
		const parsed = Number.parseInt(env, 10);
		if (!Number.isNaN(parsed) && parsed > 0) return parsed;
	}
	return DEFAULT_TIMEOUT_MS;
})();

/** Whether debug logging is enabled (BENCHMARK_DEBUG env var). */
const DEBUG = !!process.env["BENCHMARK_DEBUG"];

/** Running extraction counter for diagnostics. */
let extractionCount = 0;

function log(msg: string): void {
	if (DEBUG) {
		const mem = (process.memoryUsage().rss / 1024 / 1024).toFixed(0);
		process.stderr.write(`[wasm:${extractionCount}:${mem}MB] ${msg}\n`);
	}
}

interface ExtractionOutput {
	content: string;
	metadata: Record<string, unknown>;
	_extraction_time_ms: number;
	_batch_total_ms?: number;
	_ocr_used: boolean;
	_peak_memory_bytes?: number;
}

/** Map file extension to MIME type so we don't rely on byte-level detection. */
const MIME_MAP: Record<string, string> = {
	".txt": "text/plain",
	".md": "text/markdown",
	".markdown": "text/markdown",
	".commonmark": "text/markdown",
	".html": "text/html",
	".htm": "text/html",
	".xml": "application/xml",
	".json": "application/json",
	".yaml": "application/x-yaml",
	".yml": "application/x-yaml",
	".toml": "application/toml",
	".csv": "text/csv",
	".tsv": "text/tab-separated-values",
	".eml": "message/rfc822",
	".msg": "application/vnd.ms-outlook",
	".svg": "image/svg+xml",
	".pdf": "application/pdf",
	".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
	".doc": "application/msword",
	".xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
	".xlsm": "application/vnd.ms-excel.sheet.macroEnabled.12",
	".xlsb": "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
	".xlam": "application/vnd.ms-excel.addin.macroEnabled.12",
	".xla": "application/vnd.ms-excel",
	".xls": "application/vnd.ms-excel",
	".pptx": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
	".pptm": "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
	".ppsx": "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
	".ppt": "application/vnd.ms-powerpoint",
	".odt": "application/vnd.oasis.opendocument.text",
	".ods": "application/vnd.oasis.opendocument.spreadsheet",
	".rtf": "application/rtf",
	".epub": "application/epub+zip",
	".fb2": "application/x-fictionbook+xml",
	".rst": "text/x-rst",
	".org": "text/x-org",
	".bib": "application/x-bibtex",
	".tex": "application/x-latex",
	".latex": "application/x-latex",
	".ipynb": "application/x-ipynb+json",
	".typst": "application/x-typst",
	".typ": "application/x-typst",
	".djot": "text/x-djot",
	".jpg": "image/jpeg",
	".jpeg": "image/jpeg",
	".png": "image/png",
	".tiff": "image/tiff",
	".tif": "image/tiff",
	".gif": "image/gif",
	".bmp": "image/bmp",
	".webp": "image/webp",
	".jp2": "image/jp2",
	".zip": "application/zip",
	".tar": "application/x-tar",
	".gz": "application/gzip",
	".tgz": "application/gzip",
	".7z": "application/x-7z-compressed",
};

function guessMimeType(filePath: string): string | null {
	const ext = path.extname(filePath).toLowerCase();
	return MIME_MAP[ext] ?? null;
}

/** Image file extensions for OCR detection. */
const IMAGE_EXTENSIONS = new Set([
	".bmp",
	".gif",
	".jpg",
	".jpeg",
	".png",
	".tiff",
	".tif",
	".webp",
	".jp2",
	".jpx",
	".jpm",
	".mj2",
]);

/**
 * Determine if OCR was actually used based on extraction result metadata.
 * Mirrors the native Rust adapter logic.
 */
function determineOcrUsed(metadata: Record<string, unknown>, ocrEnabled: boolean): boolean {
	const formatType = (metadata?.format_type as string) || "";
	if (formatType === "ocr") return true;
	if (formatType === "image" && ocrEnabled) return true;
	if (formatType === "pdf" && ocrEnabled) return true;
	return false;
}

/**
 * Determine OCR status from file path when no metadata is available (error path).
 * When OCR is enabled and the file is an image or PDF, report OCR as used — matching
 * how other bindings behave (they report OCR based on configuration, not outcome).
 */
function determineOcrUsedFromPath(filePath: string, ocrEnabled: boolean): boolean {
	if (!ocrEnabled) return false;
	const ext = path.extname(filePath).toLowerCase();
	if (IMAGE_EXTENSIONS.has(ext)) return true;
	if (ext === ".pdf") return true;
	return false;
}

function createConfig(ocrEnabled: boolean, forceOcr?: boolean): ExtractionConfig {
	return {
		useCache: false,
		...(ocrEnabled && { ocr: { backend: "tesseract" } }),
		...(forceOcr && { forceOcr: true }),
	};
}

/**
 * Race an extraction promise against a timeout.
 * On timeout, returns a rejected promise with a descriptive error including
 * the file path, MIME type, file size, and memory usage for debugging.
 */
async function withTimeout<T>(promise: Promise<T>, filePath: string, mimeType: string | null): Promise<T> {
	let timer: ReturnType<typeof setTimeout> | undefined;
	const timeoutPromise = new Promise<never>((_resolve, reject) => {
		timer = setTimeout(() => {
			const fileSize = (() => {
				try {
					return fs.statSync(filePath).size;
				} catch {
					return -1;
				}
			})();
			const mem = process.memoryUsage();
			reject(
				new Error(
					`Extraction timed out after ${EXTRACTION_TIMEOUT_MS}ms` +
						` | file=${filePath}` +
						` | mime=${mimeType ?? "unknown"}` +
						` | fileSize=${fileSize}` +
						` | rss=${mem.rss}` +
						` | heapUsed=${mem.heapUsed}` +
						` | heapTotal=${mem.heapTotal}` +
						` | extractionCount=${extractionCount}`,
				),
			);
		}, EXTRACTION_TIMEOUT_MS);
	});

	try {
		return await Promise.race([promise, timeoutPromise]);
	} finally {
		if (timer) clearTimeout(timer);
	}
}

async function extractAsync(filePath: string, ocrEnabled: boolean): Promise<ExtractionOutput> {
	const config = createConfig(ocrEnabled);
	const mimeType = guessMimeType(filePath);
	const start = performance.now();
	const result = await withTimeout(extractFile(filePath, mimeType, config), filePath, mimeType);
	const durationMs = performance.now() - start;

	const metadata = (result.metadata as Record<string, unknown>) ?? {};
	return {
		content: result.content,
		metadata,
		_extraction_time_ms: durationMs,
		_ocr_used: determineOcrUsed(metadata, ocrEnabled),
		_peak_memory_bytes: process.memoryUsage().rss,
	};
}

async function extractBatch(filePaths: string[], ocrEnabled: boolean): Promise<ExtractionOutput[]> {
	const config = createConfig(ocrEnabled);
	const start = performance.now();
	const settled = await Promise.allSettled(
		filePaths.map((fp) => {
			const mime = guessMimeType(fp);
			return withTimeout(extractFile(fp, mime, config), fp, mime);
		}),
	);
	const totalDurationMs = performance.now() - start;

	const perFileDurationMs = filePaths.length > 0 ? totalDurationMs / filePaths.length : 0;

	const peakMemory = process.memoryUsage().rss;
	return settled.map((settlement, _i) => {
		if (settlement.status === "rejected") {
			const reason = settlement.reason instanceof Error ? settlement.reason.message : String(settlement.reason);
			return {
				error: reason,
				_extraction_time_ms: 0,
				_ocr_used: false,
			} as unknown as ExtractionOutput;
		}
		const result = settlement.value;
		const metadata = (result.metadata as Record<string, unknown>) ?? {};
		return {
			content: result.content,
			metadata,
			_extraction_time_ms: perFileDurationMs,
			_batch_total_ms: totalDurationMs,
			_ocr_used: determineOcrUsed(metadata, ocrEnabled),
			_peak_memory_bytes: peakMemory,
		};
	});
}

function parseRequest(line: string): { path: string; forceOcr: boolean } {
	const trimmed = line.trim();
	if (trimmed.startsWith("{")) {
		try {
			const req = JSON.parse(trimmed);
			return { path: req.path || "", forceOcr: req.force_ocr || false };
		} catch {
			// Fall through to plain path
		}
	}
	return { path: trimmed, forceOcr: false };
}

async function runServer(ocrEnabled: boolean): Promise<void> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
		terminal: false,
	});

	// Signal readiness after WASM initialization
	console.log("READY");

	for await (const line of rl) {
		const { path: filePath, forceOcr } = parseRequest(line);
		if (!filePath) {
			continue;
		}

		extractionCount++;
		const mimeType = guessMimeType(filePath);
		log(`START ${filePath} (mime=${mimeType ?? "unknown"})`);

		const start = performance.now();
		try {
			const config = createConfig(ocrEnabled || forceOcr, forceOcr);
			const result = await withTimeout(extractFile(filePath, mimeType, config), filePath, mimeType);
			const durationMs = performance.now() - start;

			log(`OK    ${filePath} (${durationMs.toFixed(1)}ms, ${result.content.length} chars)`);

			const metadata = (result.metadata as Record<string, unknown>) ?? {};
			const payload: ExtractionOutput = {
				content: result.content,
				metadata,
				_extraction_time_ms: durationMs,
				_ocr_used: determineOcrUsed(metadata, ocrEnabled || forceOcr),
				_peak_memory_bytes: process.memoryUsage().rss,
			};
			console.log(JSON.stringify(payload));
		} catch (err) {
			const durationMs = performance.now() - start;
			const error = err as Error;

			// Always log failures to stderr (not gated on DEBUG) so CI logs capture them
			process.stderr.write(`[wasm:ERROR] extraction #${extractionCount} failed: ${error.message}\n`);

			console.log(
				JSON.stringify({
					error: error.message,
					_extraction_time_ms: durationMs,
					_ocr_used: determineOcrUsedFromPath(filePath, ocrEnabled || forceOcr),
					_peak_memory_bytes: process.memoryUsage().rss,
				}),
			);
		}
	}
}

async function main(): Promise<void> {
	let ocrEnabled = false;
	const args: string[] = [];

	for (const arg of process.argv.slice(2)) {
		if (arg === "--ocr") {
			ocrEnabled = true;
		} else if (arg === "--no-ocr") {
			ocrEnabled = false;
		} else {
			args.push(arg);
		}
	}

	if (args.length < 1) {
		console.error("Usage: kreuzberg_extract_wasm.ts [--ocr|--no-ocr] <mode> <file_path> [additional_files...]");
		console.error("Modes: async, batch, server");
		process.exit(1);
	}

	process.stderr.write(`[wasm] initializing (timeout=${EXTRACTION_TIMEOUT_MS}ms, debug=${DEBUG})\n`);

	// Initialize WASM BEFORE timing measurement
	await initWasm();

	// Ensure PDFium is fully initialized before processing any files.
	// initWasm() fires off PDFium init asynchronously (fire-and-forget),
	// so we must explicitly await it to avoid empty PDF results.
	const wasmModule = getWasmModule();
	if (wasmModule) {
		try {
			await initializePdfiumAsync(wasmModule);
			process.stderr.write("[wasm] PDFium initialized\n");
		} catch {
			process.stderr.write("[wasm] PDFium not available — PDF extraction disabled\n");
		}
	}

	// Enable OCR backend when requested (required for image extraction)
	if (ocrEnabled) {
		try {
			await enableOcr();
			process.stderr.write("[wasm] OCR enabled\n");
		} catch (err) {
			process.stderr.write(`[wasm] Failed to enable OCR: ${(err as Error).message}\n`);
		}
	}

	process.stderr.write("[wasm] initialization complete\n");

	const mode = args[0];
	const filePaths = args.slice(1);

	if (mode === "server") {
		await runServer(ocrEnabled);
	} else if (mode === "async") {
		if (filePaths.length !== 1) {
			console.error("Error: async mode requires exactly one file");
			process.exit(1);
		}
		const payload = await extractAsync(filePaths[0], ocrEnabled);
		console.log(JSON.stringify(payload));
	} else if (mode === "batch") {
		if (filePaths.length < 1) {
			console.error("Error: batch mode requires at least one file");
			process.exit(1);
		}
		const results = await extractBatch(filePaths, ocrEnabled);
		console.log(JSON.stringify(filePaths.length === 1 ? results[0] : results));
	} else {
		console.error(`Error: Unknown mode '${mode}'. Use async, batch, or server`);
		process.exit(1);
	}
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
