# Kreuzberg Spring AI DocumentReader

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg-spring-ai-document-reader"><img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg-spring-ai-document-reader?label=Maven%20Central&color=007ec6" alt="Maven Central"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License"></a>
  <a href="https://docs.xberg.io"><img src="https://img.shields.io/badge/docs-xberg.io-blue" alt="Docs"></a>
</div>

<img width="3384" height="573" alt="Kreuzberg Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

A Spring AI `DocumentReader` integration for the [Kreuzberg](https://github.com/xberg-io/kreuzberg) document extraction engine. Extract text, metadata, tables, and structural elements from 100+ document formats with native OCR support in 80+ languages.

## Overview

Kreuzberg is a high-performance document extraction engine that handles diverse formats (PDF, DOCX, XLSX, HTML, images, archives, and more) with advanced layout detection, table extraction, and native optical character recognition. This library integrates Kreuzberg seamlessly into Spring AI's ETL pipeline, giving enterprise Java applications access to sophisticated document processing capabilities through a familiar API.

Unlike traditional readers that split on fixed token boundaries, the Kreuzberg Spring AI DocumentReader offers semantic-aware chunking with heading context, element-based splitting for structured retrieval, and rich metadata including detected languages, quality scores, and format-specific fields. All processing happens locally in-process — no external service dependencies.

## Requirements

- **Java 25** with `--enable-preview` flag. This is required by the underlying Kreuzberg library, which uses Java's Foreign Function & Memory API (Panama) to bind native document processing code.
- **Maven 3.8.1** or later
- **Spring AI 1.0.0** or later

To configure your project for Java 25, set the Maven compiler plugin:

```xml
<plugin>
  <groupId>org.apache.maven.plugins</groupId>
  <artifactId>maven-compiler-plugin</artifactId>
  <version>3.14.0</version>
  <configuration>
    <release>25</release>
    <compilerArgs>
      <arg>--enable-preview</arg>
    </compilerArgs>
  </configuration>
</plugin>
```

## Installation

For local builds, add the dependency to your `pom.xml`:

```xml
<dependency>
  <groupId>dev.kreuzberg</groupId>
  <artifactId>kreuzberg-spring-ai-document-reader</artifactId>
  <version>0.1.0</version>
</dependency>
```

When published to Maven Central, the artifact will be available from the standard repositories. Note that the JAR includes ~43 MB of native libraries required for the extraction engine.

## Quickstart Examples

### 1. File-Based Extraction

Extract text from a PDF on the filesystem:

```java
Resource resource = new FileSystemResource("report.pdf");
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(resource)
    .build()
    .get();
```

### 2. Byte Array with Explicit MIME Type

Process uploaded file content with an explicit MIME type:

```java
byte[] uploadedBytes = multipartFile.getBytes();
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(new ByteArrayResource(uploadedBytes))
    .mimeType("application/pdf")
    .build()
    .get();
```

### 3. Classpath Resource

Load a document bundled with your application:

```java
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(new ClassPathResource("docs/contract.pdf"))
    .build()
    .get();
```

### 4. Custom Extraction Configuration

Enable OCR for scanned documents and request markdown output:

```java
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("scanned.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .forceOcr(true)
        .outputFormat("markdown")
        .build())
    .build()
    .get();
```

### 5. With User-Supplied Metadata

Add custom metadata fields that will be attached to all extracted documents:

```java
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("financial_report.pdf"))
    .metadata("department", "legal")
    .metadata("classification", "confidential")
    .metadata("fiscal_year", "2024")
    .build()
    .get();
```

### 6. Chunk-Based Splitting for RAG

Extract documents with automatic heading-aware chunking, ready for vector store ingestion:

```java
List<Document> chunks = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("whitepaper.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .outputFormat("markdown")
        .chunking(ChunkingConfig.builder()
            .chunkerType("markdown")
            .maxChars(1000)
            .maxOverlap(200)
            .prependHeadingContext(true)
            .build())
        .build())
    .build()
    .get();

// Chunks include heading context in metadata — skip TokenTextSplitter
vectorStore.add(chunks);
```

### 7. Element-Based Splitting

Extract documents as semantic elements (titles, headings, paragraphs, tables) for structured RAG:

```java
List<Document> elements = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("contract.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .resultFormat("element_based")
        .build())
    .build()
    .get();

// Each Document has element_type metadata: "title", "heading", "narrative_text", "table", etc.
elements.forEach(doc ->
    System.out.println(doc.getMetadata().get("element_type") + ": " +
        doc.getText().substring(0, Math.min(50, doc.getText().length()))));
```

## Document Splitting

The reader automatically splits extraction results into `Document` objects. The splitting mode depends on which `ExtractionConfig` options you enable. The reader uses the first non-empty result in priority order:

| Priority | Mode | Config Required | Documents Produced |
|---|---|---|---|
| 1 (highest) | Chunks | `ChunkingConfig` | One per chunk — heading-aware, sized for RAG |
| 2 | Elements | `resultFormat("element_based")` | One per semantic element (title, heading, paragraph, table, etc.) |
| 3 | Pages | `PageConfig(extractPages=true)` | One per page |
| 4 (lowest) | Whole document | None (default) | Single Document with all content |

### Native Chunking (Replaces TokenTextSplitter)

With `ChunkingConfig`, Kreuzberg handles chunking natively using heading-aware markdown splitting with configurable sizing and overlap control. Skip Spring AI's `TokenTextSplitter` entirely and go straight to your vector store:

```java
List<Document> chunks = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("report.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .chunking(ChunkingConfig.builder()
            .maxChars(1000)
            .maxOverlap(200)
            .build())
        .build())
    .build()
    .get();

vectorStore.add(chunks);
```

Each chunk Document includes `chunk_index`, `total_chunks`, and `heading_context` (JSON) in metadata. See the metadata reference table below for details.

### Element-Based Splitting

Each semantic element becomes its own Document — useful for structured RAG where you want to retrieve a specific table, heading, or paragraph rather than an arbitrary text window:

```java
List<Document> elements = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("contract.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .resultFormat("element_based")
        .build())
    .build()
    .get();

// Iterate over elements with type information
for (Document doc : elements) {
    String type = (String) doc.getMetadata().get("element_type");
    System.out.println("Element type: " + type);
}
```

Each element Document includes `element_type`, `element_id`, `page_number`, and bounding box coordinates (`bbox_x0`, `bbox_y0`, `bbox_x1`, `bbox_y1`) in metadata.

### Page Splitting

Extract one Document per page:

```java
List<Document> pages = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("manual.pdf"))
    .extractionConfig(ExtractionConfig.builder()
        .pages(PageConfig.builder()
            .extractPages(true)
            .build())
        .build())
    .build()
    .get();

// Each Document has a "page" metadata key (0-indexed)
pages.forEach(doc ->
    System.out.println("Page " + doc.getMetadata().get("page")));
```

### Default (Whole Document)

With no splitting config, the reader returns a single Document containing the full extracted text. This is the simplest mode for applications that don't need semantic splitting:

```java
List<Document> docs = KreuzbergDocumentReader.builder()
    .resource(new FileSystemResource("document.pdf"))
    .build()
    .get();

// docs.size() == 1
String fullText = docs.get(0).getText();
```

## Metadata Reference

All extracted documents include rich metadata fields extracted from the document and its content. The following table lists all standard metadata keys:

| Key | Type | Source | Example |
|---|---|---|---|
| `source` | String | Resource filename or `bytes://<mimeType>` | `"document.pdf"` or `"bytes://application/pdf"` |
| `mime_type` | String | Extracted from document | `"application/pdf"` |
| `page_count` | int | Document page count | `42` |
| `detected_languages` | String | Comma-separated detected languages | `"en, de, fr"` |
| `table_count` | int | Number of tables extracted | `3` |
| `quality_score` | float | Document quality assessment (0.0–1.0) | `0.95` |
| `title` | String | Document title metadata | `"Annual Report"` |
| `subject` | String | Document subject metadata | `"Financial Summary"` |
| `authors` | String | Comma-separated author list | `"John Doe, Jane Smith"` |
| `keywords` | String | Comma-separated keywords | `"finance, report, 2024"` |
| `language` | String | Primary language code | `"en"` |
| `created_at` | String | Document creation timestamp | `"2024-01-15T10:30:00Z"` |
| `modified_at` | String | Last modification timestamp | `"2024-03-20T14:45:00Z"` |
| `created_by` | String | Document creator | `"Alice"` |
| `modified_by` | String | Last modifier | `"Bob"` |
| `category` | String | Document category | `"Legal"` |
| `tags` | String | Comma-separated tags | `"urgent, review, archived"` |
| `document_version` | String | Version number or identifier | `"1.2.3"` |
| `abstract_text` | String | Document abstract or summary | `"This report summarizes..."` |
| `output_format` | String | Requested output format | `"markdown"` |
| `tables` | String | JSON array of extracted tables | `"[{"cells":[...],"markdown":"..."}]"` |
| `extracted_keywords` | String | JSON array of extracted keywords with scores | `"[{"text":"...", "score":0.95}]"` |
| `processing_warnings` | String | JSON array of warnings during processing | `"[{"source":"ocr", "message":"..."}]"` |

### Format-Specific Metadata

Additional format-specific fields are automatically included in document metadata. Examples:

- **PDF**: `pdf_version`, `producer`, `is_encrypted`, `width`, `height`
- **Excel**: `sheet_count`, `sheet_names`
- **Email**: `from_email`, `from_name`, `to_emails`, `cc_emails`, `message_id`, `attachments`
- **PowerPoint**: `slide_count`, `slide_names`
- **Images**: `width`, `height`, `format`, `exif`
- **HTML**: `description`, `canonical_url`, `open_graph`, `twitter_card`, `meta_tags`

### Chunk-Specific Metadata

When using chunked splitting, each Document includes:

| Key | Type | Source |
|---|---|---|
| `chunk_index` | int | 0-based chunk index |
| `total_chunks` | int | Total chunks in document |
| `token_count` | int | Approximate token count in chunk (if available) |
| `first_page` | int | First page number in chunk (if available) |
| `last_page` | int | Last page number in chunk (if available) |
| `heading_context` | String | JSON array of heading hierarchy for context |

### Element-Specific Metadata

When using element-based splitting, each Document includes:

| Key | Type | Source |
|---|---|---|
| `element_id` | String | Unique element identifier |
| `element_type` | String | `"title"`, `"heading"`, `"narrative_text"`, `"table"`, etc. |
| `element_index` | int | Index of element in sequence (if available) |
| `page_number` | int | Page number containing element (if available) |
| `bbox_x0` | float | Left edge of bounding box (if available) |
| `bbox_y0` | float | Bottom edge of bounding box (if available) |
| `bbox_x1` | float | Right edge of bounding box (if available) |
| `bbox_y1` | float | Top edge of bounding box (if available) |

### Page-Specific Metadata

When using page-based splitting, each Document includes:

| Key | Type | Source |
|---|---|---|
| `page` | int | 0-based page index |

## Configuration

For comprehensive configuration options, refer to the [Kreuzberg documentation](https://xberg.io/docs). The `ExtractionConfig` builder supports all extraction parameters including:

- OCR settings (force OCR, language selection, confidence thresholds)
- Output format selection (markdown, HTML, plain text)
- Table extraction and processing
- Chunking and splitting configuration
- Page extraction options
- Language detection

## Comparison with Other Readers

| Feature | Kreuzberg | PDFBox | Tika | Markdown |
|---|---|---|---|---|
| Format Support | 100+ formats | PDF only | 200+ (with dependencies) | Markdown only |
| Native OCR | Yes (80+ languages) | No | Yes (with Tesseract) | No |
| Layout Detection | Yes (RT-DETR v2) | Basic | Limited | No |
| Table Extraction | Yes (TATR, SLANeXT) | Basic | Limited | No |
| Semantic Chunking | Yes (heading-aware) | No | No | No |
| Element-Based Splitting | Yes | No | No | No |
| Dependencies (Java) | Jackson (included in Spring Boot) | None | Dozens of specialized libraries | None |
| Local Processing | Yes | Yes | Yes | Yes |
| Metadata Richness | 20+ explicit fields + format-specific | Minimal | Format-specific | Minimal |

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
