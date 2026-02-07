#!/usr/bin/env tsx
import * as readline from "readline";
import * as path from "path";
import { extractFile, initWasm, ExtractionConfig } from "@kreuzberg/wasm";

interface ExtractionOutput {
	content: string;
	metadata: Record<string, unknown>;
	_extraction_time_ms: number;
	_batch_total_ms?: number;
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
	".yaml": "application/yaml",
	".yml": "application/yaml",
	".toml": "application/toml",
	".csv": "text/csv",
	".tsv": "text/tab-separated-values",
	".eml": "message/rfc822",
	".msg": "application/vnd.ms-outlook",
	".svg": "image/svg+xml",
	".pdf": "application/pdf",
	".docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
	".xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
	".pptx": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
	".rtf": "application/rtf",
	".epub": "application/epub+zip",
	".rst": "text/x-rst",
	".org": "text/x-org",
	".bib": "application/x-bibtex",
	".tex": "application/x-latex",
	".latex": "application/x-latex",
	".ipynb": "application/x-ipynb+json",
	".typst": "application/x-typst",
	".typ": "application/x-typst",
	".djot": "text/djot",
};

function guessMimeType(filePath: string): string | null {
	const ext = path.extname(filePath).toLowerCase();
	return MIME_MAP[ext] ?? null;
}

function createConfig(ocrEnabled: boolean): ExtractionConfig {
	return {
		useCache: false,
		...(ocrEnabled && { ocr: { enabled: true } }),
	};
}

async function extractAsync(filePath: string, ocrEnabled: boolean): Promise<ExtractionOutput> {
	const config = createConfig(ocrEnabled);
	const mimeType = guessMimeType(filePath);
	const start = performance.now();
	const result = await extractFile(filePath, mimeType, config);
	const durationMs = performance.now() - start;

	return {
		content: result.content,
		metadata: (result.metadata as Record<string, unknown>) ?? {},
		_extraction_time_ms: durationMs,
	};
}

async function extractBatch(filePaths: string[], ocrEnabled: boolean): Promise<ExtractionOutput[]> {
	const config = createConfig(ocrEnabled);
	const start = performance.now();
	const results = await Promise.all(
		filePaths.map((fp) => extractFile(fp, guessMimeType(fp), config)),
	);
	const totalDurationMs = performance.now() - start;

	const perFileDurationMs = filePaths.length > 0 ? totalDurationMs / filePaths.length : 0;

	return results.map((result) => ({
		content: result.content,
		metadata: (result.metadata as Record<string, unknown>) ?? {},
		_extraction_time_ms: perFileDurationMs,
		_batch_total_ms: totalDurationMs,
	}));
}

async function runServer(ocrEnabled: boolean): Promise<void> {
	const rl = readline.createInterface({
		input: process.stdin,
		output: process.stdout,
		terminal: false,
	});

	for await (const line of rl) {
		const filePath = line.trim();
		if (!filePath) {
			continue;
		}
		const start = performance.now();
		try {
			const payload = await extractAsync(filePath, ocrEnabled);
			console.log(JSON.stringify(payload));
		} catch (err) {
			const durationMs = performance.now() - start;
			const error = err as Error;
			console.log(JSON.stringify({ error: error.message, _extraction_time_ms: durationMs }));
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

	// Initialize WASM BEFORE timing measurement
	await initWasm();

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
