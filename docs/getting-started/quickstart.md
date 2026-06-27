# Quick Start

This guide walks you through Xberg's core API: `extract`, `extract_batch`,
`ExtractInput`, and the `ExtractionOutput` envelope. Install your binding first if you haven't:
[Installation](installation.md).

TypeScript users: `@xberg-io/xberg` for Node.js, `@xberg-io/xberg-wasm` for browsers and edge runtimes — see [Language Support](../index.md#language-support).

## Your First Extraction

Pass an `ExtractInput` with `kind = "uri"` to extract a local path, `file://` URI,
or HTTP(S) URL. `extract` returns an `ExtractionOutput` with a `results` list:

=== "C"

    --8<-- "snippets/c/api/extract.md"

=== "C#"

    --8<-- "snippets/csharp/extract.md"

=== "Dart"

    --8<-- "snippets/dart/api/extract.md"

=== "Go"

    --8<-- "snippets/go/api/extract.md"

=== "Java"

    --8<-- "snippets/java/api/extract.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/api/extract.md"

=== "Python"

    --8<-- "snippets/python/api/extract.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract.md"

=== "R"

    --8<-- "snippets/r/api/extract.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract.md"

=== "Swift"

    --8<-- "snippets/swift/api/extract.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/extract.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract.md"

=== "Wasm"

    --8<-- "snippets/wasm/getting-started/extract.md"

=== "Zig"

    --8<-- "snippets/zig/api/extract.md"

=== "CLI"

    --8<-- "snippets/cli/extract_basic.md"

## Handle Errors

Wrap extractions in error handling before going further. Xberg raises specific
exceptions for missing files, parse failures, and OCR problems:

=== "C"

    --8<-- "snippets/c/api/error_handling.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Dart"

    --8<-- "snippets/dart/api/error_handling.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/api/error_handling.md"

=== "Python"

    --8<-- "snippets/python/utils/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling.md"

=== "R"

    --8<-- "snippets/r/api/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling.md"

=== "Swift"

    --8<-- "snippets/swift/api/error_handling.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/error_handling.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling.md"

=== "Wasm"

    --8<-- "snippets/wasm/api/error_handling.md"

=== "Zig"

    --8<-- "snippets/zig/api/error_handling.md"

## OCR for Scanned Documents

Xberg runs OCR automatically when it detects an image or scanned PDF.
You can also force OCR on any document:

=== "C"

    --8<-- "snippets/c/ocr/ocr_extraction.md"

=== "C#"

    --8<-- "snippets/csharp/ocr_extraction.md"

=== "Dart"

    --8<-- "snippets/dart/ocr/ocr_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_extraction.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/ocr/ocr_extraction.md"

=== "Python"

    --8<-- "snippets/python/ocr/ocr_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_extraction.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_extraction.md"

=== "Swift"

    --8<-- "snippets/swift/ocr/ocr_extraction.md"

=== "Elixir"

    --8<-- "snippets/elixir/ocr/tesseract_basic.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_extraction.md"

=== "Wasm"

    --8<-- "snippets/wasm/ocr/ocr_extraction.md"

=== "Zig"

    --8<-- "snippets/zig/ocr/ocr_extraction.md"

=== "CLI"

    --8<-- "snippets/cli/ocr_basic.md"

## Process Multiple Inputs

Pass a list of `ExtractInput` values to `extract_batch`. Mix `kind = "uri"` and
`kind = "bytes"` inputs when needed:

=== "C"

    --8<-- "snippets/c/api/extract_batch.md"

=== "C#"

    --8<-- "snippets/csharp/extract_batch.md"

=== "Dart"

    --8<-- "snippets/dart/api/extract_batch.md"

=== "Go"

    --8<-- "snippets/go/api/extract_batch.md"

=== "Java"

    --8<-- "snippets/java/api/extract_batch.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/api/extract_batch.md"

=== "Python"

    --8<-- "snippets/python/api/extract_batch.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_batch.md"

=== "R"

    --8<-- "snippets/r/api/extract_batch.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_batch.md"

=== "Swift"

    --8<-- "snippets/swift/api/extract_batch.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/extract_batch.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_batch.md"

=== "Wasm"

    --8<-- "snippets/wasm/getting-started/extract_batch.md"

=== "Zig"

    --8<-- "snippets/zig/api/extract_batch.md"

=== "CLI"

    --8<-- "snippets/cli/batch_basic.md"

## Read Document Metadata

Every `ExtractionOutput` contains result metadata in `results`. Each
`ExtractionResult` includes format-specific metadata: page count for PDFs, sheet
names for Excel, dimensions for images:

=== "C"

    --8<-- "snippets/c/metadata/metadata.md"

=== "C#"

    --8<-- "snippets/csharp/metadata.md"

=== "Dart"

    --8<-- "snippets/dart/metadata/metadata.md"

=== "Go"

    --8<-- "snippets/go/metadata/metadata.md"

=== "Java"

    --8<-- "snippets/java/metadata/metadata.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/metadata/metadata.md"

=== "Python"

    --8<-- "snippets/python/metadata/metadata.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/metadata.md"

=== "R"

    --8<-- "snippets/r/metadata/metadata.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/metadata.md"

=== "Swift"

    --8<-- "snippets/swift/metadata/metadata.md"

=== "Elixir"

    --8<-- "snippets/elixir/advanced/metadata_extraction.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata/metadata.md"

=== "Wasm"

    --8<-- "snippets/wasm/metadata/metadata.md"

=== "Zig"

    --8<-- "snippets/zig/metadata/metadata.md"

=== "CLI"

    Extract and parse metadata using JSON output:

    ```bash title="Terminal"
    # Extract with metadata (JSON format includes metadata automatically)
    xberg extract document.pdf --format json

    # Save to file and parse metadata
    xberg extract document.pdf --format json > result.json

    # Print all metadata fields
    cat result.json | jq '.metadata'

    # Extract HTML metadata
    xberg extract page.html --format json | jq '.metadata'

    # Get specific fields
    xberg extract document.pdf --format json | \
      jq '.metadata | {page_count, authors, title}'

    # Process multiple files
    xberg batch documents/*.pdf --format json > all_metadata.json
    ```

    **JSON Output Structure:**

    ```json title="JSON"
    {
      "results": [
        {
          "content": "Extracted text...",
          "mime_type": "application/pdf",
          "metadata": {
            "title": "Document Title",
            "authors": ["John Doe"],
            "created_by": "LaTeX with hyperref package",
            "format_type": "pdf",
            "page_count": 10
          },
          "tables": []
        }
      ],
      "errors": [],
      "summary": {
        "inputs": 1,
        "results": 1,
        "errors": 0
      }
    }
    ```

Xberg extracts format-specific metadata for:

- **PDF**: page count, title, authors (list), creation date, modification date
- **HTML**: SEO tags, Open Graph, Twitter Card, structured data, headers, links, images
- **Excel**: sheet count, sheet names
- **Email**: from, to, CC, BCC, message ID, attachments
- **PowerPoint**: title, author, description, fonts
- **Images**: dimensions, format, EXIF data
- **Archives**: format, file count, file list, sizes
- **XML**: element count, unique elements
- **Text/Markdown**: word count, line count, headers, links

See [Types Reference](../reference/types.md) for complete metadata reference.

## Extract Tables

Tables come back as both structured cells and Markdown. Xberg extracts them
from PDFs, spreadsheets, and HTML:

=== "C"

    --8<-- "snippets/c/metadata/tables.md"

=== "C#"

    --8<-- "snippets/csharp/tables.md"

=== "Dart"

    --8<-- "snippets/dart/metadata/tables.md"

=== "Go"

    --8<-- "snippets/go/metadata/tables.md"

=== "Java"

    --8<-- "snippets/java/metadata/tables.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/metadata/tables.md"

=== "Python"

    --8<-- "snippets/python/utils/tables.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/tables.md"

=== "R"

    --8<-- "snippets/r/metadata/tables.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/tables.md"

=== "Swift"

    --8<-- "snippets/swift/metadata/tables.md"

=== "Elixir"

    --8<-- "snippets/elixir/advanced/table_extraction.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/api/tables.md"

=== "Wasm"

    --8<-- "snippets/wasm/api/tables.md"

=== "Zig"

    --8<-- "snippets/zig/metadata/tables.md"

=== "CLI"

    Extract and process tables from documents:

    ```bash title="Terminal"
    # Extract with JSON format (includes tables when detected)
    xberg extract document.pdf --format json

    # Save tables to JSON
    xberg extract spreadsheet.xlsx --format json > tables.json

    # Extract and parse table markdown
    xberg extract document.pdf --format json | \
      jq '.tables[]? | .markdown'

    # Get table cells
    xberg extract document.pdf --format json | \
      jq '.tables[]? | .cells'

    # Batch extract tables from multiple files
    xberg batch documents/**/*.pdf --format json > all_tables.json
    ```

    **JSON Table Structure:**

    ```json title="JSON"
    {
      "results": [
        {
          "content": "...",
          "tables": [
            {
              "cells": [
                ["Name", "Age", "City"],
                ["Alice", "30", "New York"],
                ["Bob", "25", "Los Angeles"]
              ],
              "markdown": "| Name | Age | City |\\n|------|-----|--------|\\n| Alice | 30 | New York |\\n| Bob | 25 | Los Angeles |"
            }
          ]
        }
      ],
      "errors": [],
      "summary": {
        "inputs": 1,
        "results": 1,
        "errors": 0
      }
    }
    ```

## Going Async

Use async extraction in web servers, background workers, or anywhere you need
non-blocking I/O:

=== "C"

    --8<-- "snippets/c/api/extract.md"

=== "C#"

    --8<-- "snippets/csharp/extract.md"

=== "Dart"

    --8<-- "snippets/dart/api/extract.md"

=== "Go"

    --8<-- "snippets/go/api/extract.md"

=== "Java"

    --8<-- "snippets/java/api/extract.md"

=== "Kotlin"

    --8<-- "snippets/kotlin/api/extract.md"

=== "Python"

    --8<-- "snippets/python/api/extract.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract.md"

=== "R"

    --8<-- "snippets/r/api/extract.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract.md"

=== "Swift"

    --8<-- "snippets/swift/api/extract.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/extract.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract.md"

=== "Wasm"

    --8<-- "snippets/wasm/getting-started/extract.md"

=== "Zig"

    --8<-- "snippets/zig/api/extract.md"

=== "CLI"

    !!! note "Not Applicable"
        Async extraction is an API-level feature. The CLI operates synchronously.
        Use language-specific bindings (Python, TypeScript, Rust, WASM) for async operations.

## Next Steps

You've covered the core API. Go deeper:

- **[Configuration Guide](../guides/configuration.md)** — OCR backends, chunking, language detection, config files
- **[Extract from Bytes](../reference/api-python.md#extract)** — Use `ExtractInput(kind="bytes")`
- **[OCR Setup](../guides/ocr.md)** — Tesseract, PaddleOCR, EasyOCR backends
- **[Types Reference](../reference/types.md)** — Full metadata fields for every format
- **[Docker Deployment](../guides/docker.md)** — Run Xberg in containers
- **[API Reference](../reference/api-python.md)** — Complete API documentation
