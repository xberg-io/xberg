# Extraction Basics

Kreuzberg provides 8 core extraction functions organized by input type (file path vs in-memory bytes), cardinality (single vs batch), and execution model (sync vs async). Pick the function that matches your situation — the extraction logic is identical across all variants.

| Input | Single sync | Single async | Batch sync | Batch async |
|-------|------------|-------------|------------|-------------|
| **File path** | `extract_file_sync` | `extract_file` | `batch_extract_files_sync` | `batch_extract_files` |
| **Bytes** | `extract_bytes_sync` | `extract_bytes` | `batch_extract_bytes_sync` | `batch_extract_bytes` |

!!! tip "Sync vs Async"
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

When the file is already loaded in memory (e.g., from an upload or network response), pass the byte array with its MIME type. Unlike file extraction, the MIME type is required since there's no file extension to infer it from.

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

When a batch contains a mix of document types that need different settings (e.g., scanned images needing OCR alongside text-based PDFs), use `FileExtractionConfig` to override options per file while sharing a common batch config.

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

See the [installation guide](../getting-started/installation.md#system-dependencies) for optional dependencies (Tesseract).

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

!!! warning "System Errors"
    `OSError` (Python), `IOException` (Rust), and system-level errors always propagate through. These indicate real system problems (permissions, disk space, etc.) that your application should handle.

## Next Steps

- [Configuration](configuration.md) — all configuration options and file formats
- [OCR Guide](ocr.md) — set up optical character recognition
- [Advanced Features](advanced.md) — chunking, language detection, embeddings
- [Element-Based Output](element-based-output.md) — structured element arrays for RAG
- [Document Structure](document-structure.md) — hierarchical tree output
