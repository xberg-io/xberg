// Auto-generated tests for ocr fixtures.

import { existsSync, readFileSync } from "node:fs";
import type { ExtractionResult } from "@kreuzberg/node";
import { extractFileSync } from "@kreuzberg/node";
import { describe, it } from "vitest";
import { assertions, buildConfig, chunkAssertions, resolveDocument, shouldSkipFixture } from "./helpers.js";

const TEST_TIMEOUT_MS = 60_000;

describe("ocr fixtures", () => {
	it(
		"ocr_image_hello_world",
		() => {
			const documentPath = resolveDocument("images/test_hello_world.png");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_image_hello_world: missing document at", documentPath);
				console.warn("Notes: Requires Tesseract OCR for image text extraction.");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_image_hello_world",
						["tesseract"],
						"Requires Tesseract OCR for image text extraction.",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/png"]);
			assertions.assertMinContentLength(result, 5);
			assertions.assertContentContainsAny(result, ["hello", "world"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_image_no_text",
		() => {
			const documentPath = resolveDocument("images/flower_no_text.jpg");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_image_no_text: missing document at", documentPath);
				console.warn("Notes: Skip when Tesseract is unavailable.");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "ocr_image_no_text", ["tesseract"], "Skip when Tesseract is unavailable.")) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/jpeg"]);
			assertions.assertMaxContentLength(result, 200);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_confidence_filter",
		() => {
			const documentPath = resolveDocument("images/ocr_image.jpg");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_confidence_filter: missing document at", documentPath);
				console.warn("Notes: Tests confidence threshold filtering with PaddleOCR");
				return;
			}
			const config = buildConfig({
				force_ocr: true,
				ocr: { backend: "paddle-ocr", language: "en", paddle_ocr_config: { min_confidence: 80.0 } },
			});
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_confidence_filter",
						["onnxruntime", "paddle-ocr"],
						"Tests confidence threshold filtering with PaddleOCR",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/jpeg"]);
			assertions.assertMinContentLength(result, 1);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_image_chinese",
		() => {
			const documentPath = resolveDocument("images/chi_sim_image.jpeg");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_image_chinese: missing document at", documentPath);
				console.warn("Notes: Requires PaddleOCR with Chinese models");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "paddle-ocr", language: "ch" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_image_chinese",
						["onnxruntime", "paddle-ocr"],
						"Requires PaddleOCR with Chinese models",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/jpeg"]);
			assertions.assertMinContentLength(result, 1);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_image_english",
		() => {
			const documentPath = resolveDocument("images/test_hello_world.png");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_image_english: missing document at", documentPath);
				console.warn("Notes: Requires PaddleOCR with ONNX Runtime");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "paddle-ocr", language: "en" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_image_english",
						["onnxruntime", "paddle-ocr"],
						"Requires PaddleOCR with ONNX Runtime",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/png"]);
			assertions.assertMinContentLength(result, 5);
			assertions.assertContentContainsAny(result, ["hello", "Hello", "world", "World"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_markdown",
		() => {
			const documentPath = resolveDocument("images/test_hello_world.png");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_markdown: missing document at", documentPath);
				console.warn("Notes: Tests markdown output format parity with Tesseract");
				return;
			}
			const config = buildConfig({
				force_ocr: true,
				ocr: { backend: "paddle-ocr", language: "en", paddle_ocr_config: { output_format: "markdown" } },
			});
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_markdown",
						["onnxruntime", "paddle-ocr"],
						"Tests markdown output format parity with Tesseract",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/png"]);
			assertions.assertMinContentLength(result, 5);
			assertions.assertContentContainsAny(result, ["hello", "Hello", "world", "World"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_pdf_scanned",
		() => {
			const documentPath = resolveDocument("pdf/ocr_test.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_pdf_scanned: missing document at", documentPath);
				console.warn("Notes: Requires PaddleOCR with ONNX Runtime");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "paddle-ocr", language: "en" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_pdf_scanned",
						["onnxruntime", "paddle-ocr"],
						"Requires PaddleOCR with ONNX Runtime",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/pdf"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertContentContainsAny(result, ["Docling", "Markdown", "JSON"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_structured",
		() => {
			const documentPath = resolveDocument("images/test_hello_world.png");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_structured: missing document at", documentPath);
				console.warn("Notes: Tests structured output with bbox/confidence preservation");
				return;
			}
			const config = buildConfig({
				force_ocr: true,
				ocr: { backend: "paddle-ocr", element_config: { include_elements: true }, language: "en" },
			});
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_structured",
						["onnxruntime", "paddle-ocr"],
						"Tests structured output with bbox/confidence preservation",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/png"]);
			assertions.assertMinContentLength(result, 5);
			chunkAssertions.assertOcrElements(result, true, true, true, null);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_paddle_table_detection",
		() => {
			const documentPath = resolveDocument("images/simple_table.png");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_paddle_table_detection: missing document at", documentPath);
				console.warn("Notes: Tests table detection capability with PaddleOCR");
				return;
			}
			const config = buildConfig({
				force_ocr: true,
				ocr: { backend: "paddle-ocr", language: "en", paddle_ocr_config: { enable_table_detection: true } },
			});
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_paddle_table_detection",
						["onnxruntime", "paddle-ocr"],
						"Tests table detection capability with PaddleOCR",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["image/png"]);
			assertions.assertMinContentLength(result, 10);
			assertions.assertTableCount(result, 1, null);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_pdf_image_only_german",
		() => {
			const documentPath = resolveDocument("pdf/image_only_german_pdf.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_pdf_image_only_german: missing document at", documentPath);
				console.warn("Notes: Skip if OCR backend unavailable.");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "ocr_pdf_image_only_german", ["tesseract"], "Skip if OCR backend unavailable.")) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/pdf"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertMetadataExpectation(result, "format_type", { eq: "pdf" });
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_pdf_rotated_90",
		() => {
			const documentPath = resolveDocument("pdf/ocr_test_rotated_90.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_pdf_rotated_90: missing document at", documentPath);
				console.warn("Notes: Skip automatically when OCR backend is missing.");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_pdf_rotated_90",
						["tesseract"],
						"Skip automatically when OCR backend is missing.",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/pdf"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"ocr_pdf_tesseract",
		() => {
			const documentPath = resolveDocument("pdf/ocr_test.pdf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping ocr_pdf_tesseract: missing document at", documentPath);
				console.warn("Notes: Skip automatically if OCR backend is unavailable.");
				return;
			}
			const config = buildConfig({ force_ocr: true, ocr: { backend: "tesseract", language: "eng" } });
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (
					shouldSkipFixture(
						error,
						"ocr_pdf_tesseract",
						["tesseract"],
						"Skip automatically if OCR backend is unavailable.",
					)
				) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/pdf"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertContentContainsAny(result, ["Docling", "Markdown", "JSON"]);
		},
		TEST_TIMEOUT_MS,
	);
});
