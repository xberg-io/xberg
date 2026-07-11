import { describe, it, expect, beforeAll } from "vitest";
import { createOcr } from "./ocr";
import type { OcrInterface } from "./types";

describe("OCR", () => {
	let ocr: OcrInterface | null;

	beforeAll(async () => {
		ocr = await createOcr();
	}, 120_000);

	it("returns null or an OCR interface", () => {
		// OCR may not be available in all environments (e.g. the optional
		// `onnxruntime-node` peer dependency is not installed); this test
		// allows null, matching the documented optional-injection contract.
		expect(ocr === null || typeof ocr === "object").toBe(true);
	});

	// NOTE: intentionally not `it.skipIf(!ocr)` — `skipIf`'s condition is
	// evaluated synchronously at test-collection time, before the `beforeAll`
	// hook above has run, so `ocr` would still be `undefined` and the test
	// would always be (incorrectly) skipped regardless of whether OCR loads.
	// Checking availability inside the test body (as `ner.test.ts` does for
	// `createNer`) is required to genuinely exercise OCR when it is available.
	it("ocrs a test image", async () => {
		if (!ocr) {
			console.log("[skip] OCR not enabled");
			return;
		}

		// A tiny test PNG (1x1 pixel) as a placeholder fixture
		const pixel = new Uint8Array([
			0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00,
			0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00,
			0x0c, 0x49, 0x44, 0x41, 0x54, 0x08, 0x99, 0x63, 0xf8, 0xcf, 0xc0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0xbf,
			0xd0, 0xba, 0x4d, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
		]);

		const result = await ocr.ocr(pixel);

		expect(result).toHaveProperty("text");
		expect(result).toHaveProperty("lines");
		expect(Array.isArray(result.lines)).toBe(true);
	}, 60_000);

	it("ocrs a synthetic text image and returns the correct result shape", async () => {
		if (!ocr) {
			console.log("[skip] OCR not enabled");
			return;
		}

		// Render real text via ppu-ocv's own (transitively bundled) canvas
		// implementation so the pipeline is exercised end-to-end against a
		// realistic (if synthetic) input, without adding a new dependency.
		const { createRequire } = await import("module");
		const require = createRequire(import.meta.url);
		const ppuOcvEntry = require.resolve("ppu-ocv");
		const nestedRequire = createRequire(ppuOcvEntry);
		const { createCanvas } = nestedRequire("@napi-rs/canvas") as {
			createCanvas: (
				w: number,
				h: number,
			) => {
				getContext: (kind: "2d") => {
					fillStyle: string;
					fillRect: (x: number, y: number, w: number, h: number) => void;
					font: string;
					fillText: (text: string, x: number, y: number) => void;
				};
				toBuffer: (mime: string) => Buffer;
			};
		};

		// The previous fixture (800x200 canvas, bold 100px sans-serif) produced
		// ZERO detected text boxes: the detection model scales the longest side
		// to maxSideLength 640 (rounding to a multiple of 32) and is
		// scale/aspect-sensitive, and the bold `sans-serif` rasterization was
		// not reliably detected or recognized. A 640x120 canvas with a plain
		// (non-bold) 64px Arial glyph matches the model's expected input scale
		// and font rasterization, so detection finds the box and recognition
		// returns "HELLO" through the default multilingual model.
		const canvas = createCanvas(640, 120);
		const ctx = canvas.getContext("2d");
		ctx.fillStyle = "white";
		ctx.fillRect(0, 0, 640, 120);
		ctx.fillStyle = "black";
		ctx.font = "64px Arial";
		ctx.fillText("HELLO", 10, 86);

		const pngBuffer = canvas.toBuffer("image/png");
		const fixtureBytes = new Uint8Array(pngBuffer);
		const result = await ocr.ocr(fixtureBytes);

		expect(result.lines.length).toBeGreaterThan(0);
		expect(result.text.toUpperCase()).toContain("HELLO");
	}, 60_000);

	it("createOcr never throws even when the backend is unavailable", async () => {
		// Regardless of environment, createOcr must resolve (to null or an
		// interface) rather than reject — OCR is optional injection and the
		// wasm engine falls back to in-binary Tesseract OCR when this is null.
		await expect(createOcr()).resolves.not.toThrow();
	});

	it("handles inference errors gracefully", async () => {
		if (!ocr) {
			console.log("[skip] OCR not enabled");
			return;
		}

		// Pass invalid image data to trigger inference error handling
		const invalidImage = new Uint8Array([0xff, 0xd8]); // Incomplete JPEG header
		const result = await ocr.ocr(invalidImage);

		// Should return empty result instead of throwing
		expect(result).toHaveProperty("text");
		expect(result).toHaveProperty("lines");
	}, 60_000);
});
