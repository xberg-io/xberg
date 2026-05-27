# FFI (C/C++)

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://github.com/kreuzberg-dev/alef">
    <img src="https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6" alt="Bindings">
  </a>
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
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
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go/v5">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v5*" alt="Go">
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
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://pub.dev/packages/kreuzberg">
    <img src="https://img.shields.io/pub/v/kreuzberg?label=Dart&color=007ec6" alt="Dart">
  </a>
  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg-android">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg-android?label=Kotlin&color=007ec6" alt="Kotlin">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/swift">
    <img src="https://img.shields.io/badge/Swift-SPM-007ec6" alt="Swift">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/zig">
    <img src="https://img.shields.io/badge/Zig-package-007ec6" alt="Zig">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-ghcr.io-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/charts%2Fkreuzberg">
    <img src="https://img.shields.io/badge/Helm-ghcr.io-007ec6?logo=helm&logoColor=white" alt="Helm">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-Elastic--2.0-007ec6" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/Docs-kreuzberg-007ec6" alt="Documentation">
  </a>
  <a href="https://huggingface.co/Kreuzberg">
    <img src="https://img.shields.io/badge/Hugging%20Face-Kreuzberg-007ec6" alt="Hugging Face">
  </a>
</div>

<div align="center" style="margin: 24px 0 0;">
  <a href="https://kreuzberg.dev">
    <img alt="Kreuzberg" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />
  </a>
</div>

<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white" alt="Join Discord">
  </a>
  <a href="https://docs.kreuzberg.dev/demo.html">
    <img height="22" src="https://img.shields.io/badge/Live%20Demo-Open-007ec6?logo=webassembly&logoColor=white" alt="Live Demo">
  </a>
</div>

Extract text, tables, images, and metadata from 90+ file formats and 300+ programming languages including PDF, Office documents, and images. C/C++ FFI bindings providing a stable ABI for native integration, shared library distribution, and cross-language interop.

## What This Package Provides

- **Document intelligence core** — extract text, tables, images, metadata, entities, keywords, and code intelligence from one API.
- **Format coverage** — PDF, Office, images, HTML/XML, email, archives, notebooks, citations, scientific formats, and plain text.
- **OCR choices** — Tesseract, PaddleOCR, EasyOCR where supported, VLM OCR through liter-llm, and plugin hooks for custom backends.
- **Same engine as every binding** — Rust, Python, Node.js, Go, Java, PHP, Ruby, .NET, Elixir, R, WASM, Kotlin Android, Swift, Dart, Zig, and C FFI share the same Rust implementation.
- **C ABI** — stable shared library surface for custom hosts and secondary bindings.

## Installation

### Package Installation

Build the shared library from the workspace:

```bash
cargo build --release -p kreuzberg-ffi
```

The built artifacts are emitted under `target/release/` (`libkreuzberg_ffi.{so,dylib,a}`) along with the C header at `crates/kreuzberg-ffi/include/kreuzberg.h`.

### System Requirements
- A C/C++ toolchain (clang, gcc, or MSVC) and a Rust toolchain (`rustup`) for building from source
- A `pkg-config` or CMake-aware build system that can locate `libkreuzberg_ffi` and `kreuzberg.h`
- Optional: [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) version 1.22.x for embeddings support
- Optional: [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) for OCR functionality

## Quick Start

### Basic Extraction

Extract text, metadata, and structure from any supported document format:

<!-- snippet not found:  -->

### Common Use Cases

#### Extract with Custom Configuration

Most use cases benefit from configuration to control extraction behavior:

#### Table Extraction

See [Configuration Guide](https://docs.kreuzberg.dev/guides/configuration/) for table extraction options.

#### Processing Multiple Files

### Next Steps

- **[Installation Guide](https://docs.kreuzberg.dev/getting-started/installation/)** - Platform-specific setup
- **[API Documentation](https://docs.kreuzberg.dev/reference/api-python/)** - Complete API reference
- **[Examples & Guides](https://docs.kreuzberg.dev/)** - Full code examples and usage guides
- **[Configuration Guide](https://docs.kreuzberg.dev/guides/configuration/)** - Advanced configuration options

## Features

### Supported File Formats (90+)

90+ file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

#### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.docm`, `.dotx`, `.dotm`, `.dot`, `.odt` | Full text, tables, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.xltx`, `.xlt`, `.ods` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.pptm`, `.ppsx`, `.potx`, `.potm`, `.pot`, `.ppt` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |
| **Database** | `.dbf` | Table data extraction, field type support |
| **Hangul** | `.hwp`, `.hwpx` | Korean document format, text extraction |

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

#### Code Intelligence (300+ Languages)

| Feature | Description |
|---------|-------------|
| **Structure Extraction** | Functions, classes, methods, structs, interfaces, enums |
| **Import/Export Analysis** | Module dependencies, re-exports, wildcard imports |
| **Symbol Extraction** | Variables, constants, type aliases, properties |
| **Docstring Parsing** | Google, NumPy, Sphinx, JSDoc, RustDoc, and 10+ formats |
| **Diagnostics** | Parse errors with line/column positions |
| **Syntax-Aware Chunking** | Split code by semantic boundaries, not arbitrary byte offsets |

Powered by [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — [documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev).

**[Complete Format Reference](https://docs.kreuzberg.dev/reference/formats/)**

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
- **Code Intelligence** - Extract structure, imports, exports, symbols, and docstrings from [300+ programming languages](https://docs.tree-sitter-language-pack.kreuzberg.dev) via tree-sitter
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

### OCR Configuration Example

<!-- snippet not found:  -->

## Async Support

This binding provides full async/await support for non-blocking document processing:

<!-- snippet not found:  -->

## Plugin System

Kreuzberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://docs.kreuzberg.dev/guides/plugins/).

## Embeddings Support

Generate vector embeddings for extracted text using the built-in ONNX Runtime support. Requires ONNX Runtime installation.

**[Embeddings Guide](https://docs.kreuzberg.dev/features/#embeddings)**

## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://docs.kreuzberg.dev/guides/configuration/)**

## Documentation

- **[Official Documentation](https://docs.kreuzberg.dev/)**
- **[API Reference](https://docs.kreuzberg.dev/reference/api-python/)**
- **[Examples & Guides](https://docs.kreuzberg.dev/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CONTRIBUTING.md).

## Part of Kreuzberg.dev

- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

Elastic-2.0 License — see [LICENSE](../../LICENSE) for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Discussions**: [Ask questions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
