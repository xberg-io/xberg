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
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.9.1" alt="Go">
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
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C">
  </a>
  <a href="https://artifacthub.io/packages/search?repo=kreuzberg">
    <img src="https://img.shields.io/endpoint?url=https://artifacthub.io/badge/repository/kreuzberg" alt="Artifact Hub">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-Elastic--2.0-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-007ec6" alt="Documentation">
  </a>
  <a href="https://docs.kreuzberg.dev/demo.html">
    <img src="https://img.shields.io/badge/%E2%96%B6%EF%B8%8F_Live_Demo-007ec6" alt="Live Demo">
  </a>
  <a href="https://huggingface.co/Kreuzberg">
    <img src="https://img.shields.io/badge/%F0%9F%A4%97_Hugging_Face-007ec6" alt="Hugging Face">
  </a>
</div>

<img width="3384" height="573" alt="Linkedin- Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Extract text, metadata, and code intelligence from 97+ file formats and 305 programming languages at native speeds without needing a GPU.

## Key Features

- **Code intelligence** – Extract functions, classes, imports, symbols, and docstrings from [248 programming languages](https://docs.tree-sitter-language-pack.kreuzberg.dev) via tree-sitter. Results in `ExtractionResult.code_intelligence` with semantic chunking
- **Extensible architecture** – Plugin system for custom OCR backends, validators, post-processors, document extractors, and renderers
- **Polyglot** – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, C#, PHP, Elixir, R, and C
- **91+ file formats** – PDF, Office documents, images, HTML, XML, emails, archives, academic formats across 8 categories
- **LLM intelligence** – VLM OCR (GPT-4o, Claude, Gemini, Ollama), structured JSON extraction with schema constraints, and provider-hosted embeddings via 146 LLM providers (including local engines: Ollama, LM Studio, vLLM, llama.cpp) through [liter-llm](https://github.com/kreuzberg-dev/liter-llm)
- **OCR support** – Tesseract (all bindings, including Tesseract-WASM for browsers), PaddleOCR (all native bindings), EasyOCR (Python), VLM OCR (146 vision model providers including local engines), extensible via plugin API
- **High performance** – Rust core with native PDFium, SIMD optimizations and full parallelism
- **Flexible deployment** – Use as library, CLI tool, REST API server, or MCP server
- **TOON wire format** – Token-efficient serialization for LLM/RAG pipelines, ~30-50% fewer tokens than JSON
- **GFM-quality output** – Comrak-based rendering with proper fenced code blocks, table nodes, bracket escaping, and cross-format parity (Markdown, HTML, Djot, Plain)
- **HTML passthrough** – HTML-to-Markdown conversion uses html-to-markdown output directly, bypassing lossy intermediate round-trips
- **Memory efficient** – Streaming parsers for multi-GB files

**[Complete Documentation](https://kreuzberg.dev/)** | **[Live Demo](https://docs.kreuzberg.dev/demo.html)** | **[Installation Guides](#installation)**

## Installation

Each language binding provides comprehensive documentation with examples and best practices. Choose your platform to get started:

**Scripting Languages:**

- **[Python](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/python)** – PyPI package, async/sync APIs, OCR backends (Tesseract, PaddleOCR, EasyOCR)
- **[Ruby](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/ruby)** – RubyGems package, idiomatic Ruby API, native bindings
- **[PHP](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/php)** – Composer package, modern PHP 8.4+ support, type-safe API, async extraction
- **[Elixir](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/elixir)** – Hex package, OTP integration, concurrent processing
- **[R](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/r)** – r-universe package, idiomatic R API, extendr bindings

**JavaScript/TypeScript:**

- **[@kreuzberg/node](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-node)** – Native NAPI-RS bindings for Node.js/Bun, fastest performance
- **[@kreuzberg/wasm](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/typescript)** – WebAssembly for browsers/Deno/Cloudflare Workers, full feature parity (PDF, Excel, OCR, archives)

**Compiled Languages:**

- **[Go](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go)** – Go module with FFI bindings, context-aware async
- **[Java](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/java)** – Maven Central, Foreign Function & Memory API
- **[C#](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/csharp)** – NuGet package, .NET 6.0+, full async/await support

**Native:**

- **[Rust](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg)** – Core library, flexible feature flags, zero-copy APIs
- **[C (FFI)](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-ffi)** – C header + shared library, pkg-config/CMake support, cross-platform

**Containers:**

- **[Docker](https://docs.kreuzberg.dev/guides/docker/)** – Official images with API, CLI, and MCP server modes (Core: ~1.0-1.3GB, Full: ~1.0-1.3GB with OCR + legacy format support)

**Command-Line:**

- **[CLI](https://docs.kreuzberg.dev/cli/usage/)** – Cross-platform binary, batch processing, MCP server mode

> All language bindings include precompiled binaries for both x86_64 and aarch64 architectures on Linux and macOS.

## Platform Support

Complete architecture coverage across all language bindings:

| Language | Linux x86_64 | Linux aarch64 | macOS ARM64 | Windows x64 |
|----------|:------------:|:-------------:|:-----------:|:-----------:|
| Python | ✅ | ✅ | ✅ | ✅ |
| Node.js | ✅ | ✅ | ✅ | ✅ |
| WASM | ✅ | ✅ | ✅ | ✅ |
| Ruby | ✅ | ✅ | ✅ | - |
| R | ✅ | ✅ | ✅ | ✅ |
| Elixir | ✅ | ✅ | ✅ | ✅ |
| Go | ✅ | ✅ | ✅ | ✅ |
| Java | ✅ | ✅ | ✅ | ✅ |
| C# | ✅ | ✅ | ✅ | ✅ |
| PHP | ✅ | ✅ | ✅ | ✅ |
| Rust | ✅ | ✅ | ✅ | ✅ |
| C (FFI) | ✅ | ✅ | ✅ | ✅ |
| CLI | ✅ | ✅ | ✅ | ✅ |
| Docker | ✅ | ✅ | ✅ | - |

**Note**: ✅ = Precompiled binaries available with instant installation. WASM runs in any environment with WebAssembly support (browsers, Deno, Bun, Cloudflare Workers). All platforms are tested in CI. MacOS support is Apple Silicon only.

### Embeddings Support (Optional)

To use embeddings functionality:

1. **Install ONNX Runtime 1.24+**:
   - Linux: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) (Debian packages may have older versions)
   - MacOS: `brew install onnxruntime`
   - Windows: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)

2. Use embeddings in your code - see [Embeddings Guide](https://docs.kreuzberg.dev/features/#embeddings)

**Note:** Kreuzberg requires ONNX Runtime version 1.24+ for embeddings. All other Kreuzberg features work without ONNX Runtime.

## Supported Formats

91+ file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.docm`, `.dotx`, `.dotm`, `.dot`, `.odt`, `.pages` | Full text, tables, lists, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.xltx`, `.xlt`, `.ods`, `.numbers` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.pptm`, `.ppsx`, `.potx`, `.potm`, `.pot`, `.key` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |
| **Database** | `.dbf` | Table data extraction, field type support |
| **Hangul** | `.hwp`, `.hwpx` | Korean document format, text extraction |

### Images (OCR-Enabled)

| Category | Formats | Features |
|----------|---------|----------|
| **Raster** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif` | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.jbig2`, `.jb2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | Pure Rust decoders (JPEG 2000, JBIG2), OCR, table detection |
| **Vector** | `.svg` | DOM parsing, embedded text, graphics metadata |

### Web & Data

| Category | Formats | Features |
|----------|---------|----------|
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg` | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` | Schema detection, nested structures, validation |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.djot`, `.mdx`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, Djot, MDX, reStructuredText, Org Mode, Rich Text |

### Email & Archives

| Category | Formats | Features |
|----------|---------|----------|
| **Email** | `.eml`, `.msg` | Headers, body (HTML/plain), attachments, UTF-16 support |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | Recursive extraction, nested archives, metadata |

### Academic & Scientific

| Category | Formats | Features |
|----------|---------|----------|
| **Citations** | `.bib`, `.ris`, `.nbib`, `.enw`, `.csl` | BibTeX/BibLaTeX, RIS, PubMed/MEDLINE, EndNote XML, CSL JSON |
| **Scientific** | `.tex`, `.latex`, `.typ`, `.typst`, `.jats`, `.ipynb` | LaTeX, Typst, JATS journal articles, Jupyter notebooks |
| **Publishing** | `.fb2`, `.docbook`, `.dbk`, `.opml` | FictionBook, DocBook XML, OPML outlines |
| **Documentation** | `.pod`, `.mdoc`, `.troff` | Perl POD, man pages, troff |

**[Complete Format Reference →](https://docs.kreuzberg.dev/reference/formats/)**

### Code Intelligence (248 Languages)

| Feature | Description |
|---------|-------------|
| **Structure Extraction** | Functions, classes, methods, structs, interfaces, enums |
| **Import/Export Analysis** | Module dependencies, re-exports, wildcard imports |
| **Symbol Extraction** | Variables, constants, type aliases, properties |
| **Docstring Parsing** | Google, NumPy, Sphinx, JSDoc, RustDoc, and 10+ formats |
| **Diagnostics** | Parse errors with line/column positions |
| **Syntax-Aware Chunking** | Split code by semantic boundaries, not arbitrary byte offsets |

Powered by [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) with dynamic grammar download. See [TSLP documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev) for the full language list.

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

**[PDF Configuration →](https://docs.kreuzberg.dev/guides/configuration/)**

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

## AI Coding Assistants

Kreuzberg ships with an [Agent Skill](https://agentskills.io) that teaches AI coding assistants how to use the library correctly. It works with Claude Code, Codex, Gemini CLI, Cursor, VS Code, Amp, Goose, Roo Code, and any tool supporting the Agent Skills standard.

Install the skill into any project using the [Vercel Skills CLI](https://github.com/vercel-labs/skills):

```bash
npx skills add kreuzberg-dev/kreuzberg
```

The skill is located at [`skills/kreuzberg/SKILL.md`](skills/kreuzberg/SKILL.md) and is automatically discovered by supported AI coding tools once installed.

## Documentation

- **[Installation Guide](https://docs.kreuzberg.dev/getting-started/installation/)** – Setup and dependencies
- **[User Guide](https://docs.kreuzberg.dev/guides/extraction/)** – Comprehensive usage guide
- **[API Reference](https://docs.kreuzberg.dev/reference/api-python/)** – Complete API documentation
- **[Format Support](https://docs.kreuzberg.dev/reference/formats/)** – Supported file formats
- **[OCR Backends](https://docs.kreuzberg.dev/guides/ocr/)** – OCR engine setup
- **[CLI Guide](https://docs.kreuzberg.dev/cli/usage/)** – Command-line usage
- **[Migration Guides](https://docs.kreuzberg.dev/migration/from-unstructured/)** – Upgrading from other libraries

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Elastic License 2.0 (ELv2) - see [LICENSE](LICENSE) for details. See [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license) for the full license text.
