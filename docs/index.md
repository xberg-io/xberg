---
description: "Kreuzberg – Extract text, tables, and metadata from 91+ file formats with a Rust core and native bindings for 12 languages. No GPU required."
---

<div class="hero-banner" markdown>
![Kreuzberg](assets/Kreuzberg.svg)
</div>

# Kreuzberg

Kreuzberg is a document intelligence platform with a high‑performance Rust core and native bindings for Python, TypeScript/Node.js, C#, Ruby, Go, Elixir, and Rust itself. You can use it as an SDK, CLI, Docker image, REST API server, or MCP tool to extract text, tables, and metadata from 91+ file formats (PDF, Office, images, HTML, XML, archives, email, and more) with optional OCR and post-processing pipelines.

<div class="hero-badges" markdown>

[:material-play-circle: Live Demo](demo.html){ .md-button .md-button--primary }
[:material-lightning-bolt: Quick Start](getting-started/quickstart.md){ .md-button }
[:material-package-variant: Installation](getting-started/installation.md){ .md-button }
[:fontawesome-brands-discord: Join our Community](https://discord.gg/xt9WY3GnKR){ .md-button }

</div>

---

## Why Kreuzberg

<div class="grid cards" markdown>

-   :material-flash:{ .lg .middle } **High Performance**

    ---

    Rust core with native PDFium, SIMD optimizations, and full parallelism. Process thousands of documents per minute without a GPU.

-   :material-file-document-multiple:{ .lg .middle } **91+ File Formats**

    ---

    PDF, DOCX, XLSX, PPTX, images, HTML, XML, emails, archives, academic formats — one API handles them all.

-   :material-eye:{ .lg .middle } **Multi-Engine OCR**

    ---

    Tesseract and PaddleOCR work across all language bindings. EasyOCR is available for Python only.

-   :material-translate:{ .lg .middle } **12 Language Bindings**

    ---

    Native bindings for Python, TypeScript, Rust, Go, Java, C#, Ruby, PHP, Elixir, R, C, and WebAssembly.

-   :material-code-tags:{ .lg .middle } **Code Intelligence**

    ---

    Extract functions, classes, imports, symbols, and docstrings from 248 programming languages. Results in `code_intelligence` field with semantic chunking.

-   :material-puzzle:{ .lg .middle } **Plugin System**

    ---

    Register custom extractors, OCR backends, post-processors, and validators. Plugin authoring is primarily supported in Python; all bindings can consume registered plugins.

-   :material-server:{ .lg .middle } **Flexible Deployment**

    ---

    Use as a library, CLI tool, REST API server, MCP server, or Docker container. Pick what fits your stack.

</div>

→ **[See all features](features.md)**

---

## Language Support

Precompiled binaries for Linux (x86_64 & aarch64), macOS (Apple Silicon), and Windows (x64).

| Language | Package | Docs |
|:---------|:--------|:-----|
| **Python** | `pip install kreuzberg` | [API Reference](reference/api-python.md) |
| **TypeScript (Native)** | `npm install @kreuzberg/node` | [API Reference](reference/api-typescript.md) |
| **TypeScript (WASM)** | `npm install @kreuzberg/wasm` | [API Reference](reference/api-wasm.md) |
| **Rust** | `cargo add kreuzberg` | [API Reference](reference/api-rust.md) |
| **Go** | `go get .../kreuzberg/packages/go/v4` | [API Reference](reference/api-go.md) |
| **Java** | Maven Central `dev.kreuzberg:kreuzberg` | [API Reference](reference/api-java.md) |
| **C#** | `dotnet add package Kreuzberg` | [API Reference](reference/api-csharp.md) |
| **Ruby** | `gem install kreuzberg` | [API Reference](reference/api-ruby.md) |
| **PHP** | `composer require kreuzberg/kreuzberg` | [API Reference](reference/api-php.md) |
| **Elixir** | `{:kreuzberg, "~> 4.0"}` | [API Reference](reference/api-elixir.md) |
| **R** | r-universe `kreuzberg` | [API Reference](reference/api-r.md) |
| **C (FFI)** | Shared library + header | [API Reference](reference/api-c.md) |
| **CLI** | `brew install kreuzberg-dev/tap/kreuzberg` | [CLI Guide](cli/usage.md) |
| **Docker** | `ghcr.io/kreuzberg-dev/kreuzberg` | [Docker Guide](guides/docker.md) |

!!! tip "Choosing Between TypeScript Packages"

    **`@kreuzberg/node`** — Use for Node.js servers and CLI tools. Native performance (100% speed).

    **`@kreuzberg/wasm`** — Use for browsers, Cloudflare Workers, Deno, Bun, and serverless environments (60-80% speed, cross-platform).

---

## Explore the Docs

<div class="grid cards" markdown>

-   :material-rocket-launch:{ .lg .middle } **Getting Started**

    ---

    Install Kreuzberg and extract your first document in minutes.

    [:octicons-arrow-right-24: Quick Start](getting-started/quickstart.md)

-   :material-book-open-variant:{ .lg .middle } **Guides**

    ---

    Configuration, OCR setup, Docker deployment, plugins, and more.

    [:octicons-arrow-right-24: All Guides](guides/extraction.md)

-   :material-puzzle-outline:{ .lg .middle } **Concepts**

    ---

    Architecture, extraction pipeline, MIME detection, and performance.

    [:octicons-arrow-right-24: Architecture](concepts/architecture.md)

-   :material-api:{ .lg .middle } **API Reference**

    ---

    Complete API docs for every language binding, types, and errors.

    [:octicons-arrow-right-24: References](reference/api-python.md)

-   :material-console:{ .lg .middle } **CLI & Servers**

    ---

    Command-line tool, REST API server, and MCP server for AI agents.

    [:octicons-arrow-right-24: CLI Usage](cli/usage.md)

-   :material-swap-horizontal:{ .lg .middle } **Migration**

    ---

    Upgrade from v3 to v4, or migrate from Unstructured.

    [:octicons-arrow-right-24: Migration Guide](migration/v3-to-v4.md)

</div>

---

## Getting Help

- **Bugs & feature requests** — [Open an issue on GitHub](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Community chat** — [Join the Discord](https://discord.gg/xt9WY3GnKR)
- **Contributing** — [Read the contributor guide](contributing.md)
