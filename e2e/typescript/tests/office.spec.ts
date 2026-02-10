// Auto-generated tests for office fixtures.

import { existsSync, readFileSync } from "node:fs";
import type { ExtractionResult } from "@kreuzberg/node";
import { extractFileSync } from "@kreuzberg/node";
import { describe, it } from "vitest";
import { assertions, buildConfig, resolveDocument, shouldSkipFixture } from "./helpers.js";

const TEST_TIMEOUT_MS = 60_000;

describe("office fixtures", () => {
	it(
		"office_bibtex_basic",
		() => {
			const documentPath = resolveDocument("bibtex/comprehensive.bib");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_bibtex_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_bibtex_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-bibtex", "text/x-bibtex"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_djot_basic",
		() => {
			const documentPath = resolveDocument("markdown/tables.djot");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_djot_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_djot_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["text/x-djot", "text/djot"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_doc_legacy",
		() => {
			const documentPath = resolveDocument("doc/unit_test_lists.doc");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_doc_legacy: missing document at", documentPath);
				console.warn("Notes: Requires the office feature.");
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_doc_legacy", ["office"], "Requires the office feature.")) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/msword"]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docbook_basic",
		() => {
			const documentPath = resolveDocument("docbook/docbook-reader.docbook");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docbook_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docbook_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/docbook+xml", "text/docbook"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_basic",
		() => {
			const documentPath = resolveDocument("docx/sample_document.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_equations",
		() => {
			const documentPath = resolveDocument("docx/equations.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_equations: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_equations", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_fake",
		() => {
			const documentPath = resolveDocument("docx/fake.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_fake: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_fake", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_formatting",
		() => {
			const documentPath = resolveDocument("docx/unit_test_formatting.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_formatting: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_formatting", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_headers",
		() => {
			const documentPath = resolveDocument("docx/unit_test_headers.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_headers: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_headers", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_lists",
		() => {
			const documentPath = resolveDocument("docx/unit_test_lists.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_lists: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_lists", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_docx_tables",
		() => {
			const documentPath = resolveDocument("docx/docx_tables.docx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_docx_tables: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_docx_tables", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document",
			]);
			assertions.assertMinContentLength(result, 50);
			assertions.assertContentContainsAll(result, [
				"Simple uniform table",
				"Nested Table",
				"merged cells",
				"Header Col",
			]);
			assertions.assertTableCount(result, 1, null);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_epub_basic",
		() => {
			const documentPath = resolveDocument("epub/features.epub");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_epub_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_epub_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/epub+zip"]);
			assertions.assertMinContentLength(result, 50);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_fb2_basic",
		() => {
			const documentPath = resolveDocument("fictionbook/basic.fb2");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_fb2_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_fb2_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-fictionbook+xml"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_fictionbook_basic",
		() => {
			const documentPath = resolveDocument("fictionbook/basic.fb2");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_fictionbook_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_fictionbook_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-fictionbook+xml", "application/x-fictionbook"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_jats_basic",
		() => {
			const documentPath = resolveDocument("jats/sample_article.jats");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_jats_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_jats_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-jats+xml", "text/jats"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_jupyter_basic",
		() => {
			const documentPath = resolveDocument("jupyter/rank.ipynb");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_jupyter_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_jupyter_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-ipynb+json"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_latex_basic",
		() => {
			const documentPath = resolveDocument("latex/basic_sections.tex");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_latex_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_latex_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-latex", "text/x-latex"]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_markdown_basic",
		() => {
			const documentPath = resolveDocument("markdown/comprehensive.md");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_markdown_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_markdown_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["text/markdown"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_ods_basic",
		() => {
			const documentPath = resolveDocument("data_formats/test_01.ods");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_ods_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_ods_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.oasis.opendocument.spreadsheet"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_odt_bold",
		() => {
			const documentPath = resolveDocument("odt/bold.odt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_odt_bold: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_odt_bold", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.oasis.opendocument.text"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_odt_list",
		() => {
			const documentPath = resolveDocument("odt/unorderedList.odt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_odt_list: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_odt_list", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.oasis.opendocument.text"]);
			assertions.assertMinContentLength(result, 30);
			assertions.assertContentContainsAny(result, ["list item", "New level", "Pushed us"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_odt_simple",
		() => {
			const documentPath = resolveDocument("odt/simple.odt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_odt_simple: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_odt_simple", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.oasis.opendocument.text"]);
			assertions.assertMinContentLength(result, 50);
			assertions.assertContentContainsAny(result, ["favorite things", "Parrots", "Analysis"]);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_odt_table",
		() => {
			const documentPath = resolveDocument("odt/table.odt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_odt_table: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_odt_table", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.oasis.opendocument.text"]);
			assertions.assertMinContentLength(result, 10);
			assertions.assertTableCount(result, 1, null);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_opml_basic",
		() => {
			const documentPath = resolveDocument("opml/outline.opml");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_opml_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_opml_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/xml+opml", "text/x-opml", "application/x-opml+xml"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_org_basic",
		() => {
			const documentPath = resolveDocument("org/comprehensive.org");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_org_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_org_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["text/x-org", "text/org"]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_ppsx_slideshow",
		() => {
			const documentPath = resolveDocument("pptx/sample.ppsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_ppsx_slideshow: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_ppsx_slideshow", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.presentationml.slideshow"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_ppt_legacy",
		() => {
			const documentPath = resolveDocument("ppt/simple.ppt");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_ppt_legacy: missing document at", documentPath);
				console.warn("Notes: Requires the office feature.");
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_ppt_legacy", ["office"], "Requires the office feature.")) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.ms-powerpoint"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_basic",
		() => {
			const documentPath = resolveDocument("pptx/simple.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 50);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_images",
		() => {
			const documentPath = resolveDocument("pptx/powerpoint_with_image.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_images: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_images", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_pptx_pitch_deck",
		() => {
			const documentPath = resolveDocument("pptx/pitch_deck_presentation.pptx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_pptx_pitch_deck: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_pptx_pitch_deck", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, [
				"application/vnd.openxmlformats-officedocument.presentationml.presentation",
			]);
			assertions.assertMinContentLength(result, 100);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_rst_basic",
		() => {
			const documentPath = resolveDocument("rst/restructured_text.rst");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_rst_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_rst_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["text/x-rst", "text/prs.fallenstein.rst"]);
			assertions.assertMinContentLength(result, 20);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_rtf_basic",
		() => {
			const documentPath = resolveDocument("rtf/extraction_test.rtf");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_rtf_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_rtf_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/rtf", "text/rtf"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_typst_basic",
		() => {
			const documentPath = resolveDocument("typst/headings.typ");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_typst_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_typst_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/x-typst", "text/x-typst"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xls_legacy",
		() => {
			const documentPath = resolveDocument("xls/test_excel.xls");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xls_legacy: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xls_legacy", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.ms-excel"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_basic",
		() => {
			const documentPath = resolveDocument("xlsx/stanley_cups.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_basic: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_basic", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 100);
			assertions.assertContentContainsAll(result, ["Team", "Location", "Stanley Cups"]);
			assertions.assertTableCount(result, 1, null);
			assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
			assertions.assertMetadataExpectation(result, "sheet_names", { contains: ["Stanley Cups"] });
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_multi_sheet",
		() => {
			const documentPath = resolveDocument("xlsx/excel_multi_sheet.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_multi_sheet: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_multi_sheet", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 20);
			assertions.assertMetadataExpectation(result, "sheet_count", { gte: 2 });
		},
		TEST_TIMEOUT_MS,
	);

	it(
		"office_xlsx_office_example",
		() => {
			const documentPath = resolveDocument("xlsx/test_01.xlsx");
			if (!existsSync(documentPath)) {
				console.warn("Skipping office_xlsx_office_example: missing document at", documentPath);
				return;
			}
			const config = buildConfig(undefined);
			let result: ExtractionResult | null = null;
			try {
				result = extractFileSync(documentPath, null, config);
			} catch (error) {
				if (shouldSkipFixture(error, "office_xlsx_office_example", [], undefined)) {
					return;
				}
				throw error;
			}
			if (result === null) {
				return;
			}
			assertions.assertExpectedMime(result, ["application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"]);
			assertions.assertMinContentLength(result, 10);
		},
		TEST_TIMEOUT_MS,
	);
});
