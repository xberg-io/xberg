#!/usr/bin/env tsx
/**
 * Kreuzberg TypeScript/Node.js extraction wrapper for benchmark harness.
 *
 * Supports four modes:
 * - async: extractFile() - asynchronous extraction
 * - batch: batchExtractFiles() - batch extraction for multiple files
 * - async-batch: parallel async extraction
 * - server: persistent mode reading paths from stdin
 */

import * as readline from "node:readline";
import { batchExtractFiles, type ExtractionConfig, extractFile } from "@kreuzberg/node";

interface ExtractionOutput {
	content: string;
	metadata: Record<string, unknown>;
	_extraction_time_ms: number;
	_batch_total_ms?: number;
	_ocr_used: boolean;
	_peak_memory_bytes?: number;
}

/**
 * Determine if OCR was actually used based on extraction result metadata.
 * Mirrors the native Rust adapter logic: OCR is used when format_type is "ocr",
 * or when format_type is "image" and OCR was enabled in config.
 */
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

function determineOcrUsed(metadata: Record<string, unknown>, ocrEnabled: boolean): boolean {
	const formatType = (metadata?.format_type as string) || "";
	if (formatType === "ocr") return true;
	if ((formatType === "image" || formatType === "pdf") && ocrEnabled) return true;
	return false;
}

function createConfig(ocrEnabled: boolean, forceOcr = false): ExtractionConfig {
	return {
		useCache: false,
		...(ocrEnabled && { ocr: { backend: "tesseract", language: "eng" } }),
		...(forceOcr && { forceOcr: true }),
	};
}

async function extractAsync(filePath: string, ocrEnabled: boolean, forceOcr = false): Promise<ExtractionOutput> {
	const config = createConfig(ocrEnabled, forceOcr);
	const start = performance.now();
	const result = await extractFile(filePath, config);
	const durationMs = performance.now() - start;

	const metadata = result.metadata || {};
	return {
		content: result.content,
		metadata,
		_extraction_time_ms: durationMs,
		_ocr_used: determineOcrUsed(metadata as Record<string, unknown>, ocrEnabled || forceOcr),
		_peak_memory_bytes: process.memoryUsage().rss,
	};
}

async function extractBatch(filePaths: string[], ocrEnabled: boolean): Promise<ExtractionOutput[]> {
	const config = createConfig(ocrEnabled);
	const start = performance.now();
	const results = await batchExtractFiles(filePaths, config);
	const totalDurationMs = performance.now() - start;

	const perFileDurationMs = filePaths.length > 0 ? totalDurationMs / filePaths.length : 0;

	const peakMemory = process.memoryUsage().rss;
	return results.map((result) => {
		const metadata = result.metadata || {};
		return {
			content: result.content,
			metadata,
			_extraction_time_ms: perFileDurationMs,
			_batch_total_ms: totalDurationMs,
			_ocr_used: determineOcrUsed(metadata as Record<string, unknown>, ocrEnabled),
			_peak_memory_bytes: peakMemory,
		};
	});
}

async function extractAsyncBatch(filePaths: string[], ocrEnabled: boolean): Promise<ExtractionOutput[]> {
	const start = performance.now();
	const promises = filePaths.map((fp) => extractAsync(fp, ocrEnabled));
	const results = await Promise.all(promises);
	const totalDurationMs = performance.now() - start;

	const peakMemory = process.memoryUsage().rss;
	return results.map((result) => ({
		...result,
		_batch_total_ms: totalDurationMs,
		_peak_memory_bytes: peakMemory,
	}));
}

async function runServer(ocrEnabled: boolean): Promise<void> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
		terminal: false,
	});

	// Signal readiness after Node + NAPI initialization
	console.log("READY");

	for await (const line of rl) {
		const { path: filePath, forceOcr } = parseRequest(line);
		if (!filePath) {
			continue;
		}
		const start = performance.now();
		try {
			const payload = await extractAsync(filePath, ocrEnabled, forceOcr);
			console.log(JSON.stringify(payload));
		} catch (err) {
			const durationMs = performance.now() - start;
			const error = err as Error;
			console.log(
				JSON.stringify({
					error: error.message,
					_extraction_time_ms: durationMs,
					_ocr_used: false,
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
		console.error("Usage: kreuzberg_extract.ts [--ocr|--no-ocr] <mode> <file_path> [additional_files...]");
		console.error("Modes: async, batch, async-batch, server");
		process.exit(1);
	}

	const mode = args[0];
	const filePaths = args.slice(1);

	try {
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

			if (filePaths.length === 1) {
				console.log(JSON.stringify(results[0]));
			} else {
				console.log(JSON.stringify(results));
			}
		} else if (mode === "async-batch") {
			if (filePaths.length < 1) {
				console.error("Error: async-batch mode requires at least one file");
				process.exit(1);
			}

			const results = await extractAsyncBatch(filePaths, ocrEnabled);

			if (filePaths.length === 1) {
				console.log(JSON.stringify(results[0]));
			} else {
				console.log(JSON.stringify(results));
			}
		} else {
			console.error(`Error: Unknown mode '${mode}'. Use async, batch, async-batch, or server`);
			process.exit(1);
		}
	} catch (err) {
		const error = err as Error;
		console.error(`Error extracting with Kreuzberg: ${error.message}`);
		process.exit(1);
	}
}

main().catch((err) => {
	console.error(err);
	process.exit(1);
});
