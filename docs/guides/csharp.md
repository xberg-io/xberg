# C# Bindings <span class="version-badge">v4.0.0</span>

.NET bindings for Kreuzberg document extraction via P/Invoke to the native Rust library. All extraction, configuration, and plugin APIs are exposed through the static `KreuzbergClient` class.

For cross-language extraction concepts, see [Extraction Basics](extraction.md). For configuration options, see [Configuration Guide](configuration.md).

## Installation

```bash title="Terminal"
dotnet add package Kreuzberg
```

Requires **.NET 10.0+** on Windows, macOS, or Linux.

For OCR support, install Tesseract separately:

```bash title="Terminal"
# macOS
brew install tesseract

# Ubuntu/Debian
sudo apt-get install tesseract-ocr
```

## Quick Start

### Extract a File

--8<-- "snippets/csharp/extract_file_sync.md"

### Async Extraction

--8<-- "snippets/csharp/extract_file_async.md"

### Extract from Bytes

When you already have file contents in memory, pass the byte array with its MIME type:

--8<-- "snippets/csharp/extract_bytes_sync.md"

### Batch Processing

Process multiple files in a single call with automatic parallelization:

--8<-- "snippets/csharp/batch_extract_files_sync.md"

## Configuration

Pass an `ExtractionConfig` to any extraction method. All fields are optional — defaults work for most cases.

### Basic Configuration

--8<-- "snippets/csharp/config_basic.md"

### OCR Configuration

--8<-- "snippets/csharp/config_ocr.md"

For fine-grained Tesseract tuning (PSM modes, confidence thresholds, table detection):

--8<-- "snippets/csharp/tesseract_config.md"

See [OCR Guide](ocr.md) for backend options and language setup.

### Load from File

Kreuzberg discovers configuration files automatically by walking up the directory tree, or you can load from an explicit path:

--8<-- "snippets/csharp/config_discover.md"

See [Configuration Guide](configuration.md) for file formats (TOML, YAML, JSON) and discovery order.

### Full Configuration Example

--8<-- "snippets/csharp/complete_example.md"

## Working with Results

### Tables

--8<-- "snippets/csharp/tables.md"

### Metadata

--8<-- "snippets/csharp/metadata.md"

### Image Extraction

--8<-- "snippets/csharp/image_extraction.md"

### Language Detection

--8<-- "snippets/csharp/language_detection.md"

### Token Reduction

Reduce token count for LLM pipelines while preserving meaning:

--8<-- "snippets/csharp/token_reduction.md"

## Error Handling

--8<-- "snippets/csharp/error_handling.md"

**Exception hierarchy:**

- `KreuzbergException` — base exception for all Kreuzberg errors
    - `KreuzbergValidationException` — invalid configuration or input
    - `KreuzbergParsingException` — document parsing failure
    - `KreuzbergOcrException` — OCR processing failure
    - `KreuzbergIOException` — file I/O failure
    - `KreuzbergMissingDependencyException` — missing optional dependency (e.g., Tesseract)
    - `KreuzbergSerializationException` — JSON serialization failure

## Thread Safety

`KreuzbergClient` static methods are thread-safe. Configuration objects are safe for concurrent reads. Plugin registrations use thread-safe collections. Individual `ExtractionResult` instances should not be shared across threads for mutation.

## Troubleshooting

??? question "DLL not found"

    Ensure the native library is in your runtime directory at `runtimes/{rid}/native/`.

    ```bash title="Terminal"
    echo $LD_LIBRARY_PATH     # Linux
    echo $DYLD_LIBRARY_PATH   # macOS
    ```

??? question "P/Invoke errors"

    Verify: (1) native library is installed, (2) architecture matches (x64/arm64), (3) Tesseract is available if using OCR.

??? question "OCR not working"

    ```bash title="Terminal"
    tesseract --version
    ```

    Ensure Tesseract is installed and in PATH.

## Next Steps

- [Extraction Basics](extraction.md) — sync vs async, batch processing, supported formats
- [Configuration Guide](configuration.md) — all configuration options and file formats
- [OCR Guide](ocr.md) — OCR backend setup and language packs
- [Plugins Guide](plugins.md) — custom post-processors, validators, and OCR backends
- [Advanced Features](advanced.md) — chunking, embeddings, and page tracking
