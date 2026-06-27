# {{ name }}

{% include 'partials/badges.html.jinja' %}

{{ description }}

## What This Package Provides

- **Document intelligence core** — extract text, tables, images, metadata, entities, keywords, code intelligence, and transcripts in builds that enable transcription.
- **Format coverage** — PDF, Office, images, HTML/XML, email, archives, notebooks, citations, scientific formats, plain text, and audio/video formats in builds that enable transcription.
- **OCR choices** — Tesseract, PaddleOCR, EasyOCR where supported, VLM OCR through liter-llm, and plugin hooks for custom backends.
- **Same engine as every binding** — Rust, Python, Node.js, Go, Java, PHP, Ruby, .NET, Elixir, R, WASM, Kotlin Android, Swift, Dart, Zig, and C FFI share the same Rust implementation.
{% if language == "typescript" %}
- **Node-first TypeScript API** — NAPI-RS package with typed options/results and async extraction.
{% elif language == "python" %}
- **Python package** — sync and async APIs with typed results for ingestion, RAG, and data workflows.
{% elif language == "go" %}
- **Go module** — context-aware API over the shared native library.
{% elif language == "java" %}
- **Java package** — FFM binding for direct native document extraction.
{% elif language == "php" %}
- **PHP package** — PHP 8.2+ API with generated types.
{% elif language == "ruby" %}
- **Ruby package** — native extension with idiomatic Ruby objects.
{% elif language == "csharp" %}
- **.NET package** — async/await API with nullable-aware result types.
{% elif language == "elixir" %}
- **BEAM package** — Rustler NIF binding for OTP pipelines.
{% elif language == "wasm" %}
- **WASM package** — browser and edge-compatible extraction where native libraries are unavailable.
{% elif language == "r" %}
- **R package** — data workflow binding with data-frame-friendly extracted structures.
{% elif language == "ffi" %}
- **C ABI** — stable shared library surface for custom hosts and secondary bindings.
{% elif language == "kotlin_android" %}
- **Android AAR** — JNI-backed package for mobile extraction workloads.
{% elif language == "swift" %}
- **SwiftPM package** — Swift Concurrency API for Apple targets.
{% elif language == "dart" %}
- **Dart package** — Future/Stream API through flutter_rust_bridge.
{% elif language == "zig" %}
- **Zig package** — wrapper over the C FFI with explicit memory ownership.
{% endif %}

## Installation

{% include 'partials/installation.md.jinja' %}

## Quick Start

{% include 'partials/quick_start.md.jinja' %}

{% if language == "typescript" %}
{% include 'partials/napi_implementation.md.jinja' %}

{% endif %}

## Features

{% include 'partials/features.md.jinja' %}

{% if features.ocr %}

## OCR Support

Xberg supports multiple OCR backends for extracting text from scanned documents and images:

{% for backend in ocr_backends %}

- **{{ backend | title }}**
  {% endfor %}

### OCR Configuration Example

{{ snippets.ocr_configuration | include_snippet(language) }}

{% endif %}
{% if features.async %}

## Async Support

This binding provides full async/await support for non-blocking document processing:

{{ snippets.async_extraction | include_snippet(language) }}

{% endif %}
{% if features.plugin_system %}

## Plugin System

Xberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://docs.xberg.io/guides/plugins/).

{% if snippets.plugin_system %}

### Plugin Example

{{ snippets.plugin_system | include_snippet(language) }}

{% endif %}
{% endif %}
{% if features.embeddings %}

## Embeddings Support

Generate vector embeddings for extracted text using the built-in ONNX Runtime support. Requires ONNX Runtime installation.

**[Embeddings Guide](https://docs.xberg.io/features/#embeddings)**
{% endif %}

{% if snippets.batch_processing %}

## Batch Processing

Process multiple documents efficiently:

{{ snippets.batch_processing | include_snippet(language) }}

{% endif %}

## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://docs.xberg.io/guides/configuration/)**

## Documentation

- **[Official Documentation](https://docs.xberg.io/)**
- **[API Reference](https://docs.xberg.io/reference/api-python/)**
- **[Examples & Guides](https://docs.xberg.io/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/xberg-io/xberg/blob/main/CONTRIBUTING.md).

## Part of Xberg.dev

- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

{{ license }} License — see [LICENSE](../../LICENSE) for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/xberg-io/xberg/issues)
- **Discussions**: [Ask questions](https://github.com/xberg-io/xberg/discussions)
