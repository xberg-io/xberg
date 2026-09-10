---
title: "Spring AI"
---

The `spring-ai-xberg` package implements a [Spring AI](https://docs.spring.io/spring-ai/reference/) `DocumentReader` backed by Xberg's document extraction engine. It turns any Spring `Resource` into Spring AI `Document` instances carrying extracted text, tables, and metadata — with optional OCR for scans and images.

Extraction runs locally in-process through the `io.xberg:xberg` native binding. No API key, no cloud call, no data leaves your JVM.

## How it works

1. **Extract** — Xberg parses each resource and runs OCR where needed. A single resource uses `Xberg.extract`;
   multiple resources use `Xberg.extractBatch` in one call to avoid repeated native crossings.
2. **Split** — Each extracted document is split into `Document` instances at the highest granularity available: chunks, then elements, then pages, then the whole document.
3. **Map** — Extraction metadata (title, authors, language, quality score, format-specific fields) plus split-specific fields (chunk index, `heading_path`, element type, bounding box) are attached to each `Document`.
4. **Consume** — The resulting list feeds any Spring AI pipeline: `TextSplitter`, `EmbeddingModel`, `VectorStore`, and retrieval-augmented generation.

## Installation

Add the dependency to your `pom.xml`. Requires Java 21+ and Spring AI 2.0.0.

```xml
<dependency>
    <groupId>io.xberg</groupId>
    <artifactId>spring-ai-xberg</artifactId>
    <version>1.1.6</version>
</dependency>
```

Native binaries ship for Linux (x64/arm64), macOS (arm64), and Windows (x64).

## Quick start

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

## Reading multiple files

Pass several resources to extract them in a single batch call:

```java
var reader = XbergDocumentReader.builder()
        .resources(List.of(
                new FileSystemResource("a.pdf"),
                new FileSystemResource("b.docx")))
        .build();

List<Document> documents = reader.get();
```

A `FileSystemResource` is read directly by Xberg; other resource types are read into memory and submitted as bytes. A single non-file resource (e.g. `ByteArrayResource`) requires an explicit MIME type via `.mimeType(...)`.

## Splitting strategy

The reader picks the highest-granularity output available, in priority order.

|         | Produces | Best for |
| ------- | -------- | -------- |
| Chunks  | One `Document` per chunk, with `heading_path`, page range, and token count | RAG and semantic search |
| Elements | One `Document` per structural element, with type and bounding box | Layout-aware processing |
| Pages   | One `Document` per page | Page-level retrieval |
| Whole   | A single `Document` | Keyword search over full text |

Chunks are only produced when chunking is enabled in `ExtractionConfig`; otherwise the reader falls back to elements, pages, then the whole document.

## Extraction control

Pass an Xberg `ExtractionConfig` to configure chunking, OCR, keyword extraction, NER, and every other capability the engine exposes:

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

Keys supplied via `metadata(...)` are applied to every output `Document` and take precedence over extraction-derived metadata.

## Next steps

- [Format support](/reference/formats/) — full list of supported file types
- [OCR guide](/guides/ocr/) — language packs, engine selection, tuning
- For the complete API, see the [spring-ai-xberg readme](https://github.com/xberg-io/xberg/tree/main/integrations/java/spring-ai). For general Spring AI usage, see the [Spring AI docs](https://docs.spring.io/spring-ai/reference/).
