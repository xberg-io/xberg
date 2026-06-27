# Creating Plugins

Extend Xberg with custom extractors, post-processors, OCR backends, and
validators registered globally for use across all extraction calls.

!!! Note "Wasm" Custom plugins are not supported in Wasm environments. Use Python, Rust, or other native bindings.

## Plugin Types

| Type                  | Purpose                           | Use case                                                   |
| --------------------- | --------------------------------- | ---------------------------------------------------------- |
| **DocumentExtractor** | Extract content from file formats | New format support, override built-in extractors           |
| **PostProcessor**     | Transform extraction results      | Metadata enrichment, content filtering, text normalization |
| **OcrBackend**        | Perform OCR on images             | Cloud OCR services, custom OCR engines                     |
| **Validator**         | Validate extraction quality       | Minimum content length, quality score thresholds           |

All plugins must be thread-safe (`Send + Sync` in Rust, thread-safe in Python) and
implement `initialize()` / `shutdown()` lifecycle methods.

## Document Extractors

### Implementation

=== "Rust"

    --8<-- "snippets/rust/plugins/plugin_extractor.md"

=== "Python"

    --8<-- "snippets/python/plugins/plugin_extractor.md"

### Registration

=== "Python"

    --8<-- "snippets/python/plugins/extractor_registration.md"

=== "TypeScript"

    --8<-- "snippets/typescript/plugins/custom_extractor_plugin.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/extractor_registration.md"

=== "Go"

    --8<-- "snippets/go/plugins/extractor_registration.md"

=== "Java"

    --8<-- "snippets/java/plugins/extractor_registration.md"

=== "C#"

    --8<-- "snippets/csharp/extractor_registration.md"

=== "Ruby"

    --8<-- "snippets/ruby/plugins/extractor_registration.md"

=== "R"

    --8<-- "snippets/r/plugins/extractor_registration.md"

### Priority System

When multiple extractors support the same MIME type, the highest priority wins:

| Range  | Level                       |
| ------ | --------------------------- |
| 0–25   | Fallback / low-quality      |
| 26–49  | Alternative                 |
| **50** | **Default (built-in)**      |
| 51–75  | Enhanced / premium          |
| 76–100 | Specialized / high-priority |

## Post-Processors

Processors execute in three stages:

- **Early** — Foundational: language detection, quality scoring, text normalization
- **Middle** — Transformation: keyword extraction, token reduction, summarization
- **Late** — Final: custom metadata, analytics, output formatting

### Implementation

=== "Rust"

    --8<-- "snippets/rust/plugins/word_count_processor.md"

=== "Python"

    --8<-- "snippets/python/plugins/word_count_processor.md"

### Conditional Processing

=== "Python"

    --8<-- "snippets/python/plugins/pdf_only_processor.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/pdf_only_processor.md"

=== "Go"

    --8<-- "snippets/go/plugins/pdf_only_processor.md"

=== "Java"

    --8<-- "snippets/java/plugins/pdf_only_processor.md"

=== "C#"

    --8<-- "snippets/csharp/pdf_only_processor.md"

## OCR Backends

### Implementation

=== "Rust"

    --8<-- "snippets/rust/ocr/cloud_ocr_backend.md"

=== "Python"

    --8<-- "snippets/python/ocr/cloud_ocr_backend.md"

=== "Java"

    --8<-- "snippets/java/ocr/cloud_ocr_backend.md"

=== "C#"

    --8<-- "snippets/csharp/cloud_ocr_backend.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/cloud_ocr_backend.md"

=== "R"

    --8<-- "snippets/r/ocr/cloud_ocr_backend.md"

### Registration

Register the backend and set its name in `OcrConfig`:

=== "Python"

    ```python title="Python"
    from xberg import register_ocr_backend, unregister_ocr_backend

    backend = CloudOcrBackend(api_key="your-api-key")
    register_ocr_backend(backend)

    from xberg import extract, ExtractionConfig, OcrConfig

    config = ExtractionConfig(ocr=OcrConfig(backend="cloud-ocr", language="eng"))
    result = extract("scanned.pdf", config=config)

    unregister_ocr_backend("cloud-ocr")
    ```

### Using EasyOCR (Built-in)

The built-in EasyOCR backend supports 80+ languages and GPU acceleration — just point `OcrConfig` at it:

=== "Python"

    --8<-- "snippets/python/ocr/ocr_easyocr.md"

## Validators

!!! Warning Validation errors cause extraction to fail. Use validators for critical quality checks only.

=== "Rust"

    --8<-- "snippets/rust/plugins/min_length_validator.md"

=== "Python"

    --8<-- "snippets/python/plugins/min_length_validator.md"

=== "Java"

    --8<-- "snippets/java/plugins/min_length_validator.md"

=== "C#"

    --8<-- "snippets/csharp/min_length_validator.md"

### Quality Score Validator

=== "Rust"

    --8<-- "snippets/rust/plugins/quality_score_validator.md"

=== "Python"

    --8<-- "snippets/python/plugins/quality_score_validator.md"

=== "Java"

    --8<-- "snippets/java/plugins/quality_score_validator.md"

=== "C#"

    --8<-- "snippets/csharp/quality_score_validator.md"

## Plugin Management

### Listing

=== "Python"

    --8<-- "snippets/python/plugins/list_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/list_plugins.md"

=== "Java"

    --8<-- "snippets/java/plugins/list_plugins.md"

=== "C#"

    --8<-- "snippets/csharp/list_plugins.md"

### Unregistering

=== "Python"

    --8<-- "snippets/python/plugins/unregister_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/unregister_plugins.md"

=== "Java"

    --8<-- "snippets/java/plugins/unregister_plugins.md"

=== "C#"

    --8<-- "snippets/csharp/unregister_plugins.md"

### Clearing All

=== "Python"

    --8<-- "snippets/python/plugins/clear_plugins.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/clear_plugins.md"

=== "Java"

    --8<-- "snippets/java/plugins/clear_plugins.md"

=== "C#"

    --8<-- "snippets/csharp/clear_plugins.md"

## Thread Safety

=== "Rust"

    --8<-- "snippets/rust/plugins/stateful_plugin.md"

=== "Python"

    --8<-- "snippets/python/plugins/stateful_plugin.md"

=== "Java"

    --8<-- "snippets/java/plugins/stateful_plugin.md"

=== "C#"

    --8<-- "snippets/csharp/stateful_plugin.md"

## Best Practices

**Naming:** Use kebab-case (`my-custom-plugin`), lowercase only, no spaces or special characters.

### Logging

=== "Python"

    --8<-- "snippets/python/plugins/plugin_logging.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/plugin_logging.md"

=== "Java"

    --8<-- "snippets/java/plugins/plugin_logging.md"

=== "C#"

    --8<-- "snippets/csharp/plugin_logging.md"

### Testing

=== "Python"

    --8<-- "snippets/python/plugins/plugin_testing.md"

=== "Rust"

    --8<-- "snippets/rust/plugins/plugin_testing.md"

=== "Java"

    --8<-- "snippets/java/plugins/plugin_testing.md"

=== "C#"

    --8<-- "snippets/csharp/plugin_testing.md"

## Complete Example: PDF Metadata Extractor

=== "Python"

    --8<-- "snippets/python/metadata/pdf_metadata_extractor.md"

=== "Go"

    --8<-- "snippets/go/plugins/pdf_metadata_extractor.md"

=== "Java"

    --8<-- "snippets/java/plugins/pdf_metadata_extractor.md"

=== "C#"

    --8<-- "snippets/csharp/pdf_metadata_extractor.md"

=== "Ruby"

    --8<-- "snippets/ruby/plugins/pdf_metadata_extractor.md"

=== "R"

    --8<-- "snippets/r/plugins/pdf_metadata_extractor.md"
