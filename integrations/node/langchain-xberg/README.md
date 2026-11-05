<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# @xberg-io/langchain-xberg

[![npm](https://img.shields.io/npm/v/@xberg-io/langchain-xberg)](https://www.npmjs.com/package/@xberg-io/langchain-xberg)

A [LangChain.js](https://js.langchain.com) document loader for [Xberg](https://github.com/xberg-io/xberg).
Point it at a file, a directory, or raw bytes and it returns LangChain `Document`s with the extracted
text, tables, and rich metadata from 107 document formats — with optional OCR for scans and images.

Extraction runs locally in-process through the `@xberg-io/xberg` native binding. No API key, no cloud
call, no data leaves your machine.

## Installation

`@langchain/core` is a peer dependency — install it alongside the loader:

```bash
npm install @xberg-io/langchain-xberg @langchain/core
```

Node.js 22+ is required on a platform for which `@xberg-io/xberg` ships a prebuilt binary (Linux
x64/arm64 glibc or musl, macOS arm64, Windows x64/arm64).

## Quick start

```ts
import { XbergLoader } from "@xberg-io/langchain-xberg";

// Single file — one Document
const loader = new XbergLoader({ filePath: "report.pdf" });
const docs = await loader.load();
console.log(docs[0].pageContent, docs[0].metadata);

// Multiple files or a directory — one batched extraction
const many = new XbergLoader({ filePath: "./docs", glob: "**/*.pdf" });

// Raw bytes — mimeType is required
const bytes = new XbergLoader({ data: fileBytes, mimeType: "application/pdf" });
```

### One Document per chunk (retrieval)

Enable `chunking` on the `ExtractionConfig` to emit one `Document` per chunk, each carrying heading
path, page span, and token-count metadata ready to embed:

```ts
const loader = new XbergLoader({
  filePath: "report.pdf",
  config: { chunking: { max_chars: 1000, max_overlap: 200 } },
});
const chunks = await loader.load();
```

### One Document per page

```ts
const loader = new XbergLoader({
  filePath: "report.pdf",
  config: { pages: { extractPages: true } },
});
const pages = await loader.load(); // metadata.page is 0-indexed
```

## Supported formats

Xberg extracts from 107 formats including PDF, DOCX, PPTX, XLSX, HTML, EPUB, images, and more. See the
[Xberg documentation](https://docs.xberg.io) for the full list and the extraction configuration
reference.

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — document intelligence: text, tables, metadata from 107 formats with optional OCR.
- [Xberg Enterprise](https://github.com/xberg-io/xberg-enterprise) — managed extraction API with SDKs, dashboards, and observability.

## License

[MIT](./LICENSE)
