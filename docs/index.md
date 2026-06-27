---
description: "Xberg – Full content intelligence engine: extract text, tables, entities, embeddings from 96+ formats with OCR, transcription, code intelligence, and LLM integration. Native bindings for 16 languages."
---

# Xberg

Full content intelligence from one engine. Turn documents, URLs, code, and media into clean, structured output. Extract text, tables, entities, code structure, and embeddings—with automatic OCR, transcription, enrichment, and LLM-powered extraction. Available natively in 16 languages.

<div class="hero-badges" markdown>

[:material-play-circle: Live Demo](demo.html){ .md-button .md-button--primary }
[:material-lightning-bolt: Quick Start](getting-started/quickstart.md){ .md-button }
[:material-package-variant: Installation](getting-started/installation.md){ .md-button }
[:fontawesome-brands-discord: Join our Community](https://discord.gg/xt9WY3GnKR){ .md-button }

</div>

> Xberg is the next iteration of [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg-v4-lts) — the same document-intelligence engine, rebuilt and rebranded under a fresh v1 line.

---

## Why Xberg

<div class="grid cards" markdown>

- :material-layers-multiple:{ .lg .middle } **One Engine for Everything**

  Feed documents, URLs, code, images, and audio into one API. Get clean Markdown, entities, code structure, embeddings, and enrichment without stitching libraries together.

- :material-file-document-multiple:{ .lg .middle } **96+ Document Formats**

  PDFs, Office documents, images, email, archives, academic publications, and more—all standardize to clean Markdown plus 5 other output formats.

- :material-spider-web:{ .lg .middle } **Crawl and Extract at Scale**

  Recursively fetch URLs, extract documents from nested archives, and process hundreds in parallel. Configurable depth, timeouts, and headless-Chrome fallback.

- :material-robot-outline:{ .lg .middle } **Structured LLM Extraction**

  Extract entities, JSON schemas, and structured data using local LLM servers (Ollama, LM Studio, vLLM) or 143+ remote providers—no custom prompting required.

- :material-eye:{ .lg .middle } **OCR and Transcription**

  Tesseract, PaddleOCR, and VLM models for scans. Whisper for audio and video. Confidence scores, language detection, and automatic fallback chains.

- :material-code-tags:{ .lg .middle } **Code Intelligence**

  Extract functions, classes, imports, and symbols from 306 programming languages. Automatic semantic chunking for RAG pipelines.

- :material-vector-polyline:{ .lg .middle } **Embeddings and Enrichment**

  Vectorize with local ONNX models or provider-hosted embeddings. Enrich with named entity recognition, keyword extraction (YAKE/RAKE), image captions, and token reduction.

- :material-translate:{ .lg .middle } **16-Language Native SDKs**

  Python, TypeScript, Rust, Go, Java, C#, Ruby, PHP, Elixir, R, Dart, Swift, Zig, C FFI—plus CLI, REST API, MCP server, Docker, and WASM.

</div>

→ **[See all features](features.md)**

---

## Language Support

| Language                | Package                                        | Docs                                         |
| :---------------------- | :--------------------------------------------- | :------------------------------------------- |
| **Python**              | `pip install xberg`                        | [API Reference](reference/api-python.md)     |
| **TypeScript (Native)** | `npm install @xberg-io/xberg`                  | [API Reference](reference/api-typescript.md) |
| **TypeScript (WASM)**   | `npm install @xberg-io/xberg-wasm`                  | [API Reference](reference/api-wasm.md)       |
| **Rust**                | `cargo add xberg`                          | [API Reference](reference/api-rust.md)       |
| **Go**                  | `go get github.com/xberg-io/xberg` | [API Reference](reference/api-go.md)         |
| **Java / Kotlin JVM**   | Maven Central `io.xberg:xberg`        | [API Reference](reference/api-java.md)       |
| **Kotlin Android**      | Maven Central `io.xberg:xberg-android` | [API Reference](reference/api-kotlin-android.md) |
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

    **`@xberg-io/xberg`** — Use for Node.js servers and CLI tools. Native performance (100% speed).

    **`@xberg-io/xberg-wasm`** — Use for browsers, Cloudflare Workers, Deno, Bun, and serverless environments (60-80% speed, cross-platform).

---

## Quick Example

=== "Python"

    ```python
    from xberg import ExtractInput, extract

    output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
    print(output.results[0].content)
    ```

=== "TypeScript"

    ```typescript
    import { ExtractInputKind, extract } from "@xberg-io/xberg";

    const output = await extract({
      kind: ExtractInputKind.Uri,
      uri: "document.pdf",
    });
    console.log(output.results[0].content);
    ```

=== "Rust"

    ```rust
    use xberg::{extract, ExtractInput, ExtractionConfig};

    let config = ExtractionConfig::default();
    let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
    println!("{}", output.results[0].content);
    ```

---

## Part of Xberg.io

Xberg is the document intelligence core in the [Xberg.io](https://xberg.io) ecosystem.

<div class="grid cards" markdown>

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

  Migrate from Unstructured or other document extraction libraries.

  [:octicons-arrow-right-24: Migration Guides](migration/from-unstructured.md)

</div>

---

## Getting Help

- **Bugs & feature requests** — [Open an issue on GitHub](https://github.com/xberg-io/xberg/issues)
- **Community chat** — [Join the Discord](https://discord.gg/xt9WY3GnKR)
- **Contributing** — [Read the contributor guide](contributing.md)
