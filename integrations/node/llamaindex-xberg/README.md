<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# @xberg-io/llamaindex-xberg

[![npm](https://img.shields.io/npm/v/@xberg-io/llamaindex-xberg)](https://www.npmjs.com/package/@xberg-io/llamaindex-xberg)

A [LlamaIndex.TS](https://ts.llamaindex.ai) reader and node parser for [Xberg](https://github.com/xberg-io/xberg).
`XbergReader` turns a file, a directory, or raw bytes into LlamaIndex `Document`s with the extracted
text, tables, and rich metadata from 90+ formats — with optional OCR for scans and images.
`XbergNodeParser` then splits those documents into `TextNode`s along Xberg's own semantic boundaries
(native chunks, or structural elements) instead of a blind character window.

Extraction runs locally in-process through the `@xberg-io/xberg` native binding. No API key, no cloud
call, no data leaves your machine.

## Installation

`@llamaindex/core` is a peer dependency — install it alongside the reader:

```bash
npm install @xberg-io/llamaindex-xberg @llamaindex/core
```

Node.js 20.15+ is required on a platform for which `@xberg-io/xberg` ships a prebuilt binary (Linux
x64/arm64 glibc or musl, macOS arm64, Windows x64/arm64).

## Quick start

```ts
import { XbergReader } from "@xberg-io/llamaindex-xberg";

// Single file — one Document (or one per page with pages.extractPages)
const reader = new XbergReader();
const docs = await reader.loadData("report.pdf");

// Multiple files — one batched extraction
const many = await reader.loadData(["a.pdf", "b.docx"]);

// Raw bytes — mimeType is required
const fromBytes = await reader.loadData({ data: fileBytes, mimeType: "application/pdf" });
```

By default the reader requests Xberg's `element_based` result format so each document carries a
structural element stream for the node parser. Errors are logged and the failed input is skipped; pass
`{ raiseOnError: true }` to propagate them instead.

## Structure-aware node parsing

Pair the reader with `XbergNodeParser` to split documents into nodes along Xberg's boundaries. It
prefers native chunks (`ExtractionConfig.chunking`) and falls back to structural elements:

```ts
import { XbergReader, XbergNodeParser } from "@xberg-io/llamaindex-xberg";

const reader = new XbergReader({ extractionConfig: { chunking: { max_chars: 1000, max_overlap: 200 } } });
const documents = await reader.loadData("report.pdf");

const parser = new XbergNodeParser();
const nodes = parser.getNodesFromDocuments(documents);
```

The reader forwards the chunk/element payload on private metadata keys that are excluded from LLM and
embedding input; the parser consumes them and strips them from the emitted nodes.

## Supported formats

Xberg extracts from 90+ formats including PDF, DOCX, PPTX, XLSX, HTML, EPUB, images, and more. See the
[Xberg documentation](https://docs.xberg.io) for the full list and the extraction configuration
reference.

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — document intelligence: text, tables, metadata from 91+ formats with optional OCR.
- [Xberg Enterprise](https://github.com/xberg-io/xberg-enterprise) — managed extraction API with SDKs, dashboards, and observability.

## License

[MIT](./LICENSE)
