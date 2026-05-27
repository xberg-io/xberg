# Kreuzberg vs Docling

Kreuzberg and Docling are both open-source document extraction libraries, but they come at the problem from different angles. Kreuzberg is a Rust library focused on speed and broad format coverage across many languages. Docling is an IBM-backed Python library that leans heavily on deep learning models for document understanding. Here's how they compare.

## At a Glance

|                  | Kreuzberg                                                         | Docling                                              |
| ---------------- | ----------------------------------------------------------------- | ---------------------------------------------------- |
| **Written in**   | Rust                                                              | Python                                               |
| **File formats** | 90+                                                               | ~38 extensions (15+ types)                           |
| **Use from**     | Python, TypeScript, Go, Ruby, Java, C#, PHP, Elixir, Rust, Wasm   | Python                                               |
| **License**      | Apache-2.0                                                        | Elastic-2.0                                          |
| **OCR**          | Tesseract + PaddleOCR (local, multi-backend fallback)             | Tesseract + EasyOCR                                  |
| **Sweet spot**   | High-throughput pipelines, polyglot stacks, broad format coverage | ML-powered document understanding, scientific papers |

---

## How They Differ

### Architecture

Different foundations lead to different trade-offs.

- **Kreuzberg** -- Rust core with native bindings for each language. Your Python or TypeScript code calls directly into compiled Rust. No subprocess overhead, no model loading delays for basic extraction.
- **Docling** -- Python library built around deep learning models (DocLayNet for layout, TableFormer for tables). It produces a rich `DoclingDocument` object with full structural understanding, but it needs to load ML models on startup.

If you need raw extraction speed without ML overhead, Kreuzberg is faster out of the box. If you need deep structural understanding of complex layouts, Docling's ML pipeline is purpose-built for that.

### Format Coverage

What each tool can ingest.

- **Kreuzberg (90+ formats)** -- PDFs, Office docs, spreadsheets, HTML, images (via OCR), email, archives, source code, structured data (JSON/YAML/TOML), plus LaTeX, Typst, BibTeX, Jupyter notebooks, EPUB, OrgMode, and more.
- **Docling (~38 extensions)** -- PDFs, DOCX, PPTX, XLSX, HTML, Markdown, AsciiDoc, CSV, images, and JATS (scientific article XML). Focused on the formats that benefit most from layout analysis.

Docling covers the core document types well. Kreuzberg handles the long tail -- archives, email files, structured data, code, and niche markup formats.

### Output Model

How extracted content is structured.

- **Kreuzberg** -- Outputs unified text (default), element-based structures, or per-page JSON. You choose the level of detail you need. Markdown output is built-in via HTML-to-Markdown conversion.
- **Docling** -- Outputs a `DoclingDocument` object with rich structural metadata: reading order, table cells, figure captions, section hierarchy. Can export to Markdown, JSON, or other formats. The structural model is deeper but Python-specific.

### OCR

Both handle image-based documents, with different engine choices.

- **Kreuzberg** -- Tesseract + native PaddleOCR (ONNX-based, no Python dependency). Supports a multi-backend OCR pipeline that auto-falls back between engines based on output quality.
- **Docling** -- Tesseract + EasyOCR. EasyOCR offers good accuracy on CJK and Arabic scripts but requires PyTorch.

### Language Support

How you integrate each tool into your stack.

- **Kreuzberg** -- Native bindings for **16 languages** (Python, TypeScript, Rust, Go, Java, Kotlin, C#, Ruby, PHP, Elixir, R, Dart, Swift, Zig, C, Wasm). Same performance and features from every language.
- **Docling** -- **Python only**. If your backend is in Go, Java, or TypeScript, you'd need to wrap Docling in an HTTP service.

---

## When to Use Kreuzberg

- You're building a pipeline in **Go, TypeScript, Ruby, Java**, or any language beyond Python
- You need to process **high volumes** quickly without ML model loading overhead
- Your pipeline ingests **diverse formats** beyond PDFs and Office docs
- You want **local embeddings and chunking** built into the extraction step
- You need to run in the **browser or on edge runtimes** via Wasm

## When to Use Docling

- You need **deep structural understanding** of complex document layouts (reading order, nested tables, figure captions)
- You're working with **scientific papers or technical documents** where layout analysis matters
- Your stack is **Python-only** and you want a rich document object model
- You need **TableFormer-based table extraction** for complex tables with merged cells and spanning rows
- You value IBM's **ongoing investment** in document AI research

---

!!! Info "Benchmarks"

    For extraction speed and quality comparisons between Kreuzberg and Docling, see the [live benchmark dashboard](https://kreuzberg.dev/benchmarks).
