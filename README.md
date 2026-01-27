# Kreuzberg

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
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.2.1" alt="Go">
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

<img width="3384" height="573" alt="Linkedin- Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />



<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Extract text and metadata from a wide range of file formats (57+), generate embeddings and post-process at native speeds without needing a GPU.

## Key Features

- **Extensible architecture** – Plugin system for custom OCR backends, validators, post-processors, and document extractors
- **Polyglot** – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#, PHP, and Elixir
- **57 file formats** – PDF, Office documents, images, HTML, XML, emails, archives, academic formats across 8 categories
- **OCR support** – Tesseract (all languages via native binding), EasyOCR/PaddleOCR (Python), Guten (Node.js), extensible via plugin API
- **High performance** – Rust core with native PDFium, SIMD optimizations and full parallelism
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **Memory efficient** – Streaming parsers for multi-GB files

**[Complete Documentation](https://kreuzberg.dev/)** | **[Installation Guides](#installation)**

## Installation

Each language binding provides comprehensive documentation with examples and best practices. Choose your platform to get started:

**Scripting Languages:**
- **[Python](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/python)** – PyPI package, async/sync APIs, OCR backends (Tesseract, EasyOCR, PaddleOCR)
- **[Ruby](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/ruby)** – RubyGems package, idiomatic Ruby API, native bindings
- **[PHP](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/php)** – Composer package, modern PHP 8.2+ support, type-safe API
- **[Elixir](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/elixir)** – Hex package, OTP integration, concurrent processing

**JavaScript/TypeScript:**
- **[@kreuzberg/node](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-node)** – Native NAPI-RS bindings for Node.js/Bun, fastest performance
- **[@kreuzberg/wasm](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/typescript)** – WebAssembly for browsers/Deno/Cloudflare Workers

**Compiled Languages:**
- **[Go](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go)** – Go module with FFI bindings, context-aware async
- **[Java](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/java)** – Maven Central, Foreign Function & Memory API
- **[C#](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/csharp)** – NuGet package, .NET 6.0+, full async/await support

**Native:**
- **[Rust](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg)** – Core library, flexible feature flags, zero-copy APIs

**Containers:**
- **[Docker](https://docs.kreuzberg.dev/guides/docker/)** – Official images with API, CLI, and MCP server modes (Core: ~1.0-1.3GB, Full: ~1.5-2.1GB with LibreOffice)

**Command-Line:**
- **[CLI](https://docs.kreuzberg.dev/cli/usage/)** – Cross-platform binary, batch processing, MCP server mode

> All language bindings include precompiled binaries for both x86_64 and aarch64 architectures on Linux and macOS.

## Platform Support

Complete architecture coverage across all language bindings:

| Language | Linux x86_64 | Linux aarch64 | macOS ARM64 | Windows x64 |
|----------|:------------:|:-------------:|:-----------:|:-----------:|
| Python | ✅ | ✅ | ✅ | ✅ |
| Node.js | ✅ | ✅ | ✅ | ✅ |
| Ruby | ✅ | ✅ | ✅ | - |
| Elixir | ✅ | ✅ | ✅ | ✅ |
| Go | ✅ | ✅ | ✅ | ✅ |
| Java | ✅ | ✅ | ✅ | ✅ |
| C# | ✅ | ✅ | ✅ | ✅ |
| PHP | ✅ | ✅ | ✅ | ✅ |
| Rust | ✅ | ✅ | ✅ | ✅ |
| CLI | ✅ | ✅ | ✅ | ✅ |
| Docker | ✅ | ✅ | ✅ | - |

**Note**: ✅ = Precompiled binaries available with instant installation. All platforms are tested in CI. macOS support is Apple Silicon only.

### Embeddings Support (Optional)

To use embeddings functionality:

1. **Install ONNX Runtime 1.22.x**:
   - Linux: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) (Debian packages may have older versions)
   - macOS: `brew install onnxruntime`
   - Windows: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)

2. Use embeddings in your code - see [Embeddings Guide](https://docs.kreuzberg.dev/features/#embeddings)

**Note:** Kreuzberg requires ONNX Runtime version 1.22.x for embeddings. All other Kreuzberg features work without ONNX Runtime.

## Supported Formats

57 file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.odt` | Full text, tables, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.ods` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.ppt`, `.ppsx` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |

### Images (OCR-Enabled)

| Category | Formats | Features |
|----------|---------|----------|
| **Raster** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif` | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | OCR, table detection, format-specific metadata |
| **Vector** | `.svg` | DOM parsing, embedded text, graphics metadata |

### Web & Data

| Category | Formats | Features |
|----------|---------|----------|
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg` | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` | Schema detection, nested structures, validation |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.djot`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, Djot, reStructuredText, Org Mode |

### Email & Archives

| Category | Formats | Features |
|----------|---------|----------|
| **Email** | `.eml`, `.msg` | Headers, body (HTML/plain), attachments, threading |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | File listing, nested archives, metadata |

### Academic & Scientific

| Category | Formats | Features |
|----------|---------|----------|
| **Citations** | `.bib`, `.biblatex`, `.ris`, `.enw`, `.csl` | Bibliography parsing, citation extraction |
| **Scientific** | `.tex`, `.latex`, `.typ`, `.jats`, `.ipynb`, `.docbook` | LaTeX, Jupyter notebooks, PubMed JATS |
| **Documentation** | `.opml`, `.pod`, `.mdoc`, `.troff` | Technical documentation formats |

**[Complete Format Reference →](https://docs.kreuzberg.dev/reference/formats/)**

## Key Features

<details>
<summary><strong>OCR with Table Extraction</strong></summary>

Multiple OCR backends (Tesseract, EasyOCR, PaddleOCR) with intelligent table detection and reconstruction. Extract structured data from scanned documents and images with configurable accuracy thresholds.

**[OCR Backend Documentation →](https://docs.kreuzberg.dev/guides/ocr/)**

</details>

<details>
<summary><strong>Batch Processing</strong></summary>

Process multiple documents concurrently with configurable parallelism. Optimize throughput for large-scale document processing workloads with automatic resource management.

**[Batch Processing Guide →](https://docs.kreuzberg.dev/features/#batch-processing)**

</details>

<details>
<summary><strong>Password-Protected PDFs</strong></summary>

Handle encrypted PDFs with single or multiple password attempts. Supports both RC4 and AES encryption with automatic fallback strategies.

**[PDF Configuration →](https://docs.kreuzberg.dev/migration/v3-to-v4/#password-protected-pdfs)**

</details>

<details>
<summary><strong>Language Detection</strong></summary>

Automatic language detection in extracted text using fast-langdetect. Configure confidence thresholds and access per-language statistics.

**[Language Detection Guide →](https://docs.kreuzberg.dev/features/#language-detection)**

</details>

<details>
<summary><strong>Metadata Extraction</strong></summary>

Extract comprehensive metadata from all supported formats: authors, titles, creation dates, page counts, EXIF data, and format-specific properties.

**[Metadata Guide →](https://docs.kreuzberg.dev/reference/types/#metadata)**

</details>

## Documentation

- **[Installation Guide](https://docs.kreuzberg.dev/getting-started/installation/)** – Setup and dependencies
- **[User Guide](https://docs.kreuzberg.dev/guides/extraction/)** – Comprehensive usage guide
- **[API Reference](https://docs.kreuzberg.dev/reference/api-python/)** – Complete API documentation
- **[Format Support](https://docs.kreuzberg.dev/reference/formats/)** – Supported file formats
- **[OCR Backends](https://docs.kreuzberg.dev/guides/ocr/)** – OCR engine setup
- **[CLI Guide](https://docs.kreuzberg.dev/cli/usage/)** – Command-line usage
- **[Migration Guide](https://docs.kreuzberg.dev/migration/v3-to-v4/)** – Upgrading from v3

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details. You can use Kreuzberg freely in both commercial and closed-source products with no obligations, no viral effects, and no licensing restrictions.
