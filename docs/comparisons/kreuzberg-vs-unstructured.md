# Kreuzberg vs Unstructured

Both Kreuzberg and Unstructured are open-source tools for extracting text, tables, and metadata from documents. They solve similar problems but make very different architectural choices. This doc breaks down where each one shines so you can pick the right tool for your project.

## At a Glance

| | Kreuzberg | Unstructured |
|---|---|---|
| **Written in** | Rust | Python |
| **File formats** | 88+ | ~30 |
| **Use from** | Python, TypeScript, Go, Ruby, Java, C#, PHP, Elixir, Rust, WASM | Python, or any language via REST API |
| **Run it as** | Library, CLI, self-hosted API, browser (WASM) | Python library, managed cloud API, self-hosted API |
| **Pricing** | Free, Apache 2.0 | Free (open-source) + paid managed API |
| **Sweet spot** | High-throughput pipelines, polyglot stacks, on-prem | Managed service, ML-heavy layout analysis, quick prototyping |

---

## How They Differ

### Architecture and Performance

The core difference is what's under the hood.

- **Kreuzberg** -- Rust library with native bindings for each language. Your Python, TypeScript, or Go code calls directly into compiled Rust with no subprocess spawning or HTTP overhead.
- **Unstructured** -- Python library with an optional managed cloud API. Well-optimized for Python workflows, but other languages go through HTTP.

Bottom line: if you're processing thousands of documents in a pipeline, the Rust core gives Kreuzberg a throughput advantage. If you're in a Python-only stack, Unstructured is a natural fit.

### Format Coverage

How much of your document zoo each tool can handle.

- **Kreuzberg (88+ formats)** -- PDFs, Office docs, spreadsheets, HTML, images (via OCR), email, archives, source code, structured data (JSON/YAML/TOML), plus niche formats like LaTeX, Typst, BibTeX, Jupyter notebooks, EPUB, and OrgMode.
- **Unstructured (~30 formats)** -- PDFs, Office files, HTML, images, email, and the most common document types. Covers the essentials well.

If your pipeline only deals with PDFs and Word docs, both work. If you need to ingest Jupyter notebooks or OrgMode files, Kreuzberg has you covered.

### Language Support and Deployment

How you integrate each tool into your stack.

- **Kreuzberg** -- Native bindings for **10 languages** (Python, TypeScript, Go, Ruby, Java, C#, PHP, Elixir, Rust, WASM). Each binding calls directly into the Rust core -- same performance, same features. Also runs in the browser via WASM.
- **Unstructured** -- Python-first. Other languages go through a **REST API**, either self-hosted or via their managed cloud. The managed API is a genuine advantage if you don't want to run infrastructure.

### OCR and Layout Analysis

Both tools handle OCR and document layout, but with different approaches.

- **OCR** -- Both integrate Tesseract. Kreuzberg adds a native **PaddleOCR** backend (ONNX-based, no Python needed) and a **multi-backend pipeline** that auto-falls back between engines based on output quality.
- **Layout detection** -- Kreuzberg ships ONNX-based models with two presets: YOLO (fast) and RT-DETR v2 (accurate), covering 17 element classes plus SLANet table structure recognition. Unstructured offers mature ML-based layout detection with strong complex table support.

### Embeddings and Chunking

How each tool prepares extracted text for RAG pipelines.

- **Kreuzberg** -- Generates embeddings **locally** with ONNX models (no API keys needed). Supports recursive, semantic, and markdown-aware chunking with optional token-based sizing via HuggingFace tokenizers.
- **Unstructured** -- Uses **external APIs** (OpenAI, Cohere, etc.) for embeddings. Offers its own chunking strategies including a `by_title` chunker that respects document structure. Integrates cleanly if you're already paying for an embedding API.

### Privacy and Cost

Where your documents go and what you pay.

- **Kreuzberg** -- Fully self-hosted. Documents never leave your infrastructure. No API fees -- you pay only for compute.
- **Unstructured** -- Self-host for free, or use their **managed API** (free tier: 1,000 pages/month, paid plans beyond). The managed option trades cost for convenience -- no servers to maintain, no OCR dependencies to install.

---

## When to Use Kreuzberg

- You're processing **high volumes** of documents and need throughput
- Your stack isn't Python-only -- you need native support in **Go, TypeScript, Ruby, Java**, or other languages
- You need to keep documents **on-prem** for privacy, compliance, or air-gapped environments
- You want **local embeddings** without external API dependencies
- You need to handle **uncommon formats** like LaTeX, Typst, Jupyter notebooks, or archives

## When to Use Unstructured

- You want a **managed cloud service** so you don't run any infrastructure
- You're in a **Python-only** environment and want the simplest setup
- You need **mature ML models** for complex table extraction and layout analysis
- You're prototyping and want to **get started quickly** with their hosted API
- You're already using **OpenAI or Cohere** for embeddings and want a unified pipeline

---

!!! tip "Switching over?"

    If you're currently using Unstructured and want to try Kreuzberg, check out the [Migration Guide](../migration/from-unstructured.md) for a step-by-step walkthrough.
