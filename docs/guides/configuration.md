# Configuration Guide <span class="version-badge">v4.0.0</span>

For complete field documentation, see [Configuration Reference](../reference/configuration.md).

All extraction behavior is controlled through `ExtractionConfig`. Every field is optional with sensible defaults — configure only what you need. You can pass config objects directly in code, or load them from TOML/YAML/JSON files.

## Quick Start

=== "Python"

    --8<-- "snippets/python/config/config_basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/config_basic.md"

=== "Rust"

    --8<-- "snippets/rust/config/config_basic.md"

=== "Go"

    --8<-- "snippets/go/config/config_basic.md"

=== "Java"

    --8<-- "snippets/java/config/config_basic.md"

=== "C#"

    --8<-- "snippets/csharp/config_basic.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/config_basic.md"

=== "R"

    --8<-- "snippets/r/config/config_basic.md"

## Configuration Files

Kreuzberg supports three file formats. TOML is recommended for readability.

=== "TOML (Recommended)"

    ```toml title="kreuzberg.toml"
    use_cache = true
    enable_quality_processing = true

    [ocr]
    backend = "tesseract"
    language = "eng"

    [ocr.tesseract_config]
    psm = 3
    ```

=== "YAML"

    ```yaml title="kreuzberg.yaml"
    use_cache: true
    enable_quality_processing: true

    ocr:
      backend: tesseract
      language: eng
      tesseract_config:
        psm: 3
    ```

=== "JSON"

    ```json title="kreuzberg.json"
    {
      "use_cache": true,
      "enable_quality_processing": true,
      "ocr": {
        "backend": "tesseract",
        "language": "eng",
        "tesseract_config": {
          "psm": 3
        }
      }
    }
    ```

### Automatic Discovery

Kreuzberg searches for configuration files in this order:

1. **Current directory** — `./kreuzberg.{toml,yaml,yml,json}`
2. **User config** — `~/.config/kreuzberg/config.{toml,yaml,yml,json}`
3. **System config** — `/etc/kreuzberg/config.{toml,yaml,yml,json}`

The first file found is merged with defaults. If no file exists, defaults are used.

=== "Python"

    --8<-- "snippets/python/config/config_discover.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/config_discover.md"

=== "Rust"

    --8<-- "snippets/rust/config/config_discover.md"

=== "Go"

    --8<-- "snippets/go/config/config_discover.md"

=== "Java"

    --8<-- "snippets/java/config/config_discover.md"

=== "C#"

    --8<-- "snippets/csharp/config_discover.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/config_discover.md"

=== "R"

    --8<-- "snippets/r/config/config_discover.md"

=== "WASM"

    --8<-- "snippets/wasm/config/config_discover.md"

## Common Use Cases

### Setting Up OCR

Enable OCR for scanned documents and images:

=== "Python"

    --8<-- "snippets/python/config/config_ocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/config_ocr.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/config_ocr.md"

=== "Go"

    --8<-- "snippets/go/config/config_ocr.md"

=== "Java"

    --8<-- "snippets/java/config/config_ocr.md"

=== "C#"

    --8<-- "snippets/csharp/config_ocr.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/config_ocr.md"

=== "R"

    --8<-- "snippets/r/config/config_ocr.md"

For backend selection and language packs, see [OCR Guide](ocr.md). For fine-grained Tesseract tuning, see [TesseractConfig Reference](../reference/configuration.md#tesseractconfig).

### Chunking for RAG

Split extracted text into overlapping chunks for vector database ingestion:

=== "Python"

    --8<-- "snippets/python/utils/chunking.md"

=== "TypeScript"

    --8<-- "snippets/typescript/utils/chunking.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/chunking.md"

=== "Go"

    --8<-- "snippets/go/utils/chunking.md"

=== "Java"

    --8<-- "snippets/java/utils/chunking.md"

=== "C#"

    --8<-- "snippets/csharp/advanced/embedding_with_chunking.md"

=== "Ruby"

    --8<-- "snippets/ruby/utils/chunking.md"

=== "R"

    --8<-- "snippets/r/utils/chunking.md"

## All Configuration Categories

Kreuzberg's configuration covers extraction behavior, OCR, formatting, chunking, and hardware acceleration:

- [ExtractionConfig](../reference/configuration.md#extractionconfig) — top-level options (cache, quality processing, output format, security limits)
- [OcrConfig](../reference/configuration.md#ocrconfig) — OCR backend, language, GPU acceleration
- [TesseractConfig](../reference/configuration.md#tesseractconfig) — Tesseract PSM mode, confidence, table detection
- [ChunkingConfig](../reference/configuration.md#chunkingconfig) — chunk size, overlap, strategy for RAG
- [TokenReductionConfig](../reference/configuration.md#tokenreductionconfig) — token count optimization for LLM prompts
- [ContentFilterConfig](../reference/configuration.md#contentfilterconfig) — header/footer/watermark filtering
- [PageConfig](../reference/configuration.md#pageconfig) — page tracking and markers
- [AccelerationConfig](../reference/configuration.md#accelerationconfig) — hardware acceleration (GPU, ONNX Runtime)

See [Configuration Reference](../reference/configuration.md) for the complete field documentation.

## Next Steps

- [Extraction Basics](extraction.md) — core extraction API and supported formats
- [OCR Guide](ocr.md) — backend installation and language setup
- [Advanced Features](advanced.md) — embeddings, language detection, page tracking
- [Plugins Guide](plugins.md) — custom post-processors and validators
