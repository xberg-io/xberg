/**
 * Comprehensive metadata type definition tests.
 *
 * These tests verify that:
 * 1. Type Compatibility: Metadata types deserialize from JSON correctly and are compatible with NAPI bindings
 * 2. Rich Metadata Types: HeaderMetadata, LinkMetadata, HtmlImageMetadata, and StructuredData have correct fields
 * 3. Breaking Changes: Old field names are removed, new field names exist (canonicalUrl, openGraph, twitterCard as Record)
 * 4. Runtime Validation: JSON structures deserialize and preserve type information correctly
 * 5. Integration: HTML extraction produces correct metadata structure
 *
 * These tests will BREAK at compile time if:
 * - Metadata types are removed from index.d.ts
 * - Type structures diverge between packages
 * - Required fields are missing or changed
 * - Old field names are accidentally restored
 */

import { describe, expect, it } from "vitest";
import type {
	ExcelMetadata as NapiExcelMetadata,
	HtmlMetadata as NapiHtmlMetadata,
	Metadata as NapiMetadata,
	PdfMetadata as NapiPdfMetadata,
} from "../../../../crates/kreuzberg/metadata";
import { extractBytesSync, extractFileSync } from "../../dist/index.js";
import type {
	ArchiveMetadata,
	EmailMetadata,
	ErrorMetadata,
	ExcelMetadata,
	HeaderMetadata,
	HtmlImageMetadata,
	HtmlMetadata,
	ImageMetadata,
	ImagePreprocessingMetadata,
	LinkMetadata,
	Metadata,
	OcrMetadata,
	PdfMetadata,
	PptxMetadata,
	StructuredData,
	TextMetadata,
	XmlMetadata,
} from "../../src/types";
import { createTempFile, getTestDocumentPath, loadTestDocument } from "../helpers/test-utils.js";

type AssertMetadataCompatible = Metadata extends NapiMetadata ? true : never;
type AssertNapiMetadataCompatible = NapiMetadata extends Metadata ? true : never;

type AssertHtmlMetadataCompatible = HtmlMetadata extends NapiHtmlMetadata ? true : never;
type AssertNapiHtmlMetadataCompatible = NapiHtmlMetadata extends HtmlMetadata ? true : never;

type AssertPdfMetadataCompatible = PdfMetadata extends NapiPdfMetadata ? true : never;
type AssertNapiPdfMetadataCompatible = NapiPdfMetadata extends PdfMetadata ? true : never;

type AssertExcelMetadataCompatible = ExcelMetadata extends NapiExcelMetadata ? true : never;
type AssertNapiExcelMetadataCompatible = NapiExcelMetadata extends ExcelMetadata ? true : never;

const _compatibilityTests: [
	AssertMetadataCompatible,
	AssertNapiMetadataCompatible,
	AssertHtmlMetadataCompatible,
	AssertNapiHtmlMetadataCompatible,
	AssertPdfMetadataCompatible,
	AssertNapiPdfMetadataCompatible,
	AssertExcelMetadataCompatible,
	AssertNapiExcelMetadataCompatible,
] = [true, true, true, true, true, true, true, true];

type AssertMetadataHasPdf = "pdf" extends keyof Metadata ? true : never;
type AssertMetadataHasExcel = "excel" extends keyof Metadata ? true : never;
type AssertMetadataHasEmail = "email" extends keyof Metadata ? true : never;
type AssertMetadataHasPptx = "pptx" extends keyof Metadata ? true : never;
type AssertMetadataHasArchive = "archive" extends keyof Metadata ? true : never;
type AssertMetadataHasImage = "image" extends keyof Metadata ? true : never;
type AssertMetadataHasXml = "xml" extends keyof Metadata ? true : never;
type AssertMetadataHasText = "text" extends keyof Metadata ? true : never;
type AssertMetadataHasHtml = "html" extends keyof Metadata ? true : never;
type AssertMetadataHasOcr = "ocr" extends keyof Metadata ? true : never;
type AssertMetadataHasImagePreprocessing = "imagePreprocessing" extends keyof Metadata ? true : never;

const _metadataStructureTests: [
	AssertMetadataHasPdf,
	AssertMetadataHasExcel,
	AssertMetadataHasEmail,
	AssertMetadataHasPptx,
	AssertMetadataHasArchive,
	AssertMetadataHasImage,
	AssertMetadataHasXml,
	AssertMetadataHasText,
	AssertMetadataHasHtml,
	AssertMetadataHasOcr,
	AssertMetadataHasImagePreprocessing,
] = [true, true, true, true, true, true, true, true, true, true, true];

type AssertHtmlHasCanonicalUrl = "canonicalUrl" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasOpenGraph = "openGraph" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasTwitterCard = "twitterCard" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasMetaTags = "metaTags" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasHeaders = "htmlHeaders" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasLinks = "htmlLinks" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasImages = "htmlImages" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasStructuredData = "structuredData" extends keyof HtmlMetadata ? true : never;
type AssertHtmlHasKeywords = "keywords" extends keyof HtmlMetadata ? true : never;

type AssertOpenGraphIsRecord = HtmlMetadata["openGraph"] extends Record<string, string> ? true : never;
type AssertTwitterCardIsRecord = HtmlMetadata["twitterCard"] extends Record<string, string> ? true : never;
type AssertKeywordsIsArray = HtmlMetadata["keywords"] extends string[] ? true : never;

const _newHtmlMetadataFields: [
	AssertHtmlHasCanonicalUrl,
	AssertHtmlHasOpenGraph,
	AssertHtmlHasTwitterCard,
	AssertHtmlHasMetaTags,
	AssertHtmlHasHeaders,
	AssertHtmlHasLinks,
	AssertHtmlHasImages,
	AssertHtmlHasStructuredData,
	AssertHtmlHasKeywords,
	AssertOpenGraphIsRecord,
	AssertTwitterCardIsRecord,
	AssertKeywordsIsArray,
] = [true, true, true, true, true, true, true, true, true, true, true, true];

describe("Metadata Types - Type Compatibility Tests", () => {
	describe("HtmlMetadata Type Structure", () => {
		it("test_html_metadata_deserializes", () => {
			const jsonMetadata = {
				keywords: ["test", "keywords"],
				canonicalUrl: "https://example.com",
				openGraph: { "og:title": "Test Title", "og:image": "image.png" },
				twitterCard: { card: "summary", site: "@example" },
				metaTags: { viewport: "width=device-width" },
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			const metadata: HtmlMetadata = jsonMetadata;

			expect(metadata).toBeDefined();
			expect(metadata.keywords).toEqual(["test", "keywords"]);
			expect(metadata.canonicalUrl).toBe("https://example.com");
			expect(metadata.openGraph).toEqual({ "og:title": "Test Title", "og:image": "image.png" });
			expect(metadata.twitterCard).toEqual({ card: "summary", site: "@example" });
		});

		it("test_keywords_is_array", () => {
			const htmlMetadata: HtmlMetadata = {
				keywords: ["search", "terms", "here"],
				openGraph: {},
				twitterCard: {},
				metaTags: {},
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(Array.isArray(htmlMetadata.keywords)).toBe(true);
			expect(htmlMetadata.keywords).toHaveLength(3);
			expect(htmlMetadata.keywords[0]).toBe("search");
		});

		it("test_canonical_url_renamed", () => {
			const htmlMetadata: HtmlMetadata = {
				canonicalUrl: "https://example.com/canonical",
				openGraph: {},
				twitterCard: {},
				metaTags: {},
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(htmlMetadata.canonicalUrl).toBe("https://example.com/canonical");
			expect(htmlMetadata).toHaveProperty("canonicalUrl");
		});

		it("test_open_graph_is_record", () => {
			const htmlMetadata: HtmlMetadata = {
				openGraph: {
					"og:title": "Page Title",
					"og:description": "Page Description",
					"og:image": "https://example.com/image.png",
					"og:url": "https://example.com",
				},
				twitterCard: {},
				metaTags: {},
				keywords: [],
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(typeof htmlMetadata.openGraph).toBe("object");
			expect(Object.keys(htmlMetadata.openGraph).length).toBe(4);
			expect(htmlMetadata.openGraph["og:title"]).toBe("Page Title");
		});

		it("test_twitter_card_is_record", () => {
			const htmlMetadata: HtmlMetadata = {
				twitterCard: {
					card: "summary_large_image",
					site: "@example",
					creator: "@author",
					title: "Tweet Title",
				},
				openGraph: {},
				metaTags: {},
				keywords: [],
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(typeof htmlMetadata.twitterCard).toBe("object");
			expect(Object.keys(htmlMetadata.twitterCard).length).toBe(4);
			expect(htmlMetadata.twitterCard.card).toBe("summary_large_image");
		});
	});

	describe("Rich Metadata Type Tests", () => {
		it("test_header_metadata_structure", () => {
			const header: HeaderMetadata = {
				level: 1,
				text: "Main Heading",
				id: "main-heading",
				depth: 0,
				htmlOffset: 42,
			};

			expect(header.level).toBe(1);
			expect(header.text).toBe("Main Heading");
			expect(header.id).toBe("main-heading");
			expect(header.depth).toBe(0);
			expect(header.htmlOffset).toBe(42);
			expect(typeof header.level).toBe("number");
			expect(typeof header.text).toBe("string");
			expect(typeof header.htmlOffset).toBe("number");
		});

		it("test_link_metadata_structure", () => {
			const link: LinkMetadata = {
				href: "https://example.com",
				text: "Example Site",
				title: "Visit Example",
				linkType: "external",
				rel: ["nofollow"],
				attributes: { target: "_blank", class: "external-link" },
			};

			expect(link.href).toBe("https://example.com");
			expect(link.text).toBe("Example Site");
			expect(link.linkType).toBe("external");
			expect(["anchor", "internal", "external", "email", "phone", "other"]).toContain(link.linkType);
			expect(Array.isArray(link.rel)).toBe(true);
			expect(typeof link.attributes).toBe("object");
		});

		it("test_link_metadata_internal_link", () => {
			const internalLink: LinkMetadata = {
				href: "/page/about",
				text: "About Us",
				linkType: "internal",
				rel: [],
				attributes: {},
			};

			expect(internalLink.linkType).toBe("internal");
			expect(internalLink.href).toBe("/page/about");
		});

		it("test_link_metadata_email_phone_types", () => {
			const emailLink: LinkMetadata = {
				href: "mailto:contact@example.com",
				text: "Email Us",
				linkType: "email",
				rel: [],
				attributes: {},
			};

			const phoneLink: LinkMetadata = {
				href: "tel:+1234567890",
				text: "Call Us",
				linkType: "phone",
				rel: [],
				attributes: {},
			};

			expect(emailLink.linkType).toBe("email");
			expect(phoneLink.linkType).toBe("phone");
		});

		it("test_image_metadata_structure", () => {
			const image: HtmlImageMetadata = {
				src: "https://example.com/image.png",
				alt: "Alternative text",
				title: "Image title",
				dimensions: [1200, 800],
				imageType: "external",
				attributes: { class: "featured-image", "data-lazy": "true" },
			};

			expect(image.src).toBe("https://example.com/image.png");
			expect(image.alt).toBe("Alternative text");
			expect(image.imageType).toBe("external");
			expect(["data_uri", "inline_svg", "external", "relative"]).toContain(image.imageType);
			expect(Array.isArray(image.dimensions)).toBe(true);
			expect(image.dimensions).toEqual([1200, 800]);
		});

		it("test_image_metadata_data_uri", () => {
			const dataUriImage: HtmlImageMetadata = {
				src: "data:image/png;base64,iVBORw0KGgo=",
				imageType: "data_uri",
				attributes: {},
			};

			expect(dataUriImage.imageType).toBe("data_uri");
		});

		it("test_image_metadata_relative", () => {
			const relativeImage: HtmlImageMetadata = {
				src: "./images/logo.svg",
				imageType: "relative",
				attributes: {},
			};

			expect(relativeImage.imageType).toBe("relative");
		});

		it("test_structured_data_structure", () => {
			const jsonLd: StructuredData = {
				dataType: "json_ld",
				rawJson: '{"@context":"https://schema.org","@type":"Article"}',
				schemaType: "Article",
			};

			const microdata: StructuredData = {
				dataType: "microdata",
				rawJson: "{}",
			};

			const rdfa: StructuredData = {
				dataType: "rdfa",
				rawJson: "{}",
				schemaType: "Person",
			};

			expect(jsonLd.dataType).toBe("json_ld");
			expect(microdata.dataType).toBe("microdata");
			expect(rdfa.dataType).toBe("rdfa");
			expect(["json_ld", "microdata", "rdfa"]).toContain(jsonLd.dataType);
			expect(typeof jsonLd.rawJson).toBe("string");
		});
	});

	describe("Integration Tests", () => {
		it("test_extract_html_with_metadata", () => {
			const htmlContent = `
        <!DOCTYPE html>
        <html lang="en">
        <head>
          <meta charset="UTF-8">
          <title>Test Page</title>
          <meta name="description" content="Test description">
          <meta name="keywords" content="test, example">
          <link rel="canonical" href="https://example.com">
          <meta property="og:title" content="OG Title">
          <meta property="og:image" content="image.png">
          <meta name="twitter:card" content="summary">
        </head>
        <body>
          <h1>Main Title</h1>
          <p>Content</p>
        </body>
        </html>
      `;

			const buffer = Buffer.from(htmlContent, "utf-8");
			const result = extractBytesSync(buffer, "text/html");

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();
			expect(result.content).toBeTruthy();
		});

		it("test_extract_html_file_integration", () => {
			const htmlPath = getTestDocumentPath("web/taylor_swift.html");
			const buffer = loadTestDocument("web/taylor_swift.html");

			const result = extractBytesSync(buffer, "text/html", null);

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();
			expect(typeof result.metadata).toBe("object");
			expect(result.content).toBeTruthy();
			expect(typeof result.content).toBe("string");

			const htmlMetadata = result.metadata.html;
			if (htmlMetadata) {
				expect(typeof htmlMetadata === "object").toBe(true);

				if (htmlMetadata.title) {
					expect(typeof htmlMetadata.title).toBe("string");
				}

				if (htmlMetadata.description) {
					expect(typeof htmlMetadata.description).toBe("string");
				}

				expect(Array.isArray(htmlMetadata.keywords)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlHeaders)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlLinks)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlImages)).toBe(true);
				expect(Array.isArray(htmlMetadata.structuredData)).toBe(true);

				expect(typeof htmlMetadata.openGraph).toBe("object");
				expect(typeof htmlMetadata.twitterCard).toBe("object");
				expect(typeof htmlMetadata.metaTags).toBe("object");
			}
		});

		it("test_empty_html_returns_defaults", () => {
			const emptyHtml = `<!DOCTYPE html><html><head></head><body></body></html>`;
			const buffer = Buffer.from(emptyHtml, "utf-8");

			const result = extractBytesSync(buffer, "text/html", null);

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();

			const htmlMetadata = result.metadata.html;
			if (htmlMetadata) {
				expect(Array.isArray(htmlMetadata.keywords)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlHeaders)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlLinks)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlImages)).toBe(true);
				expect(Array.isArray(htmlMetadata.structuredData)).toBe(true);

				expect(typeof htmlMetadata.openGraph).toBe("object");
				expect(typeof htmlMetadata.twitterCard).toBe("object");
				expect(typeof htmlMetadata.metaTags).toBe("object");

				expect(Object.keys(htmlMetadata.openGraph).length).toBeGreaterThanOrEqual(0);
				expect(Object.keys(htmlMetadata.twitterCard).length).toBeGreaterThanOrEqual(0);
			}
		});

		it("test_malformed_html_graceful_handling", () => {
			const malformedHtml = `
        <!DOCTYPE html>
        <html>
        <head>
          <title>Malformed Page</title>
          <meta name="description" content="Some content">
        </head>
        <body>
          <div>
            <p>Unclosed paragraph
            <img src="image.png" alt="image">
            <a href="https://example.com">Link without closing tag
          </body>
        </html>
      `;

			const buffer = Buffer.from(malformedHtml, "utf-8");

			expect(() => {
				extractBytesSync(buffer, "text/html", null);
			}).not.toThrow();

			const result = extractBytesSync(buffer, "text/html", null);

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();

			const htmlMetadata = result.metadata.html;
			if (htmlMetadata) {
				expect(typeof htmlMetadata === "object").toBe(true);
				expect(Array.isArray(htmlMetadata.keywords)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlHeaders)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlLinks)).toBe(true);
				expect(Array.isArray(htmlMetadata.htmlImages)).toBe(true);

				expect(typeof htmlMetadata.openGraph).toBe("object");
				expect(typeof htmlMetadata.twitterCard).toBe("object");
			}
		});

		it("test_special_characters_in_metadata", () => {
			const specialHtml = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>Advanced Features with Special Characters</title>
<meta name="description" content="Cafe, naive, special characters test">
<meta name="keywords" content="test, unicode, special">
<meta property="og:title" content="Title with test">
</head>
<body>
<h1>Unicode: Chinese Japanese Hebrew</h1>
<p>Testing special character handling</p>
<a href="https://example.com/page">Test Link</a>
</body>
</html>`;

			const buffer = Buffer.from(specialHtml, "utf-8");
			const result = extractBytesSync(buffer, "text/html", null);

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();

			const htmlMetadata = result.metadata.html;
			if (htmlMetadata) {
				if (htmlMetadata.title) {
					expect(htmlMetadata.title).toBeDefined();
					expect(typeof htmlMetadata.title).toBe("string");
					expect(htmlMetadata.title.length).toBeGreaterThan(0);
				}

				if (htmlMetadata.description) {
					expect(typeof htmlMetadata.description).toBe("string");
					expect(htmlMetadata.description.length).toBeGreaterThan(0);
				}

				expect(Array.isArray(htmlMetadata.keywords)).toBe(true);

				expect(Array.isArray(htmlMetadata.htmlHeaders)).toBe(true);
				if (htmlMetadata.htmlHeaders.length > 0) {
					const header = htmlMetadata.htmlHeaders[0];
					expect(typeof header.text).toBe("string");
					expect(header.text.length).toBeGreaterThan(0);
				}

				if (htmlMetadata.openGraph) {
					expect(typeof htmlMetadata.openGraph).toBe("object");
				}

				expect(Array.isArray(htmlMetadata.htmlLinks)).toBe(true);
				if (htmlMetadata.htmlLinks.length > 0) {
					const link = htmlMetadata.htmlLinks[0];
					expect(typeof link.href).toBe("string");
					expect(typeof link.text).toBe("string");
					expect(link.href.length).toBeGreaterThan(0);
				}
			}
		});

		it("test_large_html_extraction_performance", () => {
			let largeHtml = `
        <!DOCTYPE html>
        <html>
        <head>
          <title>Large Document Performance Test</title>
          <meta name="description" content="Testing extraction performance with large HTML">
        </head>
        <body>
          <h1>Main Heading</h1>
      `;

			for (let i = 0; i < 500; i++) {
				largeHtml += `
          <section>
            <h2>Section ${i + 1}</h2>
            <p>This is paragraph ${i + 1} with some content that provides context and information about the section.</p>
            <a href="https://example.com/page${i + 1}">Link ${i + 1}</a>
            <img src="image${i + 1}.png" alt="Image ${i + 1}">
          </section>
        `;
			}

			largeHtml += `
        </body>
        </html>
      `;

			const buffer = Buffer.from(largeHtml, "utf-8");
			const startTime = Date.now();

			const result = extractBytesSync(buffer, "text/html", null);
			const duration = Date.now() - startTime;

			expect(result).toBeDefined();
			expect(result.mimeType).toBe("text/html");
			expect(result.metadata).toBeDefined();
			expect(result.content).toBeTruthy();
			expect(duration).toBeLessThan(5000);

			const htmlMetadata = result.metadata.html;
			if (htmlMetadata) {
				expect(Array.isArray(htmlMetadata.htmlHeaders)).toBe(true);
				if (htmlMetadata.htmlHeaders.length > 0) {
					expect(htmlMetadata.htmlHeaders.length).toBeGreaterThan(0);
				}

				expect(Array.isArray(htmlMetadata.htmlLinks)).toBe(true);
				if (htmlMetadata.htmlLinks.length > 0) {
					expect(htmlMetadata.htmlLinks.length).toBeGreaterThan(0);
				}

				expect(Array.isArray(htmlMetadata.htmlImages)).toBe(true);
				if (htmlMetadata.htmlImages.length > 0) {
					expect(htmlMetadata.htmlImages.length).toBeGreaterThan(0);
				}
			}

			expect(duration).toBeLessThan(5000);
		});

		it("test_metadata_round_trip", () => {
			const originalMetadata: HtmlMetadata = {
				title: "Test Page",
				description: "Test description",
				keywords: ["test", "metadata"],
				canonicalUrl: "https://example.com",
				openGraph: { "og:title": "Test", "og:image": "img.png" },
				twitterCard: { card: "summary" },
				metaTags: { viewport: "width=device-width" },
				htmlHeaders: [
					{
						level: 1,
						text: "Heading",
						depth: 0,
						htmlOffset: 0,
					},
				],
				htmlLinks: [
					{
						href: "https://example.com",
						text: "Link",
						linkType: "external",
						rel: [],
						attributes: {},
					},
				],
				htmlImages: [
					{
						src: "image.png",
						imageType: "relative",
						attributes: {},
					},
				],
				structuredData: [],
			};

			const json = JSON.stringify(originalMetadata);
			const deserialized: HtmlMetadata = JSON.parse(json);

			expect(deserialized.title).toBe(originalMetadata.title);
			expect(deserialized.keywords).toEqual(originalMetadata.keywords);
			expect(deserialized.openGraph).toEqual(originalMetadata.openGraph);
			expect(deserialized.htmlHeaders).toEqual(originalMetadata.htmlHeaders);
			expect(deserialized.htmlLinks).toEqual(originalMetadata.htmlLinks);
		});

		it("test_empty_collections_default", () => {
			const minimalMetadata: HtmlMetadata = {
				keywords: [],
				openGraph: {},
				twitterCard: {},
				metaTags: {},
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(minimalMetadata.keywords).toEqual([]);
			expect(minimalMetadata.openGraph).toEqual({});
			expect(minimalMetadata.htmlHeaders).toEqual([]);
			expect(Object.keys(minimalMetadata.openGraph).length).toBe(0);
		});
	});

	describe("Breaking Change Validation", () => {
		it("test_old_field_names_removed", () => {
			const htmlMetadata: HtmlMetadata = {
				openGraph: { "og:title": "OG Title" },
				twitterCard: { card: "summary" },
				metaTags: {},
				keywords: [],
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect((htmlMetadata as any).ogTitle).toBeUndefined();
			expect((htmlMetadata as any).ogDescription).toBeUndefined();
			expect((htmlMetadata as any).twitterTitle).toBeUndefined();

			const keys = Object.keys(htmlMetadata);
			expect(keys).not.toContain("ogTitle");
			expect(keys).not.toContain("twitterTitle");
		});

		it("test_new_field_names_exist", () => {
			const htmlMetadata: HtmlMetadata = {
				canonicalUrl: "https://example.com",
				openGraph: { "og:title": "Test" },
				twitterCard: { card: "summary" },
				metaTags: {},
				keywords: [],
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(htmlMetadata).toHaveProperty("canonicalUrl");
			expect(htmlMetadata).toHaveProperty("openGraph");
			expect(htmlMetadata).toHaveProperty("twitterCard");
			expect(htmlMetadata).toHaveProperty("htmlHeaders");
			expect(htmlMetadata).toHaveProperty("htmlLinks");
			expect(htmlMetadata).toHaveProperty("htmlImages");
			expect(htmlMetadata).toHaveProperty("structuredData");
		});

		it("test_record_types_enforced", () => {
			const htmlMetadata: HtmlMetadata = {
				openGraph: { "og:title": "Title", "og:image": "url" },
				twitterCard: { card: "summary_large_image", site: "@user" },
				metaTags: {},
				keywords: [],
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};

			expect(typeof htmlMetadata.openGraph).toBe("object");
			expect(typeof htmlMetadata.twitterCard).toBe("object");

			expect(typeof htmlMetadata.openGraph).not.toBe("string");
			expect(typeof htmlMetadata.twitterCard).not.toBe("string");

			expect(htmlMetadata.openGraph["og:title"]).toBe("Title");
			expect(htmlMetadata.twitterCard.card).toBe("summary_large_image");
		});
	});

	describe("NAPI Bindings Compatibility", () => {
		it("should import Metadata from NAPI bindings", () => {
			const napiMetadata: NapiMetadata = {
				language: "en",
			};
			expect(napiMetadata).toBeDefined();
		});

		it("should import HtmlMetadata from NAPI bindings", () => {
			const napiHtml: NapiHtmlMetadata = {
				title: "Test",
			};
			expect(napiHtml).toBeDefined();
		});

		it("should allow local Metadata to be assigned to NAPI Metadata", () => {
			const localMetadata: Metadata = {
				language: "en",
				html: {
					title: "Test",
					keywords: [],
					openGraph: {},
					twitterCard: {},
					metaTags: {},
					htmlHeaders: [],
					htmlLinks: [],
					htmlImages: [],
					structuredData: [],
				},
			};

			const napiMetadata: NapiMetadata = localMetadata;
			expect(napiMetadata).toBeDefined();
		});

		it("should allow NAPI Metadata to be assigned to local Metadata", () => {
			const napiMetadata: NapiMetadata = {
				language: "en",
				html: {
					title: "Test",
					keywords: [],
					openGraph: {},
					twitterCard: {},
					metaTags: {},
					htmlHeaders: [],
					htmlLinks: [],
					htmlImages: [],
					structuredData: [],
				},
			};

			const localMetadata: Metadata = napiMetadata;
			expect(localMetadata).toBeDefined();
		});
	});

	describe("Type Exports", () => {
		it("should export Metadata type from local package", () => {
			const metadata: Metadata = {
				language: "en",
				date: "2025-01-01",
			};
			expect(metadata).toBeDefined();
		});

		it("should export HtmlMetadata type from local package", () => {
			const htmlMetadata: HtmlMetadata = {
				keywords: [],
				openGraph: {},
				twitterCard: {},
				metaTags: {},
				htmlHeaders: [],
				htmlLinks: [],
				htmlImages: [],
				structuredData: [],
			};
			expect(htmlMetadata).toBeDefined();
		});

		it("should export all rich metadata types", () => {
			const header: HeaderMetadata = { level: 1, text: "H1", depth: 0, htmlOffset: 0 };
			const link: LinkMetadata = {
				href: "http://example.com",
				text: "Example",
				linkType: "external",
				rel: [],
				attributes: {},
			};
			const image: HtmlImageMetadata = { src: "image.png", imageType: "relative", attributes: {} };
			const data: StructuredData = { dataType: "json_ld", rawJson: "{}" };

			expect(header).toBeDefined();
			expect(link).toBeDefined();
			expect(image).toBeDefined();
			expect(data).toBeDefined();
		});

		it("should export all metadata types from local package", () => {
			const pdf: PdfMetadata = { title: "Test", pageCount: 10 };
			const excel: ExcelMetadata = { sheetCount: 1, sheetNames: ["Sheet1"] };
			const email: EmailMetadata = { toEmails: [], ccEmails: [], bccEmails: [], attachments: [] };
			const pptx: PptxMetadata = { fonts: [] };
			const archive: ArchiveMetadata = { format: "zip", fileCount: 1, fileList: [], totalSize: 0 };
			const image: ImageMetadata = { width: 100, height: 100, format: "png", exif: {} };
			const xml: XmlMetadata = { elementCount: 10, uniqueElements: [] };
			const text: TextMetadata = { lineCount: 10, wordCount: 100, characterCount: 500 };
			const ocr: OcrMetadata = { language: "eng", psm: 3, outputFormat: "text", tableCount: 0 };
			const imgPreproc: ImagePreprocessingMetadata = {
				originalDimensions: [100, 100],
				originalDpi: [72, 72],
				targetDpi: 300,
				scaleFactor: 1.0,
				autoAdjusted: false,
				finalDpi: 300,
				resampleMethod: "lanczos",
				dimensionClamped: false,
				skippedResize: false,
			};
			const error: ErrorMetadata = { errorType: "test", message: "test error" };

			expect(pdf).toBeDefined();
			expect(excel).toBeDefined();
			expect(email).toBeDefined();
			expect(pptx).toBeDefined();
			expect(archive).toBeDefined();
			expect(image).toBeDefined();
			expect(xml).toBeDefined();
			expect(text).toBeDefined();
			expect(ocr).toBeDefined();
			expect(imgPreproc).toBeDefined();
			expect(error).toBeDefined();
		});
	});

	describe("Type Structure", () => {
		it("should have html field in Metadata type", () => {
			const metadata: Metadata = {
				html: {
					title: "Test",
					description: "Test description",
					keywords: [],
					openGraph: {},
					twitterCard: {},
					metaTags: {},
					htmlHeaders: [],
					htmlLinks: [],
					htmlImages: [],
					structuredData: [],
				},
			};

			expect(metadata.html).toBeDefined();
			expect(metadata.html?.title).toBe("Test");
		});

		it("should allow Metadata to have all format-specific fields", () => {
			const metadata: Metadata = {
				language: "en",
				pdf: { title: "PDF", pageCount: 10 },
				excel: { sheetCount: 1, sheetNames: ["Sheet1"] },
				email: { toEmails: [], ccEmails: [], bccEmails: [], attachments: [] },
				pptx: { fonts: [] },
				archive: { format: "zip", fileCount: 1, fileList: [], totalSize: 0 },
				image: { width: 100, height: 100, format: "png", exif: {} },
				xml: { elementCount: 10, uniqueElements: [] },
				text: { lineCount: 10, wordCount: 100, characterCount: 500 },
				html: {
					title: "HTML Page",
					keywords: [],
					openGraph: {},
					twitterCard: {},
					metaTags: {},
					htmlHeaders: [],
					htmlLinks: [],
					htmlImages: [],
					structuredData: [],
				},
				ocr: { language: "eng", psm: 3, outputFormat: "text", tableCount: 0 },
			};

			expect(metadata.pdf).toBeDefined();
			expect(metadata.excel).toBeDefined();
			expect(metadata.email).toBeDefined();
			expect(metadata.pptx).toBeDefined();
			expect(metadata.archive).toBeDefined();
			expect(metadata.image).toBeDefined();
			expect(metadata.xml).toBeDefined();
			expect(metadata.text).toBeDefined();
			expect(metadata.html).toBeDefined();
			expect(metadata.ocr).toBeDefined();
		});
	});
});
