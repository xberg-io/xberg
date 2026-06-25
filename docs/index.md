---
description: "Xberg – Extract text, tables, metadata, and audio transcripts from 96 file formats with a Rust core and polyglot SDKs. No GPU required."
---

# Xberg

Document intelligence with a Rust core and polyglot SDKs. Extract text, tables, metadata, and audio transcripts from 96 formats with optional OCR — usable as an SDK, CLI, REST API, MCP server, or Docker image.

<div class="hero-badges" markdown>

[:material-play-circle: Live Demo](demo.html){ .md-button .md-button--primary }
[:material-lightning-bolt: Quick Start](getting-started/quickstart.md){ .md-button }
[:material-package-variant: Installation](getting-started/installation.md){ .md-button }
[:fontawesome-brands-discord: Join our Community](https://discord.gg/xt9WY3GnKR){ .md-button }

</div>

---

## Why Xberg

<div class="grid cards" markdown>

- :material-flash:{ .lg .middle } **High Performance**

  Rust core with pdf_oxide PDF extraction, SIMD optimizations, and full parallelism. Process thousands of documents per minute without a GPU.

- :material-file-document-multiple:{ .lg .middle } **96 File Formats**

  PDF, DOCX, XLSX, PPTX, images, HTML, XML, emails, archives, academic formats, and audio/video transcription when enabled.

- :material-eye:{ .lg .middle } **Multi-Engine OCR**

  Tesseract works across native and Wasm targets. PaddleOCR is available on native ONNX Runtime builds; EasyOCR is Python-only.

- :material-translate:{ .lg .middle } **Polyglot SDKs**

  SDKs for Python, TypeScript, Rust, Go, Java, Kotlin Android, C#, Ruby, PHP, Elixir, R, Dart, Swift, Zig, C, and WebAssembly. Kotlin/JVM consumers use the Java artifact.

- :material-code-tags:{ .lg .middle } **Code Intelligence**

  Extract functions, classes, imports, symbols, and docstrings from 306 programming languages. Results in the **code_intelligence** field with semantic chunking.

- :material-puzzle:{ .lg .middle } **Plugin System**

  Register custom extractors, OCR backends, reranker backends, validators, post-processors, and renderers.

</div>

→ **[See all features](features.md)**

---

## Language Support

| Language                | Package                                        | Docs                                         |
| :---------------------- | :--------------------------------------------- | :------------------------------------------- |
| **Python**              | `pip install xberg`                        | [API Reference](reference/api-python.md)     |
| **TypeScript (Native)** | `npm install @xberg/node`                  | [API Reference](reference/api-typescript.md) |
| **TypeScript (WASM)**   | `npm install @xberg/wasm`                  | [API Reference](reference/api-wasm.md)       |
| **Rust**                | `cargo add xberg`                          | [API Reference](reference/api-rust.md)       |
| **Go**                  | `go get github.com/xberg-io/xberg` | [API Reference](reference/api-go.md)         |
| **Java / Kotlin JVM**   | Maven Central `dev.xberg:xberg`        | [API Reference](reference/api-java.md)       |
| **Kotlin Android**      | Maven Central `dev.xberg:xberg-android` | [API Reference](reference/api-kotlin-android.md) |
| **C#**                  | `dotnet add package Xberg`                 | [API Reference](reference/api-csharp.md)     |
| **Ruby**                | `gem install xberg`                        | [API Reference](reference/api-ruby.md)       |
| **PHP**                 | `composer require xberg-io/xberg`         | [API Reference](reference/api-php.md)        |
| **Elixir**              | `{:xberg, "~> 1.0"}`               | [API Reference](reference/api-elixir.md)     |
| **R**                   | r-universe `xberg`                         | [API Reference](reference/api-r.md)          |
| **Dart / Flutter**      | `dart pub add xberg`                       | [API Reference](reference/api-dart.md)       |
| **Swift**               | Swift Package Manager                          | [API Reference](reference/api-swift.md)      |
| **Zig**                 | `zig fetch --save` from GitHub                 | [API Reference](reference/api-zig.md)        |
| **C (FFI)**             | Shared library + header                        | [API Reference](reference/api-c.md)          |
| **CLI**                 | `brew install xberg-io/tap/xberg`     | [CLI Guide](cli/usage.md)                    |
| **Docker**              | `ghcr.io/xberg-io/xberg`              | [Docker Guide](guides/docker.md)             |

Homebrew 6.0+ requires explicit trust for third-party taps. Run `brew trust xberg-io/tap` once before installing the CLI from `xberg-io/tap`.

!!! Tip "Choosing Between TypeScript Packages"

    **`@xberg/node`** — Use for Node.js servers and CLI tools. Native performance (100% speed).

    **`@xberg/wasm`** — Use for browsers, Cloudflare Workers, Deno, Bun, and serverless environments (60-80% speed, cross-platform).

---

## Quick Example

=== "Python"

    --8<-- "snippets/python/api/extract_file_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_sync.md"

---

## Part of Xberg.io

Xberg is the document intelligence core in the [Xberg.io](https://xberg.io) ecosystem.

<div class="grid cards" markdown>

- :material-cloud:{ .lg .middle } **[Xberg Enterprise](https://enterprise.xberg.io)**

  Managed extraction API with SDKs, dashboards, and observability.

- :material-spider-web:{ .lg .middle } **[crawlberg](https://docs.crawlberg.xberg.io)**

  Web crawling and scraping with HTML to Markdown and headless-Chrome fallback.

- :material-language-html5:{ .lg .middle } **[html-to-markdown](https://docs.html-to-markdown.xberg.io)**

  Fast, lossless HTML to Markdown engine.

- :material-robot-outline:{ .lg .middle } **[liter-llm](https://docs.liter-llm.xberg.io)**

  Universal LLM API client with native bindings for 14 languages and 143 providers.

- :material-code-tags:{ .lg .middle } **[tree-sitter-language-pack](https://docs.tree-sitter-language-pack.xberg.io)**

  Tree-sitter grammars and code-intelligence primitives.

- :fontawesome-brands-discord:{ .lg .middle } **[Discord](https://discord.gg/xt9WY3GnKR)**

  Community chat for Xberg.io users and contributors.

</div>

---

## Explore the Docs

<div class="grid cards" markdown>

- :material-rocket-launch:{ .lg .middle } **Getting Started**

  Install Xberg and extract your first document in minutes.

  [:octicons-arrow-right-24: Quick Start](getting-started/quickstart.md)

- :material-book-open-variant:{ .lg .middle } **Guides**

  Configuration, OCR setup, Docker deployment, plugins, and more.

  [:octicons-arrow-right-24: All Guides](guides/extraction.md)

- :material-puzzle-outline:{ .lg .middle } **Concepts**

  Architecture, extraction pipeline, MIME detection, and performance.

  [:octicons-arrow-right-24: Architecture](concepts/architecture.md)

- :material-api:{ .lg .middle } **API Reference**

  Complete API docs for every language binding, types, and errors.

  [:octicons-arrow-right-24: References](reference/api-python.md)

- :material-console:{ .lg .middle } **CLI & Servers**

  Command-line tool, REST API server, and MCP server for AI agents.

  [:octicons-arrow-right-24: CLI Usage](cli/usage.md)

- :material-swap-horizontal:{ .lg .middle } **Migration**

  Migrate from Unstructured or other document extraction libraries, including the v5 image-index change.

  [:octicons-arrow-right-24: Migration Guides](migration/from-unstructured.md)
  [:octicons-arrow-right-24: v5 Image Indices](migration/v5.0-image-indices.md)

</div>

---

## Getting Help

- **Bugs & feature requests** — [Open an issue on GitHub](https://github.com/xberg-io/xberg/issues)
- **Community chat** — [Join the Discord](https://discord.gg/xt9WY3GnKR)
- **Contributing** — [Read the contributor guide](contributing.md)
