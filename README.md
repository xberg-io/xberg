# Kreuzberg

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

Extract text, metadata, transcripts, and code intelligence from 96 file formats and 306 programming languages at native speeds without needing a GPU.

## Key Features

- **Code intelligence** – Extract functions, classes, imports, symbols, and docstrings from [306 programming languages](https://docs.tree-sitter-language-pack.kreuzberg.dev) via tree-sitter. Results in `ExtractionResult.code_intelligence` with semantic chunking
- **Extensible architecture** – Plugin system for custom OCR backends, validators, post-processors, document extractors, and renderers
- **Polyglot** – Native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, Kotlin, C#, PHP, Elixir, R, Dart, Swift, Zig, and C
- **96 file formats** – PDF, Office documents, images, HTML, XML, emails, archives, academic formats across 8 categories
- **Audio/video transcription** – Whisper ONNX transcripts for MP3, M4A, WAV, WebM, and MP4 audio tracks with model caching and offline mode
- **LLM intelligence** – VLM OCR (GPT-4o, Claude, Gemini, Ollama), structured JSON extraction with schema constraints, and provider-hosted embeddings via 143 LLM providers (including local engines: Ollama, LM Studio, vLLM, llama.cpp) through [liter-llm](https://github.com/kreuzberg-dev/liter-llm)
- **OCR support** – Tesseract (all bindings, including Tesseract-WASM for browsers), PaddleOCR (all native bindings), EasyOCR (Python), VLM OCR (143 vision model providers including local engines), extensible via plugin API
- **High performance** – Rust core with pure-Rust PDF, SIMD optimizations and full parallelism
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
- **[PHP](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/php)** – Composer package, modern PHP 8.2+ support, type-safe API, async extraction
- **[Elixir](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/elixir)** – Hex package, OTP integration, concurrent processing
- **[R](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/r)** – r-universe package, idiomatic R API, extendr bindings
- **[Dart / Flutter](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/dart)** – pub.dev package, flutter_rust_bridge runtime, native bindings for macOS/iOS/Android/Linux/Windows

**JavaScript/TypeScript:**

- **[@kreuzberg/node](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-node)** – Native NAPI-RS bindings for Node.js/Bun, fastest performance
- **[@kreuzberg/wasm](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-wasm)** – WebAssembly for browsers/Deno/Cloudflare Workers, comprehensive document and OCR support (PDF, Excel, archives, office formats, real Tesseract via the WASI build) — ORT-dependent features (PaddleOCR, layout detection, embeddings, reranking, auto-rotate, transcription) and server modes (api/mcp/cli) are excluded

**Compiled Languages:**

- **[Go](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/go/v5)** – Go module with FFI bindings, context-aware async
- **[Java](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/java)** – Maven Central, Foreign Function & Memory API
- **[Kotlin](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/kotlin-android)** – Maven Central, Kotlin/JVM with idiomatic data classes, sealed enums, and coroutine-based async
- **[C#](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/csharp)** – NuGet package, .NET 6.0+, full async/await support
- **[Swift](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/swift)** – Swift Package Manager, macOS 13+/iOS 16+, native Swift types and async/await

**Native:**

- **[Rust](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg)** – Core library, flexible feature flags, zero-copy APIs
- **[Zig](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/zig)** – `zig fetch` + `build.zig.zon`, idiomatic error sets, optional types, slice-based memory
- **[C (FFI)](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-ffi)** – C header + shared library, pkg-config/CMake support, cross-platform

**Containers:**

- **[Docker](https://docs.kreuzberg.dev/guides/docker/)** – Official images with API, CLI, and MCP server modes (Core: ~1.0-1.3GB, Full: ~1.0-1.3GB with OCR + legacy format support)

**Command-Line:**

- **[CLI](https://docs.kreuzberg.dev/cli/usage/)** – Cross-platform binary, batch processing, MCP server mode

> All language bindings include precompiled binaries for both x86_64 and aarch64 architectures on Linux and macOS.

## Platform Support

Complete architecture coverage across all language bindings:

| Language | Linux x86_64 | Linux aarch64 | macOS ARM64 | Windows x64 |
| -------- | :----------: | :-----------: | :---------: | :---------: |
| Python   |      ✅      |      ✅       |     ✅      |     ✅      |
| Node.js  |      ✅      |      ✅       |     ✅      |     ✅      |
| WASM     |      ✅      |      ✅       |     ✅      |     ✅      |
| Ruby     |      ✅      |      ✅       |     ✅      |      -      |
| R        |      ✅      |      ✅       |     ✅      |     ✅      |
| Elixir   |      ✅      |      ✅       |     ✅      |     ✅      |
| Go       |      ✅      |      ✅       |     ✅      |     ✅      |
| Java     |      ✅      |      ✅       |     ✅      |     ✅      |
| Kotlin   |      ✅      |      ✅       |     ✅      |     ✅      |
| C#       |      ✅      |      ✅       |     ✅      |     ✅      |
| PHP      |      ✅      |      ✅       |     ✅      |     ✅      |
| Swift    |      -       |       -       |     ✅      |      -      |
| Dart     |      ✅      |      ✅       |     ✅      |     ✅      |
| Zig      |      ✅      |      ✅       |     ✅      |     ✅      |
| Rust     |      ✅      |      ✅       |     ✅      |     ✅      |
| C (FFI)  |      ✅      |      ✅       |     ✅      |     ✅      |
| CLI      |      ✅      |      ✅       |     ✅      |     ✅      |
| Docker   |      ✅      |      ✅       |     ✅      |      -      |

**Note**: ✅ = Precompiled binaries available with instant installation. WASM runs in any environment with WebAssembly support (browsers, Deno, Bun, Cloudflare Workers). All platforms are tested in CI. MacOS support is Apple Silicon only.

### Mobile (iOS, Android)

| Target                                             | ORT-dependent features\* |
| -------------------------------------------------- | :----------------------: |
| iOS (`aarch64-apple-ios`, `aarch64-apple-ios-sim`) |            ❌            |
| Android arm64 (`aarch64-linux-android`)            |            ❌            |
| Android x86_64 emulator (`x86_64-linux-android`)   |            ❌            |

\*ORT-dependent features: PaddleOCR, layout detection, embeddings, reranking, auto-rotate, transcription.
All non-ORT capabilities (Tesseract OCR, document formats outside audio/video transcription, chunking, language detection, keywords, tree-sitter code intelligence, API/MCP, LLM) are available on all four mobile targets.

Published mobile bindings use the reduced `android-target` feature set on iOS and Android because ONNX Runtime and libheif do not cross-compile cleanly to SDK/NDK targets. Native Rust consumers can choose features manually for custom mobile builds, but the published mobile artifacts use this no-ORT bundle.

### Browsers / Edge (WebAssembly)

WASM excludes the same ORT-dependent feature set as the Android x86_64 emulator. The shared no-ORT base lives behind the `no-ort-target` feature in the core crate; both `wasm-target` and `android-target` compose it.

### Embeddings Support (Optional)

To use embeddings functionality:

1. **Install ONNX Runtime 1.24+**:
   - Linux: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases) (Debian packages may have older versions)
   - MacOS: `brew install onnxruntime`
   - Windows: Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)

2. Use embeddings in your code - see [Embeddings Guide](https://docs.kreuzberg.dev/features/#embeddings)

**Note:** Kreuzberg requires ONNX Runtime version 1.24+ for embeddings and other ORT-dependent inference features. Non-ORT Kreuzberg features work without ONNX Runtime.

## Supported Formats

96 file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

### Office Documents

| Category            | Formats                                                                                          | Capabilities                                       |
| ------------------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------- |
| **Word Processing** | `.docx`, `.docm`, `.doc`, `.dotx`, `.dotm`, `.dot`, `.odt`, `.pages`                              | Full text, tables, lists, images, metadata, styles |
| **Spreadsheets**    | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.xltx`, `.xlt`, `.ods`, `.numbers` | Sheet data, formulas, cell metadata, charts        |
| **Presentations**   | `.pptx`, `.pptm`, `.ppt`, `.ppsx`, `.potx`, `.potm`, `.pot`, `.key`                              | Slides, speaker notes, images, metadata            |
| **PDF**             | `.pdf`                                                                                           | Text, tables, images, metadata, OCR support        |
| **eBooks**          | `.epub`, `.fb2`                                                                                  | Chapters, metadata, embedded resources             |
| **Database**        | `.dbf`                                                                                           | Table data extraction, field type support          |
| **Hangul**          | `.hwp`, `.hwpx`                                                                                  | Korean document format, text extraction            |

### Images (OCR-Enabled)

| Category     | Formats                                                                          | Features                                                     |
| ------------ | -------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| **Raster**   | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif`                | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.jbig2`, `.jb2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | Pure Rust decoders (JPEG 2000, JBIG2), OCR, table detection  |
| **HEIC family** | `.heic`, `.heics`, `.heif`, `.avif`, `.avcs`                                  | EXIF metadata, optional libheif pixel decoding               |
| **Vector**   | `.svg`                                                                           | DOM parsing, embedded text, graphics metadata                |

### Audio & Video

| Category              | Formats                                   | Features                                               |
| --------------------- | ----------------------------------------- | ------------------------------------------------------ |
| **Audio**             | `.mp3`, `.mpga`, `.m4a`, `.wav`, `.webm`  | Whisper transcription when native transcription exists |
| **Video audio track** | `.mp4`, `.mpeg`, `.webm`                  | Audio-track transcription only                         |

### Web & Data

| Category            | Formats                                                             | Features                                                          |
| ------------------- | ------------------------------------------------------------------- | ----------------------------------------------------------------- |
| **Markup**          | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg`                           | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv`                   | Schema detection, nested structures, validation                   |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.djot`, `.mdx`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, Djot, MDX, reStructuredText, Org Mode, Rich Text |

### Email & Archives

| Category     | Formats                              | Features                                                |
| ------------ | ------------------------------------ | ------------------------------------------------------- |
| **Email**    | `.eml`, `.msg`, `.pst`                | Headers, body (HTML/plain), attachments, UTF-16 support |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | Recursive extraction, nested archives, metadata         |

### Academic & Scientific

| Category          | Formats                                               | Features                                                    |
| ----------------- | ----------------------------------------------------- | ----------------------------------------------------------- |
| **Citations**     | `.bib`, `.ris`, `.nbib`, `.enw`                                  | BibTeX/BibLaTeX, RIS, PubMed/MEDLINE, EndNote XML, CSL JSON by MIME type |
| **Scientific**    | `.tex`, `.latex`, `.typ`, `.typst`, `.jats`, `.ipynb`             | LaTeX, Typst, JATS journal articles, Jupyter notebooks      |
| **Publishing**    | `.fb2`, `.docbook`, `.dbk`, `.docbook4`, `.docbook5`, `.opml`     | FictionBook, DocBook XML, OPML outlines                     |
| **Documentation** | MIME-only POD, mdoc, troff                                        | Perl POD, man pages, troff                                  |

**[Complete Format Reference →](https://docs.kreuzberg.dev/reference/formats/)**

### Code Intelligence (306 Languages)

| Feature                    | Description                                                   |
| -------------------------- | ------------------------------------------------------------- |
| **Structure Extraction**   | Functions, classes, methods, structs, interfaces, enums       |
| **Import/Export Analysis** | Module dependencies, re-exports, wildcard imports             |
| **Symbol Extraction**      | Variables, constants, type aliases, properties                |
| **Docstring Parsing**      | Google, NumPy, Sphinx, JSDoc, RustDoc, and 10+ formats        |
| **Diagnostics**            | Parse errors with line/column positions                       |
| **Syntax-Aware Chunking**  | Split code by semantic boundaries, not arbitrary byte offsets |

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

Install the Kreuzberg plugin from the [`kreuzberg-dev/plugins`](https://github.com/kreuzberg-dev/plugins) marketplace:

```text
/plugin marketplace add kreuzberg-dev/plugins
/plugin install kreuzberg@kreuzberg
```

Works with Claude Code, Codex, Cursor, Gemini CLI, Factory Droid, GitHub Copilot CLI, and opencode. See [the marketplace README](https://github.com/kreuzberg-dev/plugins) for harness-specific install instructions.

The plugin ships the Kreuzberg agent skill — previously distributed from the in-tree `skills/` directory — and teaches assistants the extraction APIs, configuration fields, OCR backends, and language-specific conventions.

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

## Part of Kreuzberg.dev

- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces every per-language binding across the 5 polyglot repos.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

Elastic License 2.0 (ELv2) - see [LICENSE](LICENSE) for details. See [https://www.elastic.co/licensing/elastic-license](https://www.elastic.co/licensing/elastic-license) for the full license text.

## FAQ

### What is Kreuzberg?

Kreuzberg is a polyglot document intelligence framework with a Rust core. It extracts text, metadata, and code intelligence from 96 file formats and 306 programming languages at native speeds without needing a GPU. It provides native bindings for Rust, Python, TypeScript/Node.js, Ruby, Go, Java, Kotlin, C#, PHP, Elixir, R, Dart, Swift, Zig, and C.

### How does Kreuzberg differ from other document extraction tools?

- **Kreuzberg**: Rust core, 96 formats, 306 languages, polyglot bindings, code intelligence via tree-sitter, VLM OCR, native speeds, no GPU needed
- **Apache Tika**: Java-based, broader format support, but slower, no code intelligence, no VLM OCR
- **pdfplumber**: Python-only, PDF focus, slower, no code intelligence
- **unstructured**: Python-based, good format coverage, but slower, requires more dependencies

Kreuzberg's Rust core with SIMD optimizations and parallelism delivers 10-100x faster extraction than Python alternatives.

### What are Kreuzberg's key features?

- **Code intelligence** — Extract functions, classes, imports, symbols, docstrings from 306 languages via tree-sitter
- **Extensible architecture** — Plugin system for custom OCR backends, validators, post-processors, document extractors, renderers
- **Polyglot bindings** — Native bindings for 14+ languages (Rust, Python, Node.js, Ruby, Go, Java, Kotlin, C#, PHP, Elixir, R, Dart, Swift, Zig, C)
- **96 file formats** — PDF, Office documents, images, HTML, XML, emails, archives, academic formats across 8 categories
- **LLM intelligence** — VLM OCR (GPT-4o, Claude, Gemini, Ollama), structured JSON extraction, embeddings via 143 LLM providers
- **OCR support** — Tesseract (all bindings including WASM for browsers), PaddleOCR, EasyOCR, VLM OCR, extensible via plugin API
- **High performance** — Rust core with pure-Rust PDF, SIMD optimizations, full parallelism
- **Flexible deployment** — Library, CLI tool, REST API server, or MCP server
- **TOON wire format** — Token-efficient serialization for LLM/RAG pipelines, ~30-50% fewer tokens than JSON
- **GFM-quality output** — Comrak-based Markdown rendering with proper fenced code blocks, table nodes
- **Memory efficient** — Streaming parsers for multi-GB files

### What file formats does Kreuzberg support?

8 categories covering 96 formats:

- **Documents** — PDF, DOCX, DOC, ODT, RTF, Hangul
- **Office** — XLSX, XLS, PPTX, PPT, ODS, iWork
- **Images** — PNG, JPEG, TIFF, BMP, GIF, WebP
- **Web** — HTML, XML, XHTML, SVG
- **Emails** — MSG, EML, PST
- **Archives** — ZIP, TAR, GZ, TGZ, 7Z
- **Academic** — LaTeX, BibTeX, RIS
- **Code** — 306 programming languages via tree-sitter

### How do I get started?

Choose your platform:

**Python:**

```bash
pip install kreuzberg
```

See [Python docs](https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/python)

**Node.js:**

```bash
npm install @kreuzberg/node
```

See [Node.js docs](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg-node)

**Rust:**

```bash
cargo add kreuzberg
```

See [Rust docs](https://github.com/kreuzberg-dev/kreuzberg/tree/main/crates/kreuzberg)

**Docker:**

```bash
docker pull ghcr.io/kreuzberg-dev/kreuzberg:latest
```

See [Docker docs](https://docs.kreuzberg.dev/guides/docker/)

### What LLM/VLM providers are supported?

143 providers including:

- **OpenAI** — GPT-4o (vision), text models
- **Anthropic** — Claude (vision), Claude 3.5 Sonnet
- **Google** — Gemini (vision), Gemini 2.0 Flash
- **Local engines** — Ollama, LM Studio, vLLM, llama.cpp
- **Cloud providers** — Fireworks, Together, Groq, OpenRouter
- **All OpenAI-compatible endpoints**

### What OCR backends are available?

- **Tesseract** — All bindings, including Tesseract-WASM for browsers
- **PaddleOCR** — All native bindings (Python, Node.js, etc.)
- **EasyOCR** — Python binding
- **VLM OCR** — 143 vision model providers (GPT-4o, Claude, Gemini, Ollama local)
- **Custom OCR** — Extensible via plugin API

### What is the TOON wire format?

TOON is Kreuzberg's token-efficient serialization format for LLM/RAG pipelines. It uses ~30-50% fewer tokens than JSON, making it ideal for:

- Large document processing
- RAG system integration
- LLM context window optimization
- Cost reduction in API calls

### What is code intelligence extraction?

Kreuzberg extracts semantic code information via tree-sitter:

- **Functions** — Names, parameters, return types, docstrings
- **Classes** — Names, methods, inheritance, properties
- **Imports** — Module names, import paths
- **Symbols** — Variables, constants, type definitions
- **Docstrings** — Documentation comments

Results in `ExtractionResult.code_intelligence` with semantic chunking.

### Does Kreuzberg work in browsers?

Yes! The WASM package (`@kreuzberg/wasm`) supports browsers, Deno, and Cloudflare Workers with:

- PDF, Excel, archives, all office formats
- Real Tesseract OCR via WASI build
- Only ORT-dependent features excluded (PaddleOCR, layout detection, embeddings, reranking, auto-rotate, transcription)

### What deployment options are available?

- **Library** — Use as a dependency in your application
- **CLI** — Cross-platform binary for batch processing
- **REST API server** — HTTP endpoint for document extraction
- **MCP server** — Model Context Protocol server for AI assistants
- **Docker** — Official images with API, CLI, and MCP modes

### What languages have native bindings?

| Language | Package Manager | Status |
|----------|----------------|--------|
| Rust | Cargo | ✅ Core library |
| Python | PyPI | ✅ Full support |
| Node.js | npm (NAPI-RS) | ✅ Fastest performance |
| WASM | npm | ✅ Browser/Deno/CF Workers |
| Ruby | RubyGems | ✅ Native bindings |
| Go | Go modules | ✅ FFI bindings |
| Java | Maven Central | ✅ Foreign Function API |
| Kotlin | Maven Central | ✅ Coroutine-based |
| C# | NuGet | ✅ .NET 6.0+ |
| PHP | Composer | ✅ PHP 8.2+ |
| Elixir | Hex | ✅ OTP integration |
| R | r-universe | ✅ extendr bindings |
| Dart/Flutter | pub.dev | ✅ flutter_rust_bridge |
| Swift | SPM | ✅ macOS 13+/iOS 16+ |
| Zig | build.zig.zon | ✅ Idiomatic API |
| C (FFI) | pkg-config/CMake | ✅ Header + shared lib |

### What platforms are supported?

All bindings support:

- **Linux** — x86_64 and aarch64
- **macOS** — ARM64
- **Windows** — x64 (most bindings)

Precompiled binaries included for all architectures.

### What license does Kreuzberg use?

Elastic-2.0 License — open-source with commercial use restrictions. See [LICENSE](https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE) for details.

### Where can I get help?

- **Documentation**: [docs.kreuzberg.dev](https://docs.kreuzberg.dev)
- **Live Demo**: [docs.kreuzberg.dev/demo.html](https://docs.kreuzberg.dev/demo.html)
- **Discord**: [discord.gg/xt9WY3GnKR](https://discord.gg/xt9WY3GnKR)
- **Hugging Face**: [huggingface.co/Kreuzberg](https://huggingface.co/Kreuzberg)
- **GitHub Issues**: [github.com/kreuzberg-dev/kreuzberg/issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
