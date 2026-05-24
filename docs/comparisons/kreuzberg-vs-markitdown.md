# Kreuzberg vs MarkItDown

MarkItDown is a Microsoft-backed Python library that converts documents to Markdown -- purpose-built for feeding content into LLMs. Kreuzberg is a Rust-based extraction library with broader format support, native language bindings, and built-in RAG pipeline features. Both are permissively licensed and work well for AI-adjacent document processing.

## At a Glance

|                  | Kreuzberg                                                       | MarkItDown                                |
| ---------------- | --------------------------------------------------------------- | ----------------------------------------- |
| **Written in**   | Rust                                                            | Python                                    |
| **File formats** | 90+                                                             | ~25                                       |
| **Use from**     | Python, TypeScript, Go, Ruby, Java, C#, PHP, Elixir, Rust, Wasm | Python                                    |
| **Output**       | Unified text, element-based, per-page JSON, Markdown            | Markdown                                  |
| **License**      | Apache-2.0                                                      | Elastic-2.0                               |
| **Sweet spot**   | Full extraction pipelines with chunking and embeddings          | Quick Markdown conversion for LLM context |

---

## How They Differ

### Philosophy

Different tools for different stages of a pipeline.

- **Kreuzberg** -- A full extraction library. Extracts text, tables, metadata, and images. Offers multiple output formats, built-in chunking, local embeddings, and OCR. Designed to be the complete document-to-vectors pipeline.
- **MarkItDown** -- A converter. Takes documents in, outputs Markdown. Intentionally lightweight and focused on one job: turning files into clean Markdown that LLMs can consume. Downstream processing is left to you.

If you need a complete pipeline (extract, chunk, embed), Kreuzberg handles the full chain. If you just need Markdown for a prompt, MarkItDown does that with minimal setup.

### Format Coverage

Both cover common formats, with different long-tail reach.

- **Kreuzberg (90+ formats)** -- PDFs, Office docs, spreadsheets, HTML, images (via OCR), email, archives, source code, structured data (JSON/YAML/TOML), plus LaTeX, Typst, BibTeX, Jupyter notebooks, EPUB, OrgMode, and more.
- **MarkItDown (~25 formats)** -- PDFs, DOCX, PPTX, XLSX, HTML, XML, CSV, JSON, EPUB, Jupyter notebooks, MSG email, images, and ZIP archives. Covers the essentials.

MarkItDown handles the formats you'll encounter most often. Kreuzberg also handles the ones you won't expect -- until you do.

### OCR

Different approaches to image-based text extraction.

- **Kreuzberg** -- Tesseract + native PaddleOCR (ONNX-based, runs locally, no Python needed). Multi-backend pipeline with automatic quality-based fallback. All processing happens on your machine.
- **MarkItDown** -- Can use **Azure Document Intelligence** for image and PDF extraction. Powerful when enabled, but requires an Azure account and sends documents to Microsoft's cloud. Without it, image OCR is limited.

### Language Support

A significant difference in how you integrate each tool.

- **Kreuzberg** -- Native bindings for **16 languages**. Same performance and API from Python, TypeScript, Rust, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, R, Dart, Swift, Zig, C, or Wasm in the browser.
- **MarkItDown** -- **Python only**. If your backend is in Go or TypeScript, you'd need to wrap MarkItDown in an HTTP service or call it as a subprocess.

### Downstream Processing

What happens after extraction.

- **Kreuzberg** -- Built-in **chunking** (recursive, semantic, markdown-aware), **local embeddings** (ONNX models, no API keys), token reduction, keyword extraction, and quality processing. Extraction output is ready for RAG pipelines.
- **MarkItDown** -- Outputs Markdown and stops. Chunking, embeddings, and vector storage are your responsibility. This is by design -- it's a converter, not a pipeline.

---

## When to Use Kreuzberg

- You need a **complete pipeline** from document to embeddings
- Your stack includes **Go, TypeScript, Ruby, Java**, or other languages beyond Python
- You want **local OCR** without cloud API dependencies
- You need to handle **niche formats** like LaTeX, Typst, email files, or archives
- You need **multiple output formats** (text, elements, per-page JSON) not just Markdown

## When to Use MarkItDown

- You just need **clean Markdown** to feed into an LLM prompt
- You're in a **Python-only** environment and want the simplest possible setup
- You're already using **Azure Document Intelligence** and want to leverage it for OCR
- Your use case is **document-to-prompt conversion** without further processing
- You value **minimal dependencies** and a small footprint

---

!!! Info "Benchmarks"

    For extraction speed and quality comparisons between Kreuzberg and MarkItDown, see the [live benchmark dashboard](https://kreuzberg.dev/benchmarks).
