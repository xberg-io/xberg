# Kreuzberg vs Apache Tika

Apache Tika is the longest-running open-source document extraction tool in the ecosystem. It's been the default answer for enterprise document processing since 2007. Kreuzberg is a newer Rust-based alternative that takes a different approach to the same problem. Both are Apache-2.0 licensed.

## At a Glance

|                  | Kreuzberg                                                       | Apache Tika                                                          |
| ---------------- | --------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Written in**   | Rust                                                            | Java                                                                 |
| **File formats** | 90+ extracted                                                   | 1500+ detected, hundreds extracted                                   |
| **Use from**     | Python, TypeScript, Go, Ruby, Java, C#, PHP, Elixir, Rust, Wasm | Java, or any language via Tika Server (HTTP)                         |
| **Run it as**    | Library, CLI, self-hosted API, browser (Wasm)                   | Java library, Tika Server (HTTP), CLI                                |
| **License**      | Apache-2.0                                                      | Apache-2.0                                                           |
| **Sweet spot**   | High-throughput pipelines, polyglot native bindings, modern CLI | Enterprise document processing, metadata extraction, search indexing |

---

## How They Differ

### Architecture

Fundamentally different runtimes.

- **Kreuzberg** -- Rust library that compiles to a native binary or links directly into your application via language-specific bindings. No runtime required. A single `kreuzberg` binary gives you CLI, API server, and MCP server.
- **Tika** -- Java library that runs on the JVM. For non-Java languages, you deploy Tika Server (an HTTP service) and send documents over the network. JVM startup time and memory overhead are part of the deal.

If your stack is already JVM-based, Tika integrates naturally. For everything else, Kreuzberg avoids the overhead of running a separate Java service.

### Format Coverage

Both tools excel here, but in different ways.

- **Tika** -- Detects **1500+ MIME types** and extracts text from hundreds of formats. It's built to handle practically anything you throw at it, including exotic formats like CAD files and scientific data. Two decades of format support.
- **Kreuzberg** -- Extracts from **90+ formats** with a focus on high-quality output. Covers PDFs, Office docs, spreadsheets, HTML, images (via OCR), email, archives, source code, structured data, and niche markup formats like Typst and Djot.

Tika's format detection is unmatched in breadth. Kreuzberg focuses on extraction quality for the formats most document pipelines actually encounter.

### Language Integration

How each tool fits into your codebase.

- **Kreuzberg** -- Native bindings for **17 languages**. Each binding calls directly into the Rust core -- same performance, same features, no network round-trips. Also runs in the browser via Wasm.
- **Tika** -- Native only in **Java**. Every other language goes through Tika Server over HTTP. This adds latency, requires running a separate service, and means your application depends on the JVM being available.

### OCR

Different levels of OCR sophistication.

- **Kreuzberg** -- Tesseract + native PaddleOCR (ONNX-based, no Python needed). Multi-backend OCR pipeline with automatic quality-based fallback between engines. Image preprocessing built-in.
- **Tika** -- Delegates to Tesseract via its OCR parser. Functional but no multi-backend fallback or built-in quality scoring.

### Metadata

Both extract metadata, with different philosophies.

- **Kreuzberg** -- Format-specific discriminated unions. PDF metadata includes page count, version, encryption status, and permissions. Each format type has its own metadata shape.
- **Tika** -- Standardized metadata using Dublin Core, XMP, and other established schemas. Extremely rich metadata extraction, especially for media files. This is one of Tika's genuine strengths.

If metadata richness is your primary concern (especially for media, geospatial, or scientific formats), Tika is hard to beat.

### Ecosystem

Where each tool fits in the broader stack.

- **Tika** -- Deep integration with Apache Solr, Elasticsearch, Apache Nutch, and enterprise content management systems. If you're building a search infrastructure on the Apache stack, Tika is the natural choice.
- **Kreuzberg** -- Standalone library with built-in chunking, embeddings, and RAG pipeline support. Designed for modern AI/ML document pipelines rather than traditional search indexing.

---

## When to Use Kreuzberg

- You need **native bindings** in languages beyond Java (Python, TypeScript, Go, Ruby, etc.)
- You want a **single binary** or library -- no JVM, no separate server process
- Your pipeline needs **built-in chunking, embeddings, and RAG support**
- You need to run extraction in the **browser or on edge runtimes** via Wasm
- You want **multi-backend OCR** with automatic quality-based fallback

## When to Use Tika

- Your stack is **JVM-based** and you want native Java integration
- You need to detect or extract from **exotic formats** (CAD, geospatial, media containers)
- You're building on the **Apache search stack** (Solr, Nutch, Elasticsearch)
- **Metadata extraction** is your primary use case, especially across media and scientific formats
- You need the **longest track record** and widest enterprise adoption

---

!!! Info "Benchmarks"

    For extraction speed and quality comparisons between Kreuzberg and Apache Tika, see the [live benchmark dashboard](https://kreuzberg.dev/benchmarks).
