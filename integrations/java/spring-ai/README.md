<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# spring-ai-xberg

A [Spring AI](https://docs.spring.io/spring-ai/reference/) `DocumentReader` that runs
[Xberg](https://github.com/xberg-io/xberg) document extraction. Point it at a `Resource` and it
returns Spring AI `Document` instances carrying the extracted text, tables, and metadata — with
optional OCR for scans and images.

Extraction runs locally in-process through the `io.xberg:xberg` native binding. No API key, no cloud
call, no data leaves your JVM.

## Requirements

- Java 21+.
- Spring AI 2.0.0 (`spring-ai-commons`).
- A platform for which `io.xberg:xberg` ships a prebuilt native binary (Linux x64/arm64, macOS
  arm64, Windows x64).

## Installation

```xml
<dependency>
    <groupId>io.xberg</groupId>
    <artifactId>spring-ai-xberg</artifactId>
    <version>1.0.0-rc.32</version>
</dependency>
```

## Usage

Read a single file:

```java
import io.xberg.springai.XbergDocumentReader;
import org.springframework.ai.document.Document;
import org.springframework.core.io.FileSystemResource;

import java.util.List;

var reader = XbergDocumentReader.builder()
        .resource(new FileSystemResource("report.pdf"))
        .build();

List<Document> documents = reader.get();
```

Read several files in one call. Multiple resources go through `Xberg.extractBatch`, which is
substantially faster than a reader per file:

```java
var reader = XbergDocumentReader.builder()
        .resources(List.of(
                new FileSystemResource("a.pdf"),
                new FileSystemResource("b.docx")))
        .build();

List<Document> documents = reader.get();
```

A single non-file resource (e.g. `ByteArrayResource`) needs an explicit MIME type:

```java
var reader = XbergDocumentReader.builder()
        .resource(new ByteArrayResource(bytes))
        .mimeType("application/pdf")
        .build();
```

`XbergDocumentReader` implements `DocumentReader`, so `read()` is also available and delegates to
`get()`.

## Splitting strategy

Each extracted document is split into `Document` instances using the highest-granularity output
available, in priority order:

1. **Chunks** — one `Document` per chunk when chunking is configured. Chunk metadata (index, token
   count, page range, `heading_path`) is attached — ready for RAG.
2. **Elements** — one `Document` per structural element (title, paragraph, table…) with element type
   and bounding box.
3. **Pages** — one `Document` per page.
4. **Whole document** — a single `Document` with the full content.

## Configuration

Pass an Xberg `ExtractionConfig` to control chunking, OCR, keyword extraction, NER, and every other
capability the engine exposes:

```java
import io.xberg.ChunkingConfig;
import io.xberg.ExtractionConfig;

var config = ExtractionConfig.builder()
        .withChunking(ChunkingConfig.builder().withMaxCharacters(1000L).build())
        .build();

var reader = XbergDocumentReader.builder()
        .resource(new FileSystemResource("report.pdf"))
        .extractionConfig(config)
        .metadata("tenant", "acme")
        .build();
```

Extraction metadata (title, authors, language, quality score, format-specific fields) is mapped onto
each `Document`. Any keys supplied via `metadata(...)` take precedence.

## Supported formats

Xberg extracts from 107 formats including PDF, DOCX, PPTX, XLSX, HTML, EPUB, images, and more. See
the [Xberg documentation](https://docs.xberg.io) for the full list.

## Part of Xberg.io

- [Xberg](https://github.com/xberg-io/xberg) — document intelligence: text, tables, metadata from 107 formats with optional OCR.
- [Xberg Enterprise](https://pro.xberg.io) — managed extraction API with SDKs, dashboards, and observability.
- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 165 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.

## License

[MIT](./LICENSE)
