import { describe, expect, it } from "vitest";

import {
  appendTables,
  buildExtractionConfig,
  buildMetadata,
  excludedKeys,
  generateDocId,
  mapResults,
  pagesRequested,
  prepareInputs,
  resultsToDocuments,
  serializeChunks,
  serializeElements,
  type DocSource,
  type XDocument,
  type XResult,
} from "../src/readerMapping";

function baseDoc(overrides: Partial<XDocument> = {}): XDocument {
  return {
    content: "hello world",
    mimeType: "text/plain",
    counts: { pages: 1 },
    metadata: { title: "Doc", outputFormat: "markdown" },
    ...overrides,
  };
}

describe("buildExtractionConfig", () => {
  it("should default resultFormat to element_based when unset", () => {
    const config = buildExtractionConfig(undefined) as { resultFormat?: string };
    expect(config.resultFormat).toBe("element_based");
  });

  it("should default resultFormat to unified when page extraction is requested", () => {
    const config = buildExtractionConfig({ pages: { extractPages: true } } as never) as { resultFormat?: string };
    expect(config.resultFormat).toBe("unified");
  });

  it("should preserve an explicit resultFormat", () => {
    const config = buildExtractionConfig({ resultFormat: "unified", pages: { extractPages: true } } as never) as {
      resultFormat?: string;
    };
    expect(config.resultFormat).toBe("unified");
  });
});

describe("pagesRequested", () => {
  it("should be true only when extractPages is set", () => {
    expect(pagesRequested({ pages: { extractPages: true } } as never)).toBe(true);
    expect(pagesRequested({ pages: { extractPages: false } } as never)).toBe(false);
    expect(pagesRequested(undefined)).toBe(false);
  });
});

describe("prepareInputs", () => {
  it("should map a single path to a uri input", () => {
    const { inputs, sources } = prepareInputs("/tmp/a.pdf");
    expect(inputs).toEqual([{ kind: "uri", uri: "/tmp/a.pdf" }]);
    expect(sources).toEqual([{ path: "/tmp/a.pdf" }]);
  });

  it("should map an array of paths to parallel uri inputs", () => {
    const { inputs, sources } = prepareInputs(["/a.txt", "/b.txt"]);
    expect(inputs).toHaveLength(2);
    expect(inputs[1]).toEqual({ kind: "uri", uri: "/b.txt" });
    expect(sources[0]).toEqual({ path: "/a.txt" });
  });

  it("should map single bytes with a mime type", () => {
    const data = new Uint8Array([1, 2, 3]);
    const { inputs, sources } = prepareInputs({ data, mimeType: "application/pdf" });
    expect(inputs).toEqual([{ kind: "bytes", bytes: data, mimeType: "application/pdf" }]);
    expect(sources).toEqual([{ data }]);
  });

  it("should require mimeType to be a string for single bytes input", () => {
    expect(() => prepareInputs({ data: new Uint8Array([1]), mimeType: ["a"] })).toThrow(/mimeType must be a string/);
  });

  it("should require parallel bytes/mime lists of equal length", () => {
    expect(() => prepareInputs({ data: [new Uint8Array([1]), new Uint8Array([2])], mimeType: ["a"] })).toThrow(
      /parallel lists of equal length/,
    );
  });
});

describe("mapResults", () => {
  it("should skip and log failed inputs by index and align survivors", () => {
    const docA = baseDoc({ content: "A" });
    const docC = baseDoc({ content: "C" });
    const result: XResult = {
      results: [docA, docC],
      errors: [{ index: 1, errorType: "Decode", message: "bad" }],
    };
    const sources = [{ path: "/a" }, { path: "/b" }, { path: "/c" }];
    const paired = mapResults(result, sources, false);
    expect(paired).toHaveLength(2);
    expect(paired[0]).toEqual([docA, { path: "/a" }]);
    // index 1 failed, so the second surviving source is /c, not /b ~keep
    expect(paired[1]).toEqual([docC, { path: "/c" }]);
  });

  it("should throw on the first error when raiseOnError is set", () => {
    const result: XResult = { results: [], errors: [{ index: 0, errorType: "IO", message: "missing" }] };
    expect(() => mapResults(result, [{ path: "/a" }], true)).toThrow(/input 0: missing/);
  });
});

describe("serializeElements / serializeChunks", () => {
  it("should serialize elements to the forwarding contract", () => {
    const elements = serializeElements([
      { elementType: "Title" as never, text: "Heading", metadata: { pageNumber: 2, elementIndex: 5 } },
    ]);
    expect(elements).toEqual([
      { text: "Heading", element_type: "Title", metadata: { page_number: 2, element_index: 5 } },
    ]);
  });

  it("should serialize chunks to the forwarding contract with defaults", () => {
    const chunks = serializeChunks([
      { content: "body", chunkType: "Text" as never, metadata: { chunkIndex: 0, totalChunks: 3, firstPage: 1 } },
    ]);
    expect(chunks[0]).toEqual({
      content: "body",
      chunk_type: "Text",
      metadata: {
        chunk_index: 0,
        total_chunks: 3,
        first_page: 1,
        last_page: null,
        heading_path: [],
        token_count: null,
      },
    });
  });
});

describe("appendTables", () => {
  it("should append table markdown not already present", () => {
    const out = appendTables("body text", [{ markdown: "| a |" }]);
    expect(out).toBe("body text\n\n| a |");
  });

  it("should dedupe table markdown already inlined in content", () => {
    const out = appendTables("body with | a | table", [{ markdown: "| a |" }]);
    expect(out).toBe("body with | a | table");
  });

  it("should return content unchanged when there are no tables", () => {
    expect(appendTables("body", null)).toBe("body");
  });
});

describe("buildMetadata", () => {
  it("should flatten the 13 metadata fields plus output_format and skip ocr/duration", () => {
    const document = baseDoc({
      metadata: {
        title: "T",
        authors: ["A"],
        createdAt: "2020",
        outputFormat: "markdown",
      },
      qualityScore: 0.9,
      detectedLanguages: ["en"],
    });
    const meta = buildMetadata({ document, filePath: "/docs/report.pdf" });
    expect(meta.file_name).toBe("report.pdf");
    expect(meta.file_path).toBe("/docs/report.pdf");
    expect(meta.file_type).toBe("text/plain");
    expect(meta.total_pages).toBe(1);
    expect(meta.title).toBe("T");
    expect(meta.authors).toEqual(["A"]);
    expect(meta.created_at).toBe("2020");
    expect(meta.output_format).toBe("markdown");
    expect(meta.quality_score).toBe(0.9);
    expect(meta.detected_languages).toEqual(["en"]);
    expect("ocr_used" in meta).toBe(false);
    expect("extraction_duration_ms" in meta).toBe(false);
  });

  it("should forward elements/chunks and set page_number only in page mode", () => {
    const document = baseDoc({
      elements: [{ elementType: "Text" as never, text: "e", metadata: { pageNumber: 1, elementIndex: 0 } }],
    });
    const pageMeta = buildMetadata({ document, filePath: "/a.pdf", pageNumber: 3 });
    expect(pageMeta.page_number).toBe(3);
    expect(pageMeta._xberg_elements).toHaveLength(1);
    const docMeta = buildMetadata({ document, filePath: "/a.pdf" });
    expect("page_number" in docMeta).toBe(false);
  });

  it("should merge caller extraInfo last", () => {
    const meta = buildMetadata({ document: baseDoc(), filePath: "/a.pdf", extraInfo: { source_id: "x" } });
    expect(meta.source_id).toBe("x");
  });
});

describe("generateDocId", () => {
  it("should be deterministic and page-dependent", () => {
    const a = generateDocId({ filePath: "/a.pdf" });
    const b = generateDocId({ filePath: "/a.pdf" });
    const p = generateDocId({ filePath: "/a.pdf", pageNumber: 1 });
    expect(a).toBe(b);
    expect(a).not.toBe(p);
  });

  it("should differ by source", () => {
    expect(generateDocId({ filePath: "/a.pdf" })).not.toBe(generateDocId({ filePath: "/b.pdf" }));
  });
});

describe("excludedKeys", () => {
  it("should list only the present forwarding keys", () => {
    expect(excludedKeys({ _xberg_chunks: [], images: [] })).toEqual(["_xberg_chunks", "images"]);
    expect(excludedKeys({ title: "x" })).toEqual([]);
  });
});

describe("resultsToDocuments", () => {
  it("should emit one whole-source Document carrying elements when element_based", () => {
    const document = baseDoc({
      elements: [{ elementType: "Text" as never, text: "e", metadata: { pageNumber: 1, elementIndex: 0 } }],
      pages: [{ pageNumber: 1, content: "p1" }],
    });
    const docSources: DocSource[] = [[document, { path: "/a.pdf" }]];
    const docs = resultsToDocuments(docSources);
    expect(docs).toHaveLength(1);
    expect(docs[0].metadata._xberg_elements).toHaveLength(1);
    expect(docs[0].excludedLlmMetadataKeys).toContain("_xberg_elements");
    expect(docs[0].excludedEmbedMetadataKeys).toContain("_xberg_elements");
  });

  it("should split per page only when no elements and no chunks (1-indexed)", () => {
    const document = baseDoc({
      elements: null,
      pages: [
        { pageNumber: 1, content: "page one" },
        { pageNumber: 2, content: "page two" },
      ],
    });
    const docs = resultsToDocuments([[document, { path: "/a.pdf" }]]);
    expect(docs).toHaveLength(2);
    expect(docs[0].metadata.page_number).toBe(1);
    expect(docs[1].metadata.page_number).toBe(2);
    expect(docs[0].id_).not.toBe(docs[1].id_);
  });

  it("should not split per page when chunks are present", () => {
    const document = baseDoc({
      elements: null,
      chunks: [{ content: "c", chunkType: "Text" as never, metadata: { chunkIndex: 0, totalChunks: 1 } }],
      pages: [
        { pageNumber: 1, content: "p1" },
        { pageNumber: 2, content: "p2" },
      ],
    });
    const docs = resultsToDocuments([[document, { path: "/a.pdf" }]]);
    expect(docs).toHaveLength(1);
    expect(docs[0].metadata._xberg_chunks).toHaveLength(1);
  });
});
