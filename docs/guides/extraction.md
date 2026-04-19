# Extraction Basics

Kreuzberg provides 8 core extraction functions organized by input type (file path vs in-memory bytes), cardinality (single vs batch), and execution model (sync vs async). Pick the function that matches your situation — the extraction logic is identical across all variants.

| Input | Single sync | Single async | Batch sync | Batch async |
|-------|------------|-------------|------------|-------------|
| **File path** | `extract_file_sync` | `extract_file` | `batch_extract_files_sync` | `batch_extract_files` |
| **Bytes** | `extract_bytes_sync` | `extract_bytes` | `batch_extract_bytes_sync` | `batch_extract_bytes` |

!!! Tip "Sync vs Async"
    Use async variants when you're already in an async context or processing multiple files concurrently. For scripts and simple pipelines, sync variants are simpler and just as fast for single files.

## Extract from Files

Pass a file path. Kreuzberg detects the MIME type from the extension and selects the right parser automatically.

### Synchronous

=== "Python"

    --8<-- "snippets/python/api/extract_file_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_sync.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_sync.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_sync.md"

=== "R"

    --8<-- "snippets/r/api/extract_file_sync.md"

=== "C"

    --8<-- "snippets/c/api/extract_file_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/api/extract_file_sync.md"

### Asynchronous

=== "Python"

    --8<-- "snippets/python/api/extract_file_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_async.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_async.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_async.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_async.md"

=== "R"

    --8<-- "snippets/r/api/extract_file_async.md"

=== "C"

    --8<-- "snippets/c/api/extract_file_async.md"

=== "WASM"

    --8<-- "snippets/wasm/api/extract_file_async.md"

## Extract from Bytes

When the file is already loaded in memory (for example, from an upload or network response), pass the byte array with its MIME type. Unlike file extraction, the MIME type is required since there's no file extension to infer it from.

### Synchronous

=== "Python"

    --8<-- "snippets/python/api/extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/api/extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/api/extract_bytes_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_bytes_sync.md"

=== "R"

    --8<-- "snippets/r/api/extract_bytes_sync.md"

=== "C"

    --8<-- "snippets/c/api/extract_bytes_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/api/extract_bytes_sync.md"

### Asynchronous

=== "Python"

    --8<-- "snippets/python/api/extract_bytes_async.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_bytes_async.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_bytes_async.md"

=== "Go"

    --8<-- "snippets/go/api/extract_bytes_async.md"

=== "Java"

    --8<-- "snippets/java/api/extract_bytes_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_bytes_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_bytes_async.md"

=== "R"

    --8<-- "snippets/r/api/extract_bytes_async.md"

=== "C"

    --8<-- "snippets/c/api/extract_bytes_async.md"

=== "WASM"

    --8<-- "snippets/wasm/api/extract_bytes_async.md"

## Batch Processing

Batch functions accept an array of file paths (or byte arrays) and process them concurrently. This is typically 2-5x faster than looping over single-file functions because Kreuzberg parallelizes internally.

### Batch Extract Files

=== "Python"

    --8<-- "snippets/python/api/batch_extract_files_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/batch_extract_files_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/batch_extract_files_sync.md"

=== "Go"

    --8<-- "snippets/go/api/batch_extract_files_sync.md"

=== "Java"

    --8<-- "snippets/java/api/batch_extract_files_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_files_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/batch_extract_files_sync.md"

=== "R"

    --8<-- "snippets/r/api/batch_extract_files_sync.md"

=== "C"

    --8<-- "snippets/c/api/batch_extract_files_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/api/batch_extract_files_sync.md"

### Batch Extract Bytes

=== "Python"

    --8<-- "snippets/python/api/batch_extract_bytes_sync.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/batch_extract_bytes_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/batch_extract_bytes_sync.md"

=== "Go"

    --8<-- "snippets/go/api/batch_extract_bytes_sync.md"

=== "Java"

    --8<-- "snippets/java/api/batch_extract_bytes_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_bytes_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/batch_extract_bytes_sync.md"

=== "R"

    --8<-- "snippets/r/api/batch_extract_bytes_sync.md"

=== "C"

    --8<-- "snippets/c/api/batch_extract_bytes_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/api/batch_extract_bytes_sync.md"

### Per-File Configuration <span class="version-badge">v4.5.0</span>

When a batch contains a mix of document types that need different settings (for example, scanned images needing OCR alongside text-based PDFs), use `FileExtractionConfig` to override options per file while sharing a common batch config.

=== "Python"

    ```python title="mixed_batch.py"
    from kreuzberg import (
        batch_extract_files_sync,
        ExtractionConfig,
        FileExtractionConfig,
        OcrConfig,
    )

    config = ExtractionConfig(output_format="markdown")

    paths = ["report.pdf", "scan.tiff", "notes.html"]
    file_configs = [
        None,
        FileExtractionConfig(
            force_ocr=True,
            ocr=OcrConfig(backend="tesseract", language="deu"),
        ),
        FileExtractionConfig(output_format="plain"),
    ]

    results = batch_extract_files_sync(paths, config, file_configs=file_configs)
    ```

=== "TypeScript"

    ```typescript title="mixed_batch.ts"
    import { batchExtractFilesSync } from '@kreuzberg/node';

    const results = batchExtractFilesSync(
      ['report.pdf', 'scan.tiff', 'notes.html'],
      { outputFormat: 'markdown' },
      [
        null,
        { forceOcr: true, ocr: { backend: 'tesseract', language: 'deu' } },
        { outputFormat: 'plain' },
      ],
    );
    ```

=== "Rust"

    ```rust title="mixed_batch.rs"
    use kreuzberg::{
        batch_extract_file, ExtractionConfig, FileExtractionConfig,
        OcrConfig, OutputFormat,
    };
    use std::path::PathBuf;

    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        ..Default::default()
    };

    let paths = vec![
        PathBuf::from("report.pdf"),
        PathBuf::from("scan.tiff"),
        PathBuf::from("notes.html"),
    ];

    let file_configs = vec![
        None,
        Some(FileExtractionConfig {
            force_ocr: Some(true),
            ocr: Some(OcrConfig {
                backend: "tesseract".to_string(),
                language: "deu".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        Some(FileExtractionConfig {
            output_format: Some(OutputFormat::Plain),
            ..Default::default()
        }),
    ];

    let results = batch_extract_file(paths, &config, Some(&file_configs)).await?;
    ```

Fields set to `None` in `FileExtractionConfig` inherit the batch default. Batch-level concerns like `max_concurrent_extractions`, `use_cache`, and `security_limits` cannot be overridden per file. See the [Configuration Reference](../reference/configuration.md#fileextractionconfig) for the full list of overridable fields.

## Content Filtering <span class="version-badge">v4.8.0</span>

Kreuzberg strips running headers, footers, watermarks, and cross-page repeating text by default so that downstream RAG and LLM pipelines see clean body content. `ContentFilterConfig` lets you opt back in to any of these when you need them, for example when extracting legal forms where the header carries the case number, or when running text analysis on a PDF whose brand name was being incorrectly removed by the repeating-text heuristic.

The defaults match the field defaults documented in [ContentFilterConfig](../reference/configuration.md#contentfilterconfig): `include_headers=False`, `include_footers=False`, `strip_repeating_text=True`, `include_watermarks=False`.

=== "Python"

    ```python title="keep_headers_footers.py"
    from kreuzberg import (
        extract_file_sync,
        ContentFilterConfig,
        ExtractionConfig,
    )

    # Legal/forms work: keep header and footer text
    config = ExtractionConfig(
        content_filter=ContentFilterConfig(
            include_headers=True,
            include_footers=True,
        ),
    )

    result = extract_file_sync("contract.pdf", config=config)
    ```

=== "TypeScript"

    ```typescript title="disable_repeating_text.ts"
    import { extract } from "@kreuzberg/node";

    // Disable cross-page deduplication so brand names aren't stripped
    const result = await extract("brochure.pdf", {
      contentFilter: {
        stripRepeatingText: false,
      },
    });
    ```

=== "Rust"

    ```rust title="content_filter.rs"
    use kreuzberg::{extract_file_sync, ContentFilterConfig, ExtractionConfig};

    let config = ExtractionConfig {
        content_filter: Some(ContentFilterConfig {
            include_headers: true,
            include_footers: true,
            strip_repeating_text: true,
            include_watermarks: false,
        }),
        ..Default::default()
    };

    let result = extract_file_sync("contract.pdf", None, &config)?;
    ```

When a layout-detection model is active, it can independently classify regions as page headers or footers and strip them per page. Setting `include_headers=True` / `include_footers=True` also disables that per-page stripping. See the [reference page](../reference/configuration.md#contentfilterconfig) for the full field semantics and per-format behavior.

## Supported Formats

Kreuzberg supports 75+ file formats across 8 categories:

| Category | Extensions | Notes |
|----------|-----------|-------|
| **PDF** | `.pdf` | Native text + OCR for scanned pages |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.webp` | Requires OCR backend |
| **Office** | `.docx`, `.pptx`, `.xlsx` | Modern formats via native parsers |
| **Legacy Office** | `.doc`, `.ppt` | Native OLE/CFB parsing |
| **Email** | `.eml`, `.msg` | Full support including attachments |
| **Web** | `.html`, `.htm` | Converted to Markdown with metadata |
| **Text** | `.md`, `.txt`, `.xml`, `.json`, `.yaml`, `.toml`, `.csv` | Direct extraction |
| **Archives** | `.zip`, `.tar`, `.tar.gz`, `.tar.bz2` | Recursive extraction |

## Page Tracking

Kreuzberg can track page boundaries and extract per-page content. Page tracking availability depends on the format:

- **PDF** — Full byte-accurate page tracking with O(1) lookup
- **PPTX** — Slide boundary tracking (each slide = one page)
- **DOCX** — Best-effort detection using explicit `<w:br type="page"/>` tags
- **Other formats** — No page tracking

Enable page extraction with `PageConfig`:

```python title="page_tracking.py"
config = ExtractionConfig(
    pages=PageConfig(
        insert_page_markers=True,
        marker_format="\n\n<!-- PAGE {page_num} -->\n\n"
    )
)
```

Page markers like `<!-- PAGE 1 -->` are inserted at boundaries in the `content` field — useful for LLMs that need to understand document layout. When both page tracking and chunking are enabled, chunks automatically include `first_page` and `last_page` metadata.

See [PageConfig Reference](../reference/configuration.md#pageconfig) for all options and [Advanced Page Tracking](./advanced.md#page-tracking-patterns) for chunk-to-page mapping examples.

## Code File Extraction

When extracting source code files (`.py`, `.rs`, `.ts`, `.go`, etc.), Kreuzberg uses tree-sitter to produce structured code intelligence. The result is available in `ExtractionResult.code_intelligence` as a `ProcessResult` containing:

- **Structure** -- Functions, classes, methods, interfaces, and their nesting hierarchy
- **Imports/Exports** -- Module dependencies and re-exports
- **Symbols** -- Variables, constants, type aliases
- **Docstrings** -- Parsed documentation in 10+ formats (Google, NumPy, JSDoc, RustDoc, etc.)
- **Diagnostics** -- Parse errors with line/column positions
- **Chunks** -- Semantic code chunks split at function/class boundaries

Code files bypass the text-splitter chunking pipeline entirely. Instead, TSLP's `CodeChunks` (function/class-aware) map directly to Kreuzberg `Chunk`s with semantic `chunk_type` and heading context.

Control the content mode with `TreeSitterProcessConfig.content_mode`:

- `chunks` (default) -- Semantic TSLP chunks as the content output
- `raw` -- Source code as-is, no transformation
- `structure` -- Headings and docstrings only

## PDF Page Rendering

Render individual PDF pages as PNG images. Unlike the extraction pipeline (which parses text, tables, metadata), this API produces raw pixel data for thumbnails, vision model input, or custom OCR pipelines.

### Two Approaches

| API | When to use |
|-----|-------------|
| `render_pdf_page` | You know which page you need, or only need a few pages |
| `PdfPageIterator` | Process every page sequentially without loading all images into memory |

### DPI Configuration

| DPI | Pixel size (US Letter) | Use case |
|-----|----------------------|----------|
| 72 | 612 x 792 | Thumbnails, quick previews |
| 150 (default) | 1275 x 1650 | General-purpose, screen display |
| 300 | 2550 x 3300 | OCR input, print quality |

**Tip:** Use 300 DPI when rendering pages for OCR or vision models. The default 150 DPI may reduce recognition accuracy on small text.

## MIME Type Detection

When extracting from bytes, Kreuzberg requires an explicit MIME type since there's no file extension to infer it from. For file paths, auto-detection from the extension is automatic.

### Example: Override MIME Type

```python title="Python"
from kreuzberg import extract_file

# File without extension — provide MIME type explicitly
result = extract_file("document_copy", mime_type="application/pdf", config=config)
```

## Error Handling

All extraction functions raise typed exceptions on failure. Catch specific exceptions to handle different failure modes:

=== "Python"

    --8<-- "snippets/python/utils/error_handling.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling.md"

=== "R"

    --8<-- "snippets/r/api/error_handling.md"

=== "C"

    --8<-- "snippets/c/api/error_handling.md"

=== "WASM"

    --8<-- "snippets/wasm/api/error_handling_wasm.md"

!!! Warning "System Errors"
    `OSError` (Python), `IOException` (Rust), and system-level errors always propagate through. These indicate real system problems (permissions, disk space, etc.) that your application should handle.

## Next Steps

- [Configuration](configuration.md) — all configuration options and file formats
- [OCR Guide](ocr.md) — set up optical character recognition
- [Advanced Features](advanced.md) — chunking, language detection, embeddings
- [Element-Based Output](output-formats.md#element-based-output) — structured element arrays for RAG
- [Document Structure](output-formats.md#document-structure) — hierarchical tree output
