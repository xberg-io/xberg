import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
	initialize: vi.fn(async () => undefined),
	changeDetectionModel: vi.fn(async () => undefined),
	changeRecognitionModel: vi.fn(async () => undefined),
	changeTextDictionary: vi.fn(async () => undefined),
	recognize: vi.fn(async () => ({ text: "", lines: [] })),
}));

vi.mock("ppu-paddle-ocr", () => ({
	PaddleOcrService: class {
		initialize = service.initialize;
		changeDetectionModel = service.changeDetectionModel;
		changeRecognitionModel = service.changeRecognitionModel;
		changeTextDictionary = service.changeTextDictionary;
		recognize = service.recognize;
	},
	V4_EN_MOBILE_MODEL: { detection: "en-det", recognition: "en-rec", charactersDictionary: "en-dict" },
	V6_SMALL_MODEL: { detection: "default-det", recognition: "default-rec", charactersDictionary: "default-dict" },
}));

import { createOcr } from "./ocr.js";

describe("OCR language model switching", () => {
	beforeEach(() => vi.clearAllMocks());

	it("returns to the default model after a language-specific request", async () => {
		const ocr = await createOcr();
		expect(ocr).not.toBeNull();
		if (!ocr) return;

		await ocr.ocr(new Uint8Array(), { languages: ["en"] });
		await ocr.ocr(new Uint8Array());

		expect(service.changeDetectionModel.mock.calls).toEqual([["en-det"], ["default-det"]]);
		expect(service.changeRecognitionModel.mock.calls).toEqual([["en-rec"], ["default-rec"]]);
		expect(service.changeTextDictionary.mock.calls).toEqual([["en-dict"], ["default-dict"]]);
	});
});
