# Quick Start

This guide walks you through Kreuzberg's core API — extracting text, handling errors,
running OCR, and working with metadata. Install your binding first if you haven't:
[Installation](installation.md).

!!! info "Node.js or Browser?"

    Kreuzberg provides **two TypeScript packages** for different runtimes:

    - **`@kreuzberg/node`** – Use for Node.js servers and CLI tools (native performance, 100% speed)
    - **`@kreuzberg/wasm`** – Use for browsers, Cloudflare Workers, Deno, Bun, and serverless (60-80% speed, cross-platform)

    The examples below show both. Pick the one matching your runtime. See [Platform Overview](../index.md#language-support) for detailed guidance.

## Your First Extraction

Pass a file path to get its text content. Kreuzberg detects the format automatically:

=== "C"

    --8<-- "snippets/c/api/extract_file_sync.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_sync.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_sync.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_sync.md"

=== "Python"

    --8<-- "snippets/python/api/extract_file_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_sync.md"

=== "R"

    --8<-- "snippets/r/api/extract_file_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_sync.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/extract_file_sync.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/extract_file_sync.md"

=== "CLI"

    --8<-- "snippets/cli/extract_basic.md"

## Handle Errors

Wrap extractions in error handling before going further. Kreuzberg raises specific
exceptions for missing files, parse failures, and OCR problems:

=== "C"

    --8<-- "snippets/c/api/error_handling.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling.md"

=== "Python"

    --8<-- "snippets/python/utils/error_handling.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling.md"

=== "R"

    --8<-- "snippets/r/api/error_handling.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/error_handling.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling.md"

=== "WASM"

    --8<-- "snippets/wasm/api/error_handling.md"

## OCR for Scanned Documents

Kreuzberg runs OCR automatically when it detects an image or scanned PDF.
You can also force OCR on any document:

=== "C"

    --8<-- "snippets/c/ocr/ocr_extraction.md"

=== "C#"

    --8<-- "snippets/csharp/ocr_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_extraction.md"

=== "Python"

    --8<-- "snippets/python/ocr/ocr_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_extraction.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_extraction.md"

=== "Elixir"

    --8<-- "snippets/elixir/ocr/tesseract_basic.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_extraction.md"

=== "WASM"

    --8<-- "snippets/wasm/ocr/ocr_extraction.md"

=== "CLI"

    --8<-- "snippets/cli/ocr_basic.md"

## Process Multiple Files

Pass a list of paths to extract them in parallel:

=== "C"

    --8<-- "snippets/c/api/batch_extract_files_sync.md"

=== "C#"

    --8<-- "snippets/csharp/batch_extract_files_sync.md"

=== "Go"

    --8<-- "snippets/go/api/batch_extract_files_sync.md"

=== "Java"

    --8<-- "snippets/java/api/batch_extract_files_sync.md"

=== "Python"

    --8<-- "snippets/python/api/batch_extract_files_sync.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/batch_extract_files_sync.md"

=== "R"

    --8<-- "snippets/r/api/batch_extract_files_sync.md"

=== "Rust"

    --8<-- "snippets/rust/api/batch_extract_files_sync.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/batch_extract_files_sync.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/batch_extract_files_sync.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/batch_extract_files_sync.md"

=== "CLI"

    --8<-- "snippets/cli/batch_basic.md"

## Read Document Metadata

Every extraction result includes format-specific metadata — page count for PDFs,
sheet names for Excel, dimensions for images:

=== "C"

    --8<-- "snippets/c/metadata/metadata.md"

=== "C#"

    --8<-- "snippets/csharp/metadata.md"

=== "Go"

    --8<-- "snippets/go/metadata/metadata.md"

=== "Java"

    --8<-- "snippets/java/metadata/metadata.md"

=== "Python"

    --8<-- "snippets/python/metadata/metadata.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/metadata.md"

=== "R"

    --8<-- "snippets/r/metadata/metadata.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/metadata.md"

=== "Elixir"

    --8<-- "snippets/elixir/advanced/metadata_extraction.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/metadata/metadata.md"

=== "WASM"

    --8<-- "snippets/wasm/metadata/metadata.md"

=== "CLI"

    Extract and parse metadata using JSON output:

    ```bash title="Terminal"
    # Extract with metadata (JSON format includes metadata automatically)
    kreuzberg extract document.pdf --format json

    # Save to file and parse metadata
    kreuzberg extract document.pdf --format json > result.json

    # Print all metadata fields
    cat result.json | jq '.metadata'

    # Extract HTML metadata
    kreuzberg extract page.html --format json | jq '.metadata'

    # Get specific fields
    kreuzberg extract document.pdf --format json | \
      jq '.metadata | {page_count, authors, title}'

    # Process multiple files
    kreuzberg batch documents/*.pdf --format json > all_metadata.json
    ```

    **JSON Output Structure:**

    ```json title="JSON"
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
    ```

Kreuzberg extracts format-specific metadata for:

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

Tables come back as both structured cells and Markdown. Kreuzberg extracts them
from PDFs, spreadsheets, and HTML:

=== "C"

    --8<-- "snippets/c/metadata/tables.md"

=== "C#"

    --8<-- "snippets/csharp/tables.md"

=== "Go"

    --8<-- "snippets/go/metadata/tables.md"

=== "Java"

    --8<-- "snippets/java/metadata/tables.md"

=== "Python"

    --8<-- "snippets/python/utils/tables.md"

=== "Ruby"

    --8<-- "snippets/ruby/metadata/tables.md"

=== "R"

    --8<-- "snippets/r/metadata/tables.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/tables.md"

=== "Elixir"

    --8<-- "snippets/elixir/advanced/table_extraction.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/api/tables.md"

=== "WASM"

    --8<-- "snippets/wasm/api/tables.md"

=== "CLI"

    Extract and process tables from documents:

    ```bash title="Terminal"
    # Extract with JSON format (includes tables when detected)
    kreuzberg extract document.pdf --format json

    # Save tables to JSON
    kreuzberg extract spreadsheet.xlsx --format json > tables.json

    # Extract and parse table markdown
    kreuzberg extract document.pdf --format json | \
      jq '.tables[]? | .markdown'

    # Get table cells
    kreuzberg extract document.pdf --format json | \
      jq '.tables[]? | .cells'

    # Batch extract tables from multiple files
    kreuzberg batch documents/**/*.pdf --format json > all_tables.json
    ```

    **JSON Table Structure:**

    ```json title="JSON"
    {
      "content": "...",
      "tables": [
        {
          "cells": [
            ["Name", "Age", "City"],
            ["Alice", "30", "New York"],
            ["Bob", "25", "Los Angeles"]
          ],
          "markdown": "| Name | Age | City |\n|------|-----|--------|\n| Alice | 30 | New York |\n| Bob | 25 | Los Angeles |"
        }
      ]
    }
    ```

## Going Async

Use async extraction in web servers, background workers, or anywhere you need
non-blocking I/O:

=== "C"

    --8<-- "snippets/c/api/extract_file_async.md"

=== "C#"

    --8<-- "snippets/csharp/extract_file_async.md"

=== "Go"

    --8<-- "snippets/go/api/extract_file_async.md"

=== "Java"

    --8<-- "snippets/java/api/extract_file_async.md"

=== "Python"

    --8<-- "snippets/python/api/extract_file_async.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/extract_file_async.md"

=== "R"

    --8<-- "snippets/r/api/extract_file_async.md"

=== "Rust"

    --8<-- "snippets/rust/api/extract_file_async.md"

=== "Elixir"

    --8<-- "snippets/elixir/core/extract_file_async.exs"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/extract_file_async.md"

=== "WASM"

    --8<-- "snippets/wasm/getting-started/extract_file_async.md"

=== "CLI"

    !!! note "Not Applicable"
        Async extraction is an API-level feature. The CLI operates synchronously.
        Use language-specific bindings (Python, TypeScript, Rust, WASM) for async operations.

## Next Steps

You've covered the core API. Go deeper:

- **[Configuration Guide](../guides/configuration.md)** — OCR backends, chunking, language detection, config files
- **[Extract from Bytes](../reference/api-python.md#extract_bytes_sync)** — Process in-memory data without writing to disk
- **[OCR Setup](../guides/ocr.md)** — Tesseract, PaddleOCR, EasyOCR backends
- **[Types Reference](../reference/types.md)** — Full metadata fields for every format
- **[Docker Deployment](../guides/docker.md)** — Run Kreuzberg in containers
- **[API Reference](../reference/api-python.md)** — Complete API documentation
