# Configuration Guide

All extraction behavior is controlled through `ExtractionConfig`. Pass it directly in code or load it from a TOML/YAML/JSON file. Every field is optional. For per-field documentation, see the [Configuration Reference](../reference/configuration.md).

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

## Configuration Files

Three formats are supported. TOML is recommended.

=== "TOML (Recommended)"

    ```toml title="xberg.toml"
    use_cache = true
    enable_quality_processing = true

    [ocr]
    backend = "tesseract"
    language = "eng"

    [ocr.tesseract_config]
    psm = 3
    ```

=== "YAML"

    ```yaml title="xberg.yaml"
    use_cache: true
    enable_quality_processing: true

    ocr:
      backend: tesseract
      language: eng
      tesseract_config:
        psm: 3
    ```

=== "JSON"

    ```json title="xberg.json"
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

When no `--config` path is supplied, Xberg walks up from the current working directory looking for `xberg.toml` and uses the first match. YAML and JSON files are supported only when passed explicitly via `--config`. If nothing is found, defaults are used.

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

=== "Wasm"

    --8<-- "snippets/wasm/config/config_discover.md"

## Common Use Cases

### Setting Up OCR

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

For backend selection and language packs, see [OCR Guide](ocr.md). For fine-grained Tesseract tuning, see [TesseractConfig Reference](../reference/configuration.md#tesseractconfig).

### Chunking for RAG

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

## All Configuration Categories

- [ExtractionConfig](../reference/configuration.md#extractionconfig) — top-level options
- [OcrConfig](../reference/configuration.md#ocrconfig) — OCR backend, language, acceleration
- [TesseractConfig](../reference/configuration.md#tesseractconfig) — Tesseract PSM, confidence, table detection
- [ChunkingConfig](../reference/configuration.md#chunkingconfig) — chunk size, overlap
- [TokenReductionConfig](../reference/configuration.md#tokenreductionconfig) — LLM prompt token reduction
- [ContentFilterConfig](../reference/configuration.md#contentfilterconfig) — header/footer/watermark filtering
- [PageConfig](../reference/configuration.md#pageconfig) — page tracking and markers
- [AccelerationConfig](../reference/configuration.md#accelerationconfig) — ONNX Runtime execution provider

## Next Steps

- [Extraction Basics](extraction.md) — core extraction API and supported formats
- [OCR Guide](ocr.md) — backend installation and language setup
- [Embeddings](embeddings.md) — semantic vectors for search
- [Language Detection](language-detection.md) — multilingual document analysis
- [Chunking](chunking.md) — split text for RAG with page tracking
- [Plugins Guide](plugins.md) — custom post-processors and validators
