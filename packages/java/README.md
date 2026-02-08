# Java

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://pypi.org/project/kreuzberg/">
    <img src="https://img.shields.io/pypi/v/kreuzberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/node">
    <img src="https://img.shields.io/npm/v/@kreuzberg/node?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/wasm">
    <img src="https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM&color=007ec6" alt="WASM">
  </a>

  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.0.0" alt="Go">
  </a>
  <a href="https://www.nuget.org/packages/Kreuzberg/">
    <img src="https://img.shields.io/nuget/v/Kreuzberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/kreuzberg/kreuzberg">
    <img src="https://img.shields.io/packagist/v/kreuzberg/kreuzberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/kreuzberg">
    <img src="https://img.shields.io/gem/v/kreuzberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Documentation">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>


Extract text, tables, images, and metadata from 75+ file formats including PDF, Office documents, and images. Java bindings with type-safe API, Foreign Function & Memory API integration, and native performance.


## Installation

### Package Installation


Install via one of the supported package managers:



**Maven:**
```xml
<dependency>
    <groupId>dev.kreuzberg</groupId>
    <artifactId>kreuzberg</artifactId>
    <version>4.0.0</version>
</dependency>
```




**Gradle:**
```gradle
implementation 'dev.kreuzberg:kreuzberg:4.0.0'
```





### System Requirements

- **Java 11+** required
- Optional: [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) version 1.23+ for embeddings support
- Optional: [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) for OCR functionality



## Quick Start

### Basic Extraction

Extract text, metadata, and structure from any supported document format:

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.io.IOException;
import java.util.Map;

public class BasicUsage {
    public static void main(String[] args) throws IOException {
        ExtractionResult result = Kreuzberg.extractFile("document.pdf");

        System.out.println("Content:");
        System.out.println(result.getContent());

        System.out.println("\nMetadata:");
        Map<String, Object> metadata = result.getMetadata();
        if (metadata != null) {
            System.out.println("Title: " + metadata.get("title"));
            System.out.println("Author: " + metadata.get("author"));
        }

        System.out.println("\nTables found: " + result.getTables().size());
        System.out.println("Images found: " + result.getImages().size());
    }
}
```


### Common Use Cases

#### Extract with Custom Configuration

Most use cases benefit from configuration to control extraction behavior:


**With OCR (for scanned documents):**

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import java.io.IOException;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng")
                    .build())
                .build();

            ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
            System.out.println(result.getContent());
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```




#### Table Extraction


See [Table Extraction Guide](https://kreuzberg.dev/features/table-extraction/) for detailed examples.



#### Processing Multiple Files


```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;

try {
    List<String> files = Arrays.asList("doc1.pdf", "doc2.docx", "doc3.pptx");

    List<ExtractionResult> results = Kreuzberg.batchExtractFiles(files, null);

    for (int i = 0; i < results.size(); i++) {
        ExtractionResult result = results.get(i);
        System.out.println("File " + (i + 1) + ": " + result.getContent().length() + " characters");
    }
} catch (IOException | KreuzbergException e) {
    e.printStackTrace();
}
```





#### Async Processing

For non-blocking document processing:

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;

public class Example {
    public static void main(String[] args) {
        CompletableFuture<ExtractionResult> future =
            Kreuzberg.extractFileAsync(Path.of("document.pdf"), null);

        future.thenAccept(result -> {
            System.out.println(result.getContent());
            System.out.println("Tables: " + result.getTables().size());
        }).join();
    }
}
```






### Next Steps

- **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** - Platform-specific setup
- **[API Documentation](https://kreuzberg.dev/api/)** - Complete API reference
- **[Examples & Guides](https://kreuzberg.dev/guides/)** - Full code examples and usage guides
- **[Configuration Guide](https://kreuzberg.dev/guides/configuration/)** - Advanced configuration options



## Features

### Supported File Formats (75+)

75+ file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

#### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.odt` | Full text, tables, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.ods` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.ppt`, `.ppsx` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |

#### Images (OCR-Enabled)

| Category | Formats | Features |
|----------|---------|----------|
| **Raster** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif` | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.jbig2`, `.jb2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | OCR via hayro-jpeg2000 (pure Rust decoder), JBIG2 support, table detection, format-specific metadata |
| **Vector** | `.svg` | DOM parsing, embedded text, graphics metadata |

#### Web & Data

| Category | Formats | Features |
|----------|---------|----------|
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg` | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` | Schema detection, nested structures, validation |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.djot`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, Djot, reStructuredText, Org Mode |

#### Email & Archives

| Category | Formats | Features |
|----------|---------|----------|
| **Email** | `.eml`, `.msg` | Headers, body (HTML/plain), attachments, threading |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | File listing, nested archives, metadata |

#### Academic & Scientific

| Category | Formats | Features |
|----------|---------|----------|
| **Citations** | `.bib`, `.biblatex`, `.ris`, `.nbib`, `.enw`, `.csl` | Structured parsing: RIS (structured), PubMed/MEDLINE, EndNote XML (structured), BibTeX, CSL JSON |
| **Scientific** | `.tex`, `.latex`, `.typst`, `.jats`, `.ipynb`, `.docbook` | LaTeX, Jupyter notebooks, PubMed JATS |
| **Documentation** | `.opml`, `.pod`, `.mdoc`, `.troff` | Technical documentation formats |

**[Complete Format Reference](https://kreuzberg.dev/reference/formats/)**

### Key Capabilities

- **Text Extraction** - Extract all text content with position and formatting information
- **Metadata Extraction** - Retrieve document properties, creation date, author, etc.
- **Table Extraction** - Parse tables with structure and cell content preservation
- **Image Extraction** - Extract embedded images and render page previews
- **OCR Support** - Integrate multiple OCR backends for scanned documents

- **Async/Await** - Non-blocking document processing with concurrent operations


- **Plugin System** - Extensible post-processing for custom text transformation


- **Embeddings** - Generate vector embeddings using ONNX Runtime models

- **Batch Processing** - Efficiently process multiple documents in parallel
- **Memory Efficient** - Stream large files without loading entirely into memory
- **Language Detection** - Detect and support multiple languages in documents
- **Configuration** - Fine-grained control over extraction behavior

### Performance Characteristics

| Format | Speed | Memory | Notes |
|--------|-------|--------|-------|
| **PDF (text)** | 10-100 MB/s | ~50MB per doc | Fastest extraction |
| **Office docs** | 20-200 MB/s | ~100MB per doc | DOCX, XLSX, PPTX |
| **Images (OCR)** | 1-5 MB/s | Variable | Depends on OCR backend |
| **Archives** | 5-50 MB/s | ~200MB per doc | ZIP, TAR, etc. |
| **Web formats** | 50-200 MB/s | Streaming | HTML, XML, JSON |



## OCR Support

Kreuzberg supports multiple OCR backends for extracting text from scanned documents and images:


- **Tesseract**

- **Paddleocr**


### OCR Configuration Example

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import dev.kreuzberg.config.ExtractionConfig;
import dev.kreuzberg.config.OcrConfig;
import java.io.IOException;

public class Main {
    public static void main(String[] args) {
        try {
            ExtractionConfig config = ExtractionConfig.builder()
                .ocr(OcrConfig.builder()
                    .backend("tesseract")
                    .language("eng")
                    .build())
                .build();

            ExtractionResult result = Kreuzberg.extractFile("scanned.pdf", config);
            System.out.println(result.getContent());
        } catch (IOException | KreuzbergException e) {
            System.err.println("Extraction failed: " + e.getMessage());
        }
    }
}
```




## Async Support

This binding provides full async/await support for non-blocking document processing:

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;

public class Example {
    public static void main(String[] args) {
        CompletableFuture<ExtractionResult> future =
            Kreuzberg.extractFileAsync(Path.of("document.pdf"), null);

        future.thenAccept(result -> {
            System.out.println(result.getContent());
            System.out.println("Tables: " + result.getTables().size());
        }).join();
    }
}
```




## Plugin System

Kreuzberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://kreuzberg.dev/guides/plugins/).




## Embeddings Support

Generate vector embeddings for extracted text using the built-in ONNX Runtime support. Requires ONNX Runtime installation.

**[Embeddings Guide](https://kreuzberg.dev/features/#embeddings)**



## Batch Processing

Process multiple documents efficiently:

```java
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.KreuzbergException;
import java.io.IOException;
import java.util.Arrays;
import java.util.List;

try {
    List<String> files = Arrays.asList("doc1.pdf", "doc2.docx", "doc3.pptx");

    List<ExtractionResult> results = Kreuzberg.batchExtractFiles(files, null);

    for (int i = 0; i < results.size(); i++) {
        ExtractionResult result = results.get(i);
        System.out.println("File " + (i + 1) + ": " + result.getContent().length() + " characters");
    }
} catch (IOException | KreuzbergException e) {
    e.printStackTrace();
}
```




## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://kreuzberg.dev/guides/configuration/)**

## Documentation

- **[Official Documentation](https://kreuzberg.dev/)**
- **[API Reference](https://kreuzberg.dev/reference/api-java/)**
- **[Examples & Guides](https://kreuzberg.dev/guides/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Discussions**: [Ask questions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
