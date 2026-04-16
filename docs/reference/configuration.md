# Configuration Reference <span class="version-badge">v4.0.0</span>

This page provides complete documentation for all Kreuzberg configuration types and fields. For quick-start examples and common use cases, see the [Configuration Guide](../guides/configuration.md).

## Getting Started

New users should start with the [Configuration Guide](../guides/configuration.md) which covers:

- Configuration discovery mechanism
- Quick-start examples in all languages
- Common use cases (OCR setup, chunking for RAG)
- Configuration file formats (TOML, YAML, JSON)

This reference page is the comprehensive source for:

- All configuration field details
- Default values and constraints
- Technical specifications for each config type

---

## ServerConfig

**NEW in v4.2.7**: The ServerConfig controls API server and network settings.

API server configuration for the Kreuzberg HTTP server, including host/port settings, CORS configuration, and upload size limits. All settings can be overridden via environment variables.

### Overview

ServerConfig is used to customize the Kreuzberg API server behavior when running `kreuzberg serve` or embedding a Kreuzberg API server in your application. It controls network binding, cross-origin resource sharing (CORS), and file upload size constraints.

### Fields

| Field                       | Type            | Default       | Description                                                                                              |
| --------------------------- | --------------- | ------------- | -------------------------------------------------------------------------------------------------------- |
| `host`                      | `String`        | `"127.0.0.1"` | Server host address (for example, "127.0.0.1", "0.0.0.0")                                                       |
| `port`                      | `u16`           | `8000`        | Server port number (1-65535)                                                                             |
| `cors_origins`              | `Vec<String>`   | empty         | CORS allowed origins. Empty list allows all origins.                                                     |
| `max_request_body_bytes`    | `usize`         | `104857600`   | Maximum request body size in bytes (100 MB default)                                                      |
| `max_multipart_field_bytes` | `usize`         | `104857600`   | Maximum multipart field size in bytes (100 MB default)                                                   |

### Configuration Precedence

Settings are applied in this order (highest priority first):

1. **Environment Variables** - `KREUZBERG_*` variables override everything
2. **Configuration File** - TOML, YAML, or JSON values
3. **Programmatic Defaults** - Hard-coded defaults

### CORS Security Warning

The default configuration (empty `cors_origins` list) **allows requests from any origin**. This is suitable for development and internal APIs, but you should explicitly configure `cors_origins` for production deployments to prevent unauthorized cross-origin requests.

**Recommended for production:**

```toml title="Production CORS Configuration"
cors_origins = ["https://yourdomain.com", "https://app.yourdomain.com"]
```

### Configuration Examples

=== "Rust"

    ```rust title="basic_server_config.rs"
    use kreuzberg::core::ServerConfig;

    // Basic configuration with defaults
    let config = ServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8000);

    // Custom configuration
    let mut config = ServerConfig::default();
    config.host = "0.0.0.0".to_string();
    config.port = 3000;

    // Listen address helper
    println!("Server listening on: {}", config.listen_addr());
    ```

=== "Rust - CORS Configuration"

    ```rust title="cors_server_config.rs"
    use kreuzberg::core::ServerConfig;

    // Allow specific origins only (secure)
    let mut config = ServerConfig::default();
    config.cors_origins = vec![
        "https://app.example.com".to_string(),
        "https://admin.example.com".to_string(),
    ];

    // Check if origin is allowed
    assert!(config.is_origin_allowed("https://app.example.com"));
    assert!(!config.is_origin_allowed("https://evil.com"));

    // Check if allowing all origins
    assert!(!config.cors_allows_all());
    ```

=== "Rust - Upload Size Configuration"

    ```rust title="size_limits_config.rs"
    use kreuzberg::core::ServerConfig;

    // Custom size limits (200 MB)
    let mut config = ServerConfig::default();
    config.max_request_body_bytes = 200 * 1_048_576;  // 200 MB
    config.max_multipart_field_bytes = 200 * 1_048_576;  // 200 MB

    // Get sizes in MB
    println!("Max request body: {} MB", config.max_request_body_mb());
    println!("Max file upload: {} MB", config.max_multipart_field_mb());
    ```

=== "Rust - Load from File"

    ```rust title="load_server_config.rs"
    use kreuzberg::core::ServerConfig;

    // Auto-detect format from extension (.toml, .yaml, .json)
    let mut config = ServerConfig::from_file("server.toml")?;

    // Or use specific loaders
    let config = ServerConfig::from_toml_file("server.toml")?;
    let config = ServerConfig::from_yaml_file("server.yaml")?;
    let config = ServerConfig::from_json_file("server.json")?;

    // Apply environment variable overrides
    config.apply_env_overrides()?;
    ```

### Environment Variable Overrides

All settings can be overridden via environment variables with `KREUZBERG_` prefix:

```bash title="Terminal"
# Network settings
export KREUZBERG_HOST="0.0.0.0"
export KREUZBERG_PORT="3000"

# CORS configuration (comma-separated)
export KREUZBERG_CORS_ORIGINS="https://app1.com, https://app2.com"

# Size limits (in bytes)
export KREUZBERG_MAX_REQUEST_BODY_BYTES="209715200"      # 200 MB
export KREUZBERG_MAX_MULTIPART_FIELD_BYTES="209715200"   # 200 MB

kreuzberg serve
```

### Configuration File Examples

#### TOML Format

```toml title="server.toml"
# Basic server configuration
host = "0.0.0.0"          # Listen on all interfaces
port = 8000               # API port

# CORS configuration (empty = allow all)
cors_origins = [
    "https://app.example.com",
    "https://admin.example.com"
]

# Upload size limits (default: 100 MB)
max_request_body_bytes = 104857600      # 100 MB
max_multipart_field_bytes = 104857600   # 100 MB
```

#### YAML Format

```yaml title="server.yaml"
host: 0.0.0.0
port: 8000

cors_origins:
  - https://app.example.com
  - https://admin.example.com

max_request_body_bytes: 104857600
max_multipart_field_bytes: 104857600
```

#### JSON Format

```json title="server.json"
{
  "host": "0.0.0.0",
  "port": 8000,
  "cors_origins": ["https://app.example.com", "https://admin.example.com"],
  "max_request_body_bytes": 104857600,
  "max_multipart_field_bytes": 104857600
}
```

### Docker Integration

When deploying Kreuzberg in Docker, use environment variables to configure the server:

```dockerfile title="Dockerfile"
FROM kreuzberg:latest

ENV KREUZBERG_HOST="0.0.0.0"
ENV KREUZBERG_PORT="8000"
ENV KREUZBERG_CORS_ORIGINS="https://yourdomain.com"
ENV KREUZBERG_MAX_MULTIPART_FIELD_BYTES="524288000"

EXPOSE 8000

CMD ["kreuzberg", "serve"]
```

```bash title="Terminal - Run with Docker"
docker run -it \
  -e KREUZBERG_HOST="0.0.0.0" \
  -e KREUZBERG_PORT="3000" \
  -e KREUZBERG_CORS_ORIGINS="https://api.example.com" \
  -p 3000:3000 \
  kreuzberg:latest kreuzberg serve
```

---

## ExtractionConfig

Main extraction configuration controlling all aspects of document processing.

| Field                        | Type                       | Default                | Description                                                                                                                                                                                      |
| ---------------------------- | -------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `use_cache`                  | `bool`                     | `true`                 | Enable caching of extraction results for faster re-processing                                                                                                                                    |
| `enable_quality_processing`  | `bool`                     | `true`                 | Enable quality post-processing (deduplication, mojibake fixing, etc.)                                                                                                                            |
| `force_ocr`                  | `bool`                     | `false`                | Force OCR even for searchable PDFs with text layers                                                                                                                                              |
| `disable_ocr`                | `bool`                     | `false`                | Disable OCR entirely — image files return empty content instead of raising errors (v4.7.0+)                                                                                                      |
| `ocr`                        | `OcrConfig?`               | `None`                 | OCR configuration (if None, OCR disabled)                                                                                                                                                        |
| `pdf_options`                | `PdfConfig?`               | `None`                 | PDF-specific configuration options                                                                                                                                                               |
| `images`                     | `ImageExtractionConfig?`   | `None`                 | Image extraction configuration                                                                                                                                                                   |
| `chunking`                   | `ChunkingConfig?`          | `None`                 | Text chunking configuration for splitting into chunks                                                                                                                                            |
| `content_filter`             | `ContentFilterConfig?` <span class="version-badge">v4.8.0</span> | `None`                 | Header, footer, watermark, and repeating-text filtering. See [ContentFilterConfig](#contentfilterconfig).                                                                                        |
| `token_reduction`            | `TokenReductionConfig?`    | `None`                 | Token reduction configuration for optimizing LLM context                                                                                                                                         |
| `language_detection`         | `LanguageDetectionConfig?` | `None`                 | Automatic language detection configuration                                                                                                                                                       |
| `postprocessor`              | `PostProcessorConfig?`     | `None`                 | Post-processing pipeline configuration                                                                                                                                                           |
| `pages`                      | `PageConfig?`              | `None`                 | Page extraction and tracking configuration                                                                                                                                                       |
| `max_concurrent_extractions` | `int?`                     | `None`                 | Maximum concurrent batch extractions (defaults to num_cpus \* 2)                                                                                                                                 |
| `concurrency`                | `ConcurrencyConfig?` <span class="version-badge">v4.5.0</span> | `None`                 | Concurrency configuration for threading (max_threads caps Rayon, ONNX intra-op threads, and batch semaphore)                                                                                    |
| `result_format`              | `OutputFormat`             | `Unified`              | Result structure format: `Unified` (content in single field) or `ElementBased` (semantic elements array)                                                                                         |
| `output_format`              | `OutputFormat`             | `Plain`                | Output format for extracted text content (Plain, Markdown, Djot, Html, Structured)                                                                                                               |
| `html_options`               | `ConversionOptions`        | `None`                 | HTML to Markdown conversion options (heading styles, list formatting, code block styles). Only available with `html` feature.                                                                    |
| `html_output`                | `HtmlOutputConfig?` <span class="version-badge">v4.8.1</span>         | `None`                 | Styled HTML output configuration: theme selection, custom CSS, class prefix. When set alongside `output_format = Html`, activates the styled renderer with `kb-*` class hooks. Only available with `html` feature. |
| `security_limits`            | `SecurityLimits?`          | `None` (uses defaults) | Archive security thresholds: max archive size (500MB), compression ratio (100:1), file count (10K), nesting depth, content size, XML depth, table cells. Only available with `archives` feature. |
| `layout`                     | `LayoutDetectionConfig?`   | `None`                 | Layout detection configuration for document structure analysis. Only available with `layout-detection` feature.                                                                                   |
| `acceleration`               | `AccelerationConfig?`      | `None`                 | Hardware acceleration configuration for ONNX Runtime inference (layout detection and embeddings). See [AccelerationConfig](#accelerationconfig).                                                 |
| `include_document_structure` | `bool`                     | `false`                | Enable structured document model output. When true, the `document` field on ExtractionResult is populated with a tree-based representation of document content.                                  |
| `tree_sitter`                | `TreeSitterConfig?`        | `None`                 | Tree-sitter code intelligence configuration. Controls code analysis features when extracting source code files. Only available with `tree-sitter` feature.                                       |
| `structured_extraction`      | `StructuredExtractionConfig?` | `None`              | Structured extraction configuration for LLM-powered schema-based extraction. When set, extraction results include a `structured_output` field with data conforming to the provided JSON schema. Only available with `liter-llm` feature. |

### Result Format vs Output Format

**Important distinction:** These two fields control different aspects of extraction results:

- **`result_format`** - Controls the **structure** of the result:
  - `Unified` (default): All content returned in the `content` field as a single string
  - `ElementBased`: Content returned as semantic elements in the `elements` array (Unstructured-compatible format)

- **`output_format`** - Controls the **text format** within the content:
  - `Plain` (default): Raw extracted text
  - `Markdown`: Markdown formatted output
  - `Djot`: Djot markup format
  - `Html`: HTML formatted output

### OutputFormat (result_format field)

Controls the structure of extraction results:

| Value           | Description                                                   |
| --------------- | ------------------------------------------------------------- |
| `unified`       | All content in single `content` field (default)               |
| `element_based` | Semantic elements with type classification, IDs, and metadata |

When `result_format` is set to `ElementBased`, the `elements` field contains an array of semantic elements with unique identifiers, element types (title, heading, narrative_text, etc.), and metadata for Unstructured-compatible processing.

### OutputFormat (output_format field)

Output format for extraction content. Controls how extracted text is formatted in the result.

| Value        | Description                                                             |
| ------------ | ----------------------------------------------------------------------- |
| `plain`      | Plain text content only (default)                                       |
| `markdown`   | Markdown formatted output                                               |
| `djot`       | Djot markup format                                                      |
| `html`       | HTML formatted output                                                   |
| `structured` | Structured JSON with full OCR element data (bounding boxes, confidence) |

**Environment Variable:** `KREUZBERG_OUTPUT_FORMAT` - Set output format via environment (plain, markdown, djot, html, structured)

### HtmlOutputConfig

Configuration for the styled HTML renderer. When set on `ExtractionConfig.html_output` alongside `output_format = Html`, the pipeline produces HTML with semantic `kb-*` class hooks instead of plain HTML.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `theme` | `HtmlTheme` | `Unstyled` | Built-in colour/typography theme |
| `css` | `string?` | `None` | Inline CSS string appended after theme stylesheet |
| `css_file` | `path?` | `None` | CSS file loaded at render time (max 1 MiB) |
| `class_prefix` | `string` | `"kb-"` | CSS class prefix (alphanumeric + hyphens + underscores only) |
| `embed_css` | `bool` | `true` | Embed CSS in `<style>` block. Set `false` for external stylesheets |

### HtmlTheme

Built-in theme selection for styled HTML output.

| Value | Description |
|-------|-------------|
| `Unstyled` (default) | No built-in stylesheet. CSS custom properties defined on `:root` for user stylesheets |
| `Default` | System font stack, neutral colours, readable line measure |
| `GitHub` | GitHub Markdown-inspired palette and spacing |
| `Dark` | Dark background, light text |
| `Light` | Minimal light theme with generous whitespace |

### Example

=== "C#"

    ```csharp
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        UseCache = true,
        EnableQualityProcessing = true,
        ForceOcr = false,
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Go"

    --8<-- "snippets/go/config/config_basic.md"

=== "Java"

    --8<-- "snippets/java/config/config_basic.md"

=== "Python"

    --8<-- "snippets/python/config/config_basic.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/config_basic.md"

=== "R"

    --8<-- "snippets/r/config/config_basic.md"

=== "Rust"

    --8<-- "snippets/rust/config/config_basic.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/config_basic.md"

---

## FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All fields are optional — `None` means "use the batch-level default from `ExtractionConfig`."

When passed as an optional parameter to `batch_extract_file` / `batch_extract_bytes` (or their sync variants), each file in the batch can specify its own overrides that are merged with the shared batch-level `ExtractionConfig`.

### Overridable Fields

| Field                        | Type                       | Description                                      |
| ---------------------------- | -------------------------- | ------------------------------------------------ |
| `enable_quality_processing`  | `bool?`                    | Override quality post-processing for this file    |
| `ocr`                        | `OcrConfig?`               | Override OCR configuration                        |
| `force_ocr`                  | `bool?`                    | Override force OCR                                |
| `disable_ocr`                | `bool?`                    | Override disable OCR (v4.7.0+)                    |
| `chunking`                   | `ChunkingConfig?`          | Override text chunking                            |
| `content_filter`             | `ContentFilterConfig?`     | Override content filtering                        |
| `images`                     | `ImageExtractionConfig?`   | Override image extraction                         |
| `pdf_options`                | `PdfConfig?`               | Override PDF-specific options                     |
| `token_reduction`            | `TokenReductionConfig?`    | Override token reduction                          |
| `language_detection`         | `LanguageDetectionConfig?` | Override language detection                       |
| `pages`                      | `PageConfig?`              | Override page extraction                          |
| `keywords`                   | `KeywordConfig?`           | Override keyword extraction                       |
| `postprocessor`              | `PostProcessorConfig?`     | Override post-processing                          |
| `html_options`               | `ConversionOptions?`       | Override HTML conversion options                  |
| `result_format`              | `OutputFormat?`            | Override result structure format                  |
| `output_format`              | `OutputFormat?`            | Override output content format                    |
| `include_document_structure` | `bool?`                    | Override document structure output                |
| `layout`                     | `LayoutDetectionConfig?`   | Override layout detection                         |

### Batch-Level Only Fields (Not Overridable)

These `ExtractionConfig` fields cannot be overridden per file:

- `max_concurrent_extractions` — controls batch parallelism
- `use_cache` — global caching policy
- `acceleration` — shared ONNX execution provider
- `security_limits` — global archive security policy

### Merge Semantics

For each file in a batch, the effective configuration is computed by overlaying the per-file `FileExtractionConfig` onto the batch-level `ExtractionConfig`. A field set to `None` in `FileExtractionConfig` falls through to the batch default. A field set to `Some(value)` replaces the batch default entirely for that file.

### Example

=== "Rust"

    ```rust title="per_file_config.rs"
    use kreuzberg::{
        batch_extract_file, ExtractionConfig, FileExtractionConfig, OcrConfig,
    };
    use std::path::PathBuf;

    #[tokio::main]
    async fn main() -> kreuzberg::Result<()> {
        let batch_config = ExtractionConfig::default();

        let paths = vec![
            PathBuf::from("report.pdf"),
            PathBuf::from("scanned.pdf"),
        ];

        let file_configs = vec![
            None, // Use batch defaults for this PDF
            Some(FileExtractionConfig { // Force OCR for this scanned document
                force_ocr: Some(true),
                ocr: Some(OcrConfig {
                    backend: "tesseract".to_string(),
                    language: "deu".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        ];

        let results = batch_extract_file(paths, &batch_config, Some(&file_configs)).await?;
        Ok(())
    }
    ```

=== "Python"

    ```python title="per_file_config.py"
    from kreuzberg import (
        batch_extract_files_sync,
        ExtractionConfig,
        FileExtractionConfig,
        OcrConfig,
    )

    config = ExtractionConfig()

    paths = ["report.pdf", "scanned.pdf"]
    file_configs = [
        None,  # use batch defaults
        FileExtractionConfig(
            force_ocr=True,
            ocr=OcrConfig(backend="tesseract", language="deu"),
        ),
    ]

    results = batch_extract_files_sync(paths, config, file_configs=file_configs)
    ```

=== "TypeScript"

    ```typescript title="per_file_config.ts"
    import { batchExtractFilesSync } from '@kreuzberg/node';

    const results = batchExtractFilesSync(
      ['report.pdf', 'scanned.pdf'],
      undefined, // use default config
      [
        null,  // use batch defaults
        {      // per-file overrides
          forceOcr: true,
          ocr: { backend: 'tesseract', language: 'deu' },
        },
      ],
    );
    ```

---

## ContentFilterConfig <span class="version-badge">v4.8.0</span>

Controls whether headers, footers, watermarks, and repeating cross-page text are kept in or stripped from extraction output. Applies to PDF, DOCX, RTF, ODT, HTML, EPUB, and PPT extractors with format-specific behavior.

When `content_filter` is `None` on `ExtractionConfig`, each extractor uses its built-in defaults (the same values listed below).

### Fields

| Field                  | Type   | Default | Description                                                                                       |
| ---------------------- | ------ | ------- | ------------------------------------------------------------------------------------------------- |
| `include_headers`      | `bool` | `False` | Keep running headers. PDF skips top-margin furniture stripping; DOCX includes header parts; HTML/EPUB keep `<header>` content. |
| `include_footers`      | `bool` | `False` | Keep running footers. PDF skips bottom-margin furniture stripping; DOCX includes footer parts; HTML/EPUB keep `<footer>` content. |
| `strip_repeating_text` | `bool` | `True`  | Detect text that repeats verbatim across most pages and remove it. Disable if brand names or repeated headings are being incorrectly stripped. Primarily PDF. |
| `include_watermarks`   | `bool` | `False` | Keep watermark text and arXiv-style identifiers. PDF only.                                         |

The `strip_repeating_text` flag also gates paragraph deduplication: when set to `False`, near-duplicate paragraphs are preserved as well (kreuzberg/kreuzberg#681, fixed in v4.8.1).

When a layout-detection model is active, it can independently classify regions as `PageHeader` or `PageFooter` and strip them per page. To preserve those regions in addition to disabling the cross-page heuristic, set `include_headers = True` and/or `include_footers = True`.

### Configuration Examples

=== "Python"

    ```python title="content_filter_config.py"
    from kreuzberg import ExtractionConfig, ContentFilterConfig

    # Keep headers and footers for legal/forms work
    config = ExtractionConfig(
        content_filter=ContentFilterConfig(
            include_headers=True,
            include_footers=True,
        ),
    )
    ```

=== "TypeScript"

    ```typescript title="content_filter_config.ts"
    import { extract } from "@kreuzberg/node";

    // Disable cross-page repeating-text detection
    const result = await extract("report.pdf", {
      contentFilter: {
        stripRepeatingText: false,
      },
    });
    ```

=== "Rust"

    ```rust title="content_filter_config.rs"
    use kreuzberg::{ExtractionConfig, ContentFilterConfig};

    let config = ExtractionConfig {
        content_filter: Some(ContentFilterConfig {
            include_headers: true,
            include_footers: true,
            strip_repeating_text: true,
            include_watermarks: false,
        }),
        ..Default::default()
    };
    ```

### Configuration File Examples

=== "TOML"

    ```toml title="kreuzberg.toml"
    [content_filter]
    include_headers = true
    include_footers = true
    strip_repeating_text = true
    include_watermarks = false
    ```

=== "YAML"

    ```yaml title="kreuzberg.yaml"
    content_filter:
      include_headers: true
      include_footers: true
      strip_repeating_text: true
      include_watermarks: false
    ```

---

## OcrConfig

Configuration for OCR (Optical Character Recognition) processing on images and scanned PDFs.

| Field              | Type               | Default       | Description                                                           |
| ------------------ | ------------------ | ------------- | --------------------------------------------------------------------- |
| `backend`          | `str`              | `"tesseract"` | OCR backend to use: `"tesseract"`, `"easyocr"`, `"paddleocr"`         |
| `language`         | `str`              | `"eng"`       | Language code(s) for OCR, for example, `"eng"`, `"eng+fra"`, `"eng+deu+fra"` |
| `tesseract_config` | `TesseractConfig?` | `None`        | Tesseract-specific configuration options                              |
| `paddle_ocr_config` | `PaddleOcrConfig?` | `None`       | PaddleOCR-specific configuration options                              |
| `vlm_config`       | `LlmConfig?`       | `None`        | Vision Language Model configuration for VLM-based OCR. When set, enables using a VLM as an OCR backend. Requires the `liter-llm` feature. |
| `vlm_prompt`       | `String?`           | `None`        | Custom prompt for VLM-based OCR. Overrides the default OCR prompt sent to the vision model. Useful for domain-specific extraction instructions. |

### Example

=== "C#"

    --8<-- "snippets/csharp/config_ocr.md"

=== "Go"

    --8<-- "snippets/go/config/config_ocr.md"

=== "Java"

    --8<-- "snippets/java/config/config_ocr.md"

=== "Python"

    --8<-- "snippets/python/config/config_ocr.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/config_ocr.md"

=== "R"

    --8<-- "snippets/r/config/config_ocr.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_multi_language.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/config_ocr.md"

---

## PaddleOcrConfig <span class="version-badge">v4.5.0</span>

PaddleOCR-specific configuration for model selection and detection tuning.

| Field              | Type   | Default      | Description                                                                                                                        |
| ------------------ | ------ | ------------ | ---------------------------------------------------------------------------------------------------------------------------------- |
| `model_tier` <span class="version-badge">v4.5.0</span> | `str`  | `"mobile"`   | Model tier: `"mobile"` (lightweight, ~21MB total, fast) or `"server"` (high accuracy, ~172MB, best with GPU) |
| `padding` <span class="version-badge">v4.5.0</span>    | `int`  | `10`         | Padding in pixels (0-100) added around the image before detection                                     |

---

## TesseractConfig

Tesseract OCR engine configuration with fine-grained control over recognition parameters.

| Field                                | Type                        | Default      | Description                                              |
| ------------------------------------ | --------------------------- | ------------ | -------------------------------------------------------- |
| `language`                           | `str`                       | `"eng"`      | Language code(s), for example, `"eng"`, `"eng+fra"`             |
| `psm`                                | `int`                       | `3`          | Page Segmentation Mode (0-13, see below)                 |
| `output_format`                      | `str`                       | `"markdown"` | Output format: `"text"`, `"markdown"`, `"hocr"`          |
| `oem`                                | `int`                       | `3`          | OCR Engine Mode (0-3, see below)                         |
| `min_confidence`                     | `float`                     | `0.0`        | Minimum confidence threshold (0.0-100.0)                 |
| `preprocessing`                      | `ImagePreprocessingConfig?` | `None`       | Image preprocessing configuration                        |
| `enable_table_detection`             | `bool`                      | `true`       | Enable automatic table detection and reconstruction      |
| `table_min_confidence`               | `float`                     | `0.0`        | Minimum confidence for table cell recognition (0.0-1.0)  |
| `table_column_threshold`             | `int`                       | `50`         | Pixel threshold for detecting table columns              |
| `table_row_threshold_ratio`          | `float`                     | `0.5`        | Row threshold ratio for table detection (0.0-1.0)        |
| `use_cache`                          | `bool`                      | `true`       | Enable OCR result caching for faster re-processing       |
| `classify_use_pre_adapted_templates` | `bool`                      | `true`       | Use pre-adapted templates for character classification   |
| `language_model_ngram_on`            | `bool`                      | `false`      | Enable N-gram language model for better word recognition |
| `tessedit_dont_blkrej_good_wds`      | `bool`                      | `true`       | Don't reject good words during block-level processing    |
| `tessedit_dont_rowrej_good_wds`      | `bool`                      | `true`       | Don't reject good words during row-level processing      |
| `tessedit_enable_dict_correction`    | `bool`                      | `true`       | Enable dictionary-based word correction                  |
| `tessedit_char_whitelist`            | `str`                       | `""`         | Allowed characters (empty = all allowed)                 |
| `tessedit_char_blacklist`            | `str`                       | `""`         | Forbidden characters (empty = none forbidden)            |
| `tessedit_use_primary_params_model`  | `bool`                      | `true`       | Use primary language params model                        |
| `textord_space_size_is_variable`     | `bool`                      | `true`       | Enable variable-width space detection                    |
| `thresholding_method`                | `bool`                      | `false`      | Use adaptive thresholding method                         |

### Page Segmentation Modes (PSM)

- `0`: Orientation and script detection only (no OCR)
- `1`: Automatic page segmentation with OSD (Orientation and Script Detection)
- `2`: Automatic page segmentation (no OSD, no OCR)
- `3`: Fully automatic page segmentation (default, best for most documents)
- `4`: Single column of text of variable sizes
- `5`: Single uniform block of vertically aligned text
- `6`: Single uniform block of text (best for clean documents)
- `7`: Single text line
- `8`: Single word
- `9`: Single word in a circle
- `10`: Single character
- `11`: Sparse text with no particular order (best for forms, invoices)
- `12`: Sparse text with OSD
- `13`: Raw line (bypass Tesseract's layout analysis)

### OCR Engine Modes (OEM)

- `0`: Legacy Tesseract engine only (pre-2016)
- `1`: Neural nets LSTM engine only (recommended for best quality)
- `2`: Legacy + LSTM engines combined
- `3`: Default based on what's available (recommended for compatibility)

### Example

=== "C#"

    --8<-- "snippets/csharp/tesseract_config.md"

=== "Go"

    --8<-- "snippets/go/config/tesseract_config.md"

=== "Java"

    --8<-- "snippets/java/config/tesseract_config.md"

=== "Python"

    --8<-- "snippets/python/config/tesseract_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/tesseract_config.md"

=== "R"

    --8<-- "snippets/r/config/tesseract_config.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/tesseract_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/tesseract_config.md"

---

## ChunkingConfig

Configuration for splitting extracted text into overlapping chunks, useful for vector databases and LLM processing.

| Field            | Type               | Default | Description                                                                       |
| ---------------- | ------------------ | ------- | --------------------------------------------------------------------------------- |
| `max_characters` | `int`              | `1000`  | Maximum characters per chunk                                                      |
| `overlap`        | `int`              | `200`   | Overlap between consecutive chunks in characters                                  |
| `embedding`      | `EmbeddingConfig?` | `None`  | Optional embedding generation for each chunk                                      |
| `preset`         | `str?`             | `None`  | Chunking preset: `"small"` (500/100), `"medium"` (1000/200), `"large"` (2000/400) |
| `trim`           | `bool`             | `true`  | Whether to trim whitespace from chunk boundaries                                  |
| `chunker_type`   | `ChunkerType`      | `Text`  | Type of chunker: `Text`, `Markdown`, or `Yaml`                                    |
| `sizing` <span class="version-badge">v4.5.0</span> | `ChunkSizing`      | `Characters` | Controls how chunk size is measured. `Characters` counts characters (default). `Tokenizer` counts tokens using a HuggingFace tokenizer model. Requires the `chunking-tokenizers` feature |

**Note:** `max_chars` and `max_overlap` are accepted as aliases for `max_characters` and `overlap` respectively for backwards compatibility.

When `chunker_type` is set to `"markdown"`, the chunker populates `heading_context` on each chunk's metadata with the heading hierarchy (for example, `# Title > ## Section`) that the chunk falls under. This is useful for preserving semantic context in RAG pipelines.

### Example

=== "C#"

    --8<-- "snippets/csharp/advanced/chunking_config.cs"

=== "Go"

    --8<-- "snippets/go/config/chunking_config.md"

=== "Java"

    --8<-- "snippets/java/config/chunking_config.md"

=== "Python"

    --8<-- "snippets/python/config/chunking_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/chunking_config.md"

=== "R"

    --8<-- "snippets/r/config/chunking_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/chunking_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/chunking_config.md"

---

## EmbeddingConfig

Configuration for generating vector embeddings for text chunks. Enables semantic search and similarity matching by converting text into high-dimensional vector representations.

### Overview

EmbeddingConfig is used to control embedding generation when chunking documents. It allows you to choose from pre-optimized models or specify custom models from HuggingFace. Embeddings can be generated for each chunk to enable vector database integration and semantic search capabilities.

### Fields

| Field                    | Type                 | Default                          | Description                                                                    |
| ------------------------ | -------------------- | -------------------------------- | ------------------------------------------------------------------------------ |
| `model`                  | `EmbeddingModelType` | `Preset { name: "balanced" }`    | Embedding model selection (preset or custom)                                   |
| `batch_size`             | `usize`              | `32`                             | Number of texts to process in each batch (higher = faster but more memory)     |
| `normalize`              | `bool`               | `true`                           | Normalize embedding vectors to unit length (recommended for cosine similarity) |
| `show_download_progress` | `bool`               | `false`                          | Show progress when downloading model files                                     |
| `cache_dir`              | `String?`            | `~/.cache/kreuzberg/embeddings/` | Custom cache directory for downloaded models                                   |

### Model Types

#### Preset Models (Recommended)

Preset models are pre-optimized configurations for common use cases. They automatically download and cache the necessary model files.

| Preset         | Model              | Dims | Speed     | Quality     | Use Case                                                                    |
| -------------- | ------------------ | ---- | --------- | ----------- | --------------------------------------------------------------------------- |
| `fast`         | AllMiniLML6V2Q     | 384  | Very Fast | Good        | Development, prototyping, resource-constrained environments                 |
| `balanced`     | BGEBaseENV15       | 768  | Fast      | Excellent   | **Default**: General-purpose RAG, production deployments, English documents |
| `quality`      | BGELargeENV15      | 1024 | Moderate  | Outstanding | Complex documents, maximum accuracy, sufficient compute resources           |
| `multilingual` | MultilingualE5Base | 768  | Fast      | Excellent   | International documents, 100+ languages, mixed-language content             |

Preset models require the `embeddings` feature to be enabled in Kreuzberg.

**Model Characteristics:**

- **Fast**: ~22M parameters, 384-dimensional vectors. Best for quick prototyping and development where speed is prioritized over quality.
- **Balanced**: ~109M parameters, 768-dimensional vectors. Excellent general-purpose model with strong semantic understanding for most use cases.
- **Quality**: ~335M parameters, 1024-dimensional vectors. Large model for maximum semantic accuracy when compute resources are available.
- **Multilingual**: ~109M parameters, 768-dimensional vectors. Trained on multilingual data, effective for 100+ languages including rare languages.

#### FastEmbed Models

FastEmbed is a library for fast embedding generation. You can specify any supported FastEmbed model by name.

**Common FastEmbed models:**

- `AllMiniLML6V2Q` - 384 dims, fast, quantized (same as `fast` preset)
- `BGEBaseENV15` - 768 dims, balanced (same as `balanced` preset)
- `BGELargeENV15` - 1024 dims, high quality (same as `quality` preset)
- `MultilingualE5Base` - 768 dims, multilingual (same as `multilingual` preset)

Requires the `embeddings` feature and explicit dimensions specification.

#### Custom Models

Custom ONNX models from HuggingFace can be specified for specialized use cases. Provide the HuggingFace model ID and vector dimensions.

**Note**: Custom model support for full embedding generation is planned for future releases. Currently, custom models can be loaded and used via the Rust API.

#### LLM Provider-Hosted Embeddings

Instead of running local ONNX models, you can delegate embedding generation to a cloud provider's embedding API via liter-llm. This is useful when you want to use the same embedding model as your vector database provider or when local model hosting is impractical.

```rust title="llm_embedding.rs"
use kreuzberg::core::{EmbeddingConfig, EmbeddingModelType, LlmConfig};

let config = EmbeddingConfig {
    model: EmbeddingModelType::Llm {
        llm: LlmConfig {
            model: "openai/text-embedding-3-small".to_string(),
            api_key: None, // Falls back to OPENAI_API_KEY env var
            base_url: None,
        },
    },
    batch_size: 32,
    ..Default::default()
};
```

```toml title="kreuzberg.toml"
[chunking.embedding]
model = { type = "llm", model = "openai/text-embedding-3-small" }
batch_size = 32
```

**Note**: When `api_key` is not set in `LlmConfig`, liter-llm falls back to provider-standard environment variables (for example, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`). Requires the `liter-llm` feature.

### Cache Directory

Model files are cached locally to avoid re-downloading on subsequent runs.

**Default cache location:**

```text
~/.cache/kreuzberg/embeddings/
```

**Features:**

- Tilde (`~`) expansion: Home directory automatically resolved
- Automatic creation: Cache directory created if it doesn't exist
- Persistent across runs: Models cached indefinitely until manually removed
- Multi-process safe: Thread-safe concurrent access

**Custom cache directory:**

```toml title="Custom Embedding Cache Directory"
[chunking.embedding]
model = { type = "preset", name = "balanced" }
cache_dir = "/custom/cache/path"
```

### Performance Considerations

#### Batch Size Tuning

- **Default**: 32 texts per batch
- **Small values** (8-16): Lower memory usage, slower processing
- **Large values** (64-128): Faster processing, higher memory usage
- Adjust based on available GPU/CPU memory and document sizes

#### Normalization

- **Enabled (default)**: Vectors normalized to unit length, suitable for cosine similarity
- **Disabled**: Raw vectors suitable for other distance metrics (Euclidean, dot product)

#### Model Size Trade-offs

| Model        | Size   | Speed    | Quality     | Memory | Network |
| ------------ | ------ | -------- | ----------- | ------ | ------- |
| Fast         | 20 MB  | Fastest  | Good        | 200 MB | 100 MB  |
| Balanced     | 250 MB | Fast     | Excellent   | 500 MB | 250 MB  |
| Quality      | 800 MB | Moderate | Outstanding | 1.5 GB | 800 MB  |
| Multilingual | 250 MB | Fast     | Excellent   | 500 MB | 250 MB  |

### Configuration Examples

=== "Rust"

    ```rust title="embedding_basic.rs"
    use kreuzberg::core::{ExtractionConfig, ChunkingConfig, EmbeddingConfig, EmbeddingModelType};

    // Basic embedding with default balanced preset
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 1000,
            overlap: 200,
            embedding: Some(EmbeddingConfig::default()),
            preset: None,
        }),
        ..Default::default()
    };
    ```

=== "Rust - Custom Preset"

    ```rust title="embedding_preset.rs"
    use kreuzberg::core::{EmbeddingConfig, EmbeddingModelType};

    // Use fast preset for quick processing
    let config = EmbeddingConfig {
        model: EmbeddingModelType::Preset {
            name: "fast".to_string(),
        },
        normalize: true,
        batch_size: 16,
        show_download_progress: true,
        cache_dir: None,
    };

    // Use quality preset for best accuracy
    let config = EmbeddingConfig {
        model: EmbeddingModelType::Preset {
            name: "quality".to_string(),
        },
        batch_size: 32,
        ..Default::default()
    };

    // Use multilingual for international content
    let config = EmbeddingConfig {
        model: EmbeddingModelType::Preset {
            name: "multilingual".to_string(),
        },
        ..Default::default()
    };
    ```

=== "Rust - Custom ONNX Model"

    ```rust title="embedding_custom_onnx.rs"
    use kreuzberg::core::{EmbeddingConfig, EmbeddingModelType};

    // Explicit ONNX model specification
    let config = EmbeddingConfig {
        model: EmbeddingModelType::FastEmbed {
            model: "BGEBaseENV15".to_string(),
            dimensions: 768,
        },
        batch_size: 32,
        ..Default::default()
    };
    ```

=== "Rust - Custom Cache Directory"

    ```rust title="embedding_cache.rs"
    use kreuzberg::core::{EmbeddingConfig, EmbeddingModelType};
    use std::path::PathBuf;

    let config = EmbeddingConfig {
        model: EmbeddingModelType::Preset {
            name: "balanced".to_string(),
        },
        cache_dir: Some(PathBuf::from("/custom/models/cache")),
        show_download_progress: true,
        ..Default::default()
    };
    ```

### Configuration File Examples

#### TOML Format

```toml title="kreuzberg.toml"
[chunking]
max_characters = 1000
overlap = 200

# Use balanced preset (default)
[chunking.embedding]
model = { type = "preset", name = "balanced" }
batch_size = 32
normalize = true

# Or use fast preset
# [chunking.embedding]
# model = { type = "preset", name = "fast" }
# batch_size = 16

# Or use custom cache directory
# [chunking.embedding]
# model = { type = "preset", name = "quality" }
# cache_dir = "/data/models"
# show_download_progress = true
```

#### Token-Based Sizing (TOML)

```toml title="kreuzberg.toml"
[chunking]
max_chars = 512
max_overlap = 50

[chunking.sizing]
type = "tokenizer"
model = "Xenova/gpt-4o"
```

!!! Note
    Token-based sizing requires the `chunking-tokenizers` feature to be enabled.

#### YAML Format

```yaml title="kreuzberg.yaml"
chunking:
  max_characters: 1000
  overlap: 200
  embedding:
    model:
      type: preset
      name: balanced
    batch_size: 32
    normalize: true
```

#### JSON Format

```json title="kreuzberg.json"
{
  "chunking": {
    "max_characters": 1000,
    "overlap": 200,
    "embedding": {
      "model": {
        "type": "preset",
        "name": "balanced"
      },
      "batch_size": 32,
      "normalize": true
    }
  }
}
```

---

## LlmConfig

Configuration for LLM provider connections used by structured extraction, VLM-based OCR, and provider-hosted embeddings. Uses [liter-llm](https://github.com/kreuzberg-dev/liter-llm) for provider-agnostic model access.

### Fields

| Field      | Type      | Default | Description                                                                                                                                     |
| ---------- | --------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `model`    | `String`  | —       | Model identifier in `provider/model-name` format (for example, `"openai/gpt-4o-mini"`, `"anthropic/claude-sonnet-4-20250514"`)                                |
| `api_key`  | `String?` | `None`  | API key for the provider. When `None`, falls back to provider-standard env vars (for example, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`)                   |
| `base_url` | `String?` | `None`  | Custom base URL for the provider API. When `None`, uses the provider's default endpoint. Useful for proxies or self-hosted API-compatible servers |

### Configuration Examples

=== "Rust"

    ```rust title="llm_config.rs"
    use kreuzberg::core::LlmConfig;

    // Minimal config (uses provider env var for API key)
    let config = LlmConfig {
        model: "openai/gpt-4o-mini".to_string(),
        api_key: None,
        base_url: None,
    };

    // Explicit API key and custom endpoint
    let config = LlmConfig {
        model: "openai/gpt-4o".to_string(),
        api_key: Some("sk-...".to_string()),
        base_url: Some("https://api.example.com".to_string()),
    };
    ```

=== "Python"

    ```python title="llm_config.py"
    config = {
        "model": "openai/gpt-4o-mini",
        "api_key": None,       # Falls back to OPENAI_API_KEY
        "base_url": None,
    }
    ```

=== "TypeScript"

    ```typescript title="llm_config.ts"
    const config: LlmConfig = {
      model: "openai/gpt-4o-mini",
      apiKey: undefined,     // Falls back to OPENAI_API_KEY
      baseUrl: undefined,
    };
    ```

=== "Go"

    ```go title="llm_config.go"
    config := kreuzberg.LlmConfig{
        Model:   "openai/gpt-4o-mini",
        ApiKey:  nil,  // Falls back to OPENAI_API_KEY
        BaseUrl: nil,
    }
    ```

### Configuration File Examples

```toml title="kreuzberg.toml"
[llm]
model = "openai/gpt-4o-mini"
# api_key = "sk-..."       # Optional: falls back to OPENAI_API_KEY
# base_url = "https://..."  # Optional: uses provider default
```

```yaml title="kreuzberg.yaml"
llm:
  model: openai/gpt-4o-mini
  # api_key: sk-...
  # base_url: https://...
```

---

## StructuredExtractionConfig

Configuration for LLM-powered structured data extraction. Enables extracting structured data from documents by providing a JSON schema that defines the expected output format. The LLM processes the document content and returns data conforming to the schema.

### Fields

| Field             | Type         | Default | Description                                                                                                                                                    |
| ----------------- | ------------ | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `llm`             | `LlmConfig`  | —       | LLM provider configuration for the structured extraction model                                                                                                |
| `schema`          | `JsonValue`  | —       | JSON Schema defining the expected output structure. Must be a valid JSON Schema object.                                                                        |
| `prompt`          | `String?`    | `None`  | Custom system prompt for structured extraction. Overrides the default prompt. Useful for domain-specific instructions.                                         |
| `max_tokens`      | `usize?`     | `None`  | Maximum tokens for LLM response. When `None`, uses the provider's default limit.                                                                               |
| `temperature`     | `f64?`       | `None`  | Sampling temperature (0.0-2.0). Lower values produce more deterministic output. When `None`, defaults to `0.0` for maximum consistency.                        |

### Configuration Examples

=== "Rust"

    ```rust title="structured_extraction.rs"
    use kreuzberg::core::{ExtractionConfig, StructuredExtractionConfig, LlmConfig};
    use serde_json::json;

    let config = ExtractionConfig {
        structured_extraction: Some(StructuredExtractionConfig {
            llm: LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                api_key: None,
                base_url: None,
            },
            schema: json!({
                "type": "object",
                "properties": {
                    "invoice_number": { "type": "string" },
                    "total_amount": { "type": "number" },
                    "line_items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "description": { "type": "string" },
                                "amount": { "type": "number" }
                            }
                        }
                    }
                },
                "required": ["invoice_number", "total_amount"]
            }),
            prompt: None,
            max_tokens: None,
            temperature: Some(0.0),
        }),
        ..Default::default()
    };
    ```

=== "Python"

    ```python title="structured_extraction.py"
    config = {
        "structured_extraction": {
            "llm": {
                "model": "openai/gpt-4o-mini",
            },
            "schema": {
                "type": "object",
                "properties": {
                    "invoice_number": {"type": "string"},
                    "total_amount": {"type": "number"},
                    "line_items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "description": {"type": "string"},
                                "amount": {"type": "number"},
                            },
                        },
                    },
                },
                "required": ["invoice_number", "total_amount"],
            },
            "temperature": 0.0,
        },
    }
    ```

=== "TypeScript"

    ```typescript title="structured_extraction.ts"
    const config: ExtractionConfig = {
      structuredExtraction: {
        llm: {
          model: "openai/gpt-4o-mini",
        },
        schema: {
          type: "object",
          properties: {
            invoice_number: { type: "string" },
            total_amount: { type: "number" },
            line_items: {
              type: "array",
              items: {
                type: "object",
                properties: {
                  description: { type: "string" },
                  amount: { type: "number" },
                },
              },
            },
          },
          required: ["invoice_number", "total_amount"],
        },
        temperature: 0.0,
      },
    };
    ```

### Configuration File Examples

```toml title="kreuzberg.toml"
[structured_extraction]
prompt = "Extract invoice data from the document."
max_tokens = 4096
temperature = 0.0

[structured_extraction.llm]
model = "openai/gpt-4o-mini"

[structured_extraction.schema]
type = "object"

[structured_extraction.schema.properties.invoice_number]
type = "string"

[structured_extraction.schema.properties.total_amount]
type = "number"
```

```yaml title="kreuzberg.yaml"
structured_extraction:
  llm:
    model: openai/gpt-4o-mini
  schema:
    type: object
    properties:
      invoice_number:
        type: string
      total_amount:
        type: number
    required:
      - invoice_number
      - total_amount
  temperature: 0.0
```

---

## EmailConfig

Configuration for `.msg` (Outlook/MAPI) and `.eml` email file extraction. Controls how legacy Windows codepage encodings are handled when reading email headers and bodies that lack explicit character set declarations.

### Overview

Many older email messages — particularly those created by Microsoft Outlook on Windows — encode text using a Windows code page rather than UTF-8. When no charset is declared in the message headers, Kreuzberg defaults to Windows-1252 (Western European). Use `msg_fallback_codepage` to override this default for mailboxes that predominantly contain messages in a different encoding.

### Fields

| Field                  | Type      | Default                | Description                                                                                      |
| ---------------------- | --------- | ---------------------- | ------------------------------------------------------------------------------------------------ |
| `msg_fallback_codepage` | `int?`   | `None` (Windows-1252) | Windows code page number used when no charset is declared in the message. `None` = use 1252.    |

### Common Codepage Values

| Code Page | Encoding                       | Region / Language              |
| --------- | ------------------------------ | ------------------------------ |
| `1250`    | Windows Central European       | Polish, Czech, Hungarian, and so on. |
| `1251`    | Windows Cyrillic               | Russian, Ukrainian, Bulgarian  |
| `1252`    | Windows Western European       | English, German, French (default) |
| `1253`    | Windows Greek                  | Greek                          |
| `1254`    | Windows Turkish                | Turkish                        |
| `1255`    | Windows Hebrew                 | Hebrew                         |
| `1256`    | Windows Arabic                 | Arabic                         |
| `932`     | Shift-JIS                      | Japanese                       |
| `936`     | GBK (Simplified Chinese)       | Simplified Chinese             |

### Configuration Examples

=== "Python"

    ```python title="email_config.py"
    from kreuzberg import ExtractionConfig, PdfConfig
    from kreuzberg.email import EmailConfig

    # Extract a Russian Outlook .msg file with Cyrillic encoding
    config = ExtractionConfig(
        pdf_options=PdfConfig(
            email=EmailConfig(msg_fallback_codepage=1251)
        )
    )
    ```

=== "TypeScript"

    ```typescript title="email_config.ts"
    import { extract } from "kreuzberg";

    // Extract a Japanese .msg file encoded in Shift-JIS
    const result = await extract("message.msg", {
      pdfOptions: {
        email: { msgFallbackCodepage: 932 },
      },
    });
    ```

=== "Rust"

    ```rust title="email_config.rs"
    use kreuzberg::core::{ExtractionConfig, PdfConfig, EmailConfig};

    // Extract a Central European .msg file
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            email: Some(EmailConfig {
                msg_fallback_codepage: Some(1250),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

---

## LanguageDetectionConfig

Configuration for automatic language detection in extracted text.

| Field             | Type    | Default | Description                                                             |
| ----------------- | ------- | ------- | ----------------------------------------------------------------------- |
| `enabled`         | `bool`  | `true`  | Enable language detection                                               |
| `min_confidence`  | `float` | `0.8`   | Minimum confidence threshold (0.0-1.0) for reporting detected languages |
| `detect_multiple` | `bool`  | `false` | Detect multiple languages (vs. dominant language only)                  |

### Example

=== "C#"

    --8<-- "snippets/csharp/language_detection.md"

=== "Go"

    --8<-- "snippets/go/config/language_detection_config.md"

=== "Java"

    --8<-- "snippets/java/config/language_detection_config.md"

=== "Python"

    --8<-- "snippets/python/config/language_detection_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/language_detection_config.md"

=== "R"

    --8<-- "snippets/r/config/language_detection_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/language_detection_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/language_detection_config.md"

---

## KeywordConfig

Configuration for automatic keyword extraction from document text using YAKE or RAKE algorithms.

**Feature Gate**: Requires either `keywords-yake` or `keywords-rake` Cargo feature. Keyword extraction is only available when at least one of these features is enabled.

### Overview

Keyword extraction automatically identifies important terms and phrases in extracted text without manual labeling. Two algorithms are available:

- **YAKE**: Statistical approach based on term frequency and co-occurrence analysis
- **RAKE**: Rapid Automatic Keyword Extraction using word co-occurrence and frequency

Both algorithms analyze text independently and require no external training data, making them suitable for documents in any domain.

### Configuration Fields

| Field          | Type                 | Default               | Description                                                                              |
| -------------- | -------------------- | --------------------- | ---------------------------------------------------------------------------------------- |
| `algorithm`    | `KeywordAlgorithm`   | `Yake` (if available) | Algorithm to use: `yake` or `rake`                                                       |
| `max_keywords` | `usize`              | `10`                  | Maximum number of keywords to extract                                                    |
| `min_score`    | `f32`                | `0.0`                 | Minimum score threshold (0.0-1.0) for keyword filtering                                  |
| `ngram_range`  | `(usize, usize)`     | `(1, 3)`              | N-gram range: (min, max) words per keyword phrase                                        |
| `language`     | `Option<String>`     | `Some("en")`          | Language code for stopword filtering (for example, "en", "de", "fr"), `None` disables filtering |
| `yake_params`  | `Option<YakeParams>` | `None`                | YAKE-specific tuning parameters                                                          |
| `rake_params`  | `Option<RakeParams>` | `None`                | RAKE-specific tuning parameters                                                          |

### Algorithm Comparison

#### YAKE (Yet Another Keyword Extractor)

**Approach**: Statistical scoring based on term statistics and co-occurrence patterns.

| Aspect          | Details                                                             |
| --------------- | ------------------------------------------------------------------- |
| **Best For**    | General-purpose documents, balanced keyword distribution            |
| **Strengths**   | No training required, handles rare terms well, language-independent |
| **Limitations** | May extract very common terms, single-word focus                    |
| **Score Range** | 0.0-1.0 (lower scores = more relevant)                              |
| **Tuning**      | `window_size` (default: 2) - context window for co-occurrence       |
| **Use Cases**   | Research papers, news articles, general text                        |

**Characteristic**: YAKE assigns lower scores to more relevant keywords, so use higher `min_score` to be more selective.

#### RAKE (Rapid Automatic Keyword Extraction)

**Approach**: Co-occurrence graph analysis separating keywords by frequent stop words.

| Aspect          | Details                                                                 |
| --------------- | ----------------------------------------------------------------------- |
| **Best For**    | Multi-word phrases, domain-specific terminology                         |
| **Strengths**   | Excellent for extracting multi-word phrases, fast, domain-aware         |
| **Limitations** | Requires good stopword list, less effective with poorly structured text |
| **Score Range** | 0.0+ (higher scores = more relevant, unbounded)                         |
| **Tuning**      | `min_word_length`, `max_words_per_phrase`                               |
| **Use Cases**   | Technical documentation, scientific papers, product descriptions        |

**Characteristic**: RAKE assigns higher scores to more relevant keywords, so use lower `min_score` thresholds.

### N-gram Range Explanation

The `ngram_range` parameter controls the size of keyword phrases:

```text
ngram_range: (1, 1)  → Single words only: "python", "machine", "learning"
ngram_range: (1, 2)  → 1-2 word phrases: "python", "machine learning", "deep learning"
ngram_range: (1, 3)  → 1-3 word phrases: "python", "machine learning", "deep neural networks"
ngram_range: (2, 3)  → 2-3 word phrases only: "machine learning", "neural networks"
```

**Recommendations**:

- Use `(1, 1)` for single-word indexing (tagging, classification)
- Use `(1, 2)` for balanced coverage of terms and phrases
- Use `(1, 3)` for comprehensive phrase extraction (default)
- Use `(2, 3)` if you only want multi-word phrases

### Keyword Output Format

Keywords are returned as a list of `Keyword` structures in the extraction result:

```json title="Keyword Output Structure"
{
  "text": "machine learning",
  "score": 0.85,
  "algorithm": "yake",
  "positions": [42, 156, 203]
}
```

**Fields**:

- `text`: The keyword or phrase text
- `score`: Relevance score (algorithm-specific range and meaning)
- `algorithm`: Which algorithm extracted this keyword
- `positions`: Optional character offsets where the keyword appears in text

### Example: YAKE Configuration

=== "C#"

    ```csharp
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        Keywords = new KeywordConfig
        {
            Algorithm = KeywordAlgorithm.Yake,
            MaxKeywords = 10,
            MinScore = 0.3,
            NgramRange = (1, 3),
            Language = "en"
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Go"

    ```go
    config := &ExtractionConfig{
        Keywords: &KeywordConfig{
            Algorithm:   KeywordAlgorithm.Yake,
            MaxKeywords: 10,
            MinScore:    0.3,
            NgramRange:  [2]uint32{1, 3},
            Language:    "en",
        },
    }
    ```

=== "Java"

    ```java
    var config = ExtractionConfig.builder()
        .keywords(KeywordConfig.builder()
            .algorithm(KeywordAlgorithm.YAKE)
            .maxKeywords(10)
            .minScore(0.3f)
            .ngramRange(1, 3)
            .language("en")
            .build())
        .build();
    ```

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm

    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3,
            ngram_range=(1, 3),
            language="en"
        )
    )
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      keywords: Kreuzberg::KeywordConfig.new(
        algorithm: :yake,
        max_keywords: 10,
        min_score: 0.3,
        ngram_range: [1, 3],
        language: "en"
      )
    )
    ```

=== "Rust"

    --8<-- "snippets/rust/advanced/keyword_extraction_config.md"

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, KeywordConfig, KeywordAlgorithm } from 'kreuzberg';

    const config: ExtractionConfig = {
      keywords: {
        algorithm: KeywordAlgorithm.Yake,
        maxKeywords: 10,
        minScore: 0.3,
        ngramRange: [1, 3],
        language: "en"
      }
    };
    ```

### Example: RAKE Configuration with Multi-word Phrases

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm, RakeParams

    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.RAKE,
            max_keywords=15,
            min_score=0.1,
            ngram_range=(1, 4),
            language="en",
            rake_params=RakeParams(
                min_word_length=2,
                max_words_per_phrase=4
            )
        )
    )
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, KeywordConfig, KeywordAlgorithm, RakeParams};

    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Rake,
            max_keywords: 15,
            min_score: 0.1,
            ngram_range: (1, 4),
            language: Some("en".to_string()),
            rake_params: Some(RakeParams {
                min_word_length: 2,
                max_words_per_phrase: 4,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

### Language Support

Stopword filtering is applied when a language is specified. Common supported languages:

- `en` - English
- `es` - Spanish
- `fr` - French
- `de` - German
- `pt` - Portuguese
- `it` - Italian
- `ru` - Russian
- `ja` - Japanese
- `zh` - Chinese
- `ar` - Arabic

Set `language: None` to disable stopword filtering and extract keywords in any language without filtering.

---

## KeywordConfig

Configuration for automatic keyword extraction from document text using YAKE or RAKE algorithms.

**Feature Gate**: Requires either `keywords-yake` or `keywords-rake` Cargo feature. Keyword extraction is only available when at least one of these features is enabled.

### Overview

Keyword extraction automatically identifies important terms and phrases in extracted text without manual labeling. Two algorithms are available:

- **YAKE**: Statistical approach based on term frequency and co-occurrence analysis
- **RAKE**: Rapid Automatic Keyword Extraction using word co-occurrence and frequency

Both algorithms analyze text independently and require no external training data, making them suitable for documents in any domain.

### Configuration Fields

| Field          | Type                 | Default               | Description                                                                              |
| -------------- | -------------------- | --------------------- | ---------------------------------------------------------------------------------------- |
| `algorithm`    | `KeywordAlgorithm`   | `Yake` (if available) | Algorithm to use: `yake` or `rake`                                                       |
| `max_keywords` | `usize`              | `10`                  | Maximum number of keywords to extract                                                    |
| `min_score`    | `f32`                | `0.0`                 | Minimum score threshold (0.0-1.0) for keyword filtering                                  |
| `ngram_range`  | `(usize, usize)`     | `(1, 3)`              | N-gram range: (min, max) words per keyword phrase                                        |
| `language`     | `Option<String>`     | `Some("en")`          | Language code for stopword filtering (for example, "en", "de", "fr"), `None` disables filtering |
| `yake_params`  | `Option<YakeParams>` | `None`                | YAKE-specific tuning parameters                                                          |
| `rake_params`  | `Option<RakeParams>` | `None`                | RAKE-specific tuning parameters                                                          |

### Algorithm Comparison

#### YAKE (Yet Another Keyword Extractor)

**Approach**: Statistical scoring based on term statistics and co-occurrence patterns.

| Aspect          | Details                                                             |
| --------------- | ------------------------------------------------------------------- |
| **Best For**    | General-purpose documents, balanced keyword distribution            |
| **Strengths**   | No training required, handles rare terms well, language-independent |
| **Limitations** | May extract very common terms, single-word focus                    |
| **Score Range** | 0.0-1.0 (lower scores = more relevant)                              |
| **Tuning**      | `window_size` (default: 2) - context window for co-occurrence       |
| **Use Cases**   | Research papers, news articles, general text                        |

**Characteristic**: YAKE assigns lower scores to more relevant keywords, so use higher `min_score` to be more selective.

#### RAKE (Rapid Automatic Keyword Extraction)

**Approach**: Co-occurrence graph analysis separating keywords by frequent stop words.

| Aspect          | Details                                                                 |
| --------------- | ----------------------------------------------------------------------- |
| **Best For**    | Multi-word phrases, domain-specific terminology                         |
| **Strengths**   | Excellent for extracting multi-word phrases, fast, domain-aware         |
| **Limitations** | Requires good stopword list, less effective with poorly structured text |
| **Score Range** | 0.0+ (higher scores = more relevant, unbounded)                         |
| **Tuning**      | `min_word_length`, `max_words_per_phrase`                               |
| **Use Cases**   | Technical documentation, scientific papers, product descriptions        |

**Characteristic**: RAKE assigns higher scores to more relevant keywords, so use lower `min_score` thresholds.

### N-gram Range Explanation

The `ngram_range` parameter controls the size of keyword phrases:

```text
ngram_range: (1, 1)  → Single words only: "python", "machine", "learning"
ngram_range: (1, 2)  → 1-2 word phrases: "python", "machine learning", "deep learning"
ngram_range: (1, 3)  → 1-3 word phrases: "python", "machine learning", "deep neural networks"
ngram_range: (2, 3)  → 2-3 word phrases only: "machine learning", "neural networks"
```

**Recommendations**:

- Use `(1, 1)` for single-word indexing (tagging, classification)
- Use `(1, 2)` for balanced coverage of terms and phrases
- Use `(1, 3)` for comprehensive phrase extraction (default)
- Use `(2, 3)` if you only want multi-word phrases

### Keyword Output Format

Keywords are returned as a list of `Keyword` structures in the extraction result:

```json title="Keyword Output Structure"
{
  "text": "machine learning",
  "score": 0.85,
  "algorithm": "yake",
  "positions": [42, 156, 203]
}
```

**Fields**:

- `text`: The keyword or phrase text
- `score`: Relevance score (algorithm-specific range and meaning)
- `algorithm`: Which algorithm extracted this keyword
- `positions`: Optional character offsets where the keyword appears in text

### Example: YAKE Configuration

=== "C#"

    ```csharp
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        Keywords = new KeywordConfig
        {
            Algorithm = KeywordAlgorithm.Yake,
            MaxKeywords = 10,
            MinScore = 0.3,
            NgramRange = (1, 3),
            Language = "en"
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Go"

    ```go
    config := &ExtractionConfig{
        Keywords: &KeywordConfig{
            Algorithm:   KeywordAlgorithm.Yake,
            MaxKeywords: 10,
            MinScore:    0.3,
            NgramRange:  [2]uint32{1, 3},
            Language:    "en",
        },
    }
    ```

=== "Java"

    ```java
    var config = ExtractionConfig.builder()
        .keywords(KeywordConfig.builder()
            .algorithm(KeywordAlgorithm.YAKE)
            .maxKeywords(10)
            .minScore(0.3f)
            .ngramRange(1, 3)
            .language("en")
            .build())
        .build();
    ```

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm

    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3,
            ngram_range=(1, 3),
            language="en"
        )
    )
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      keywords: Kreuzberg::KeywordConfig.new(
        algorithm: :yake,
        max_keywords: 10,
        min_score: 0.3,
        ngram_range: [1, 3],
        language: "en"
      )
    )
    ```

=== "Rust"

    --8<-- "snippets/rust/advanced/keyword_extraction_config.md"

=== "TypeScript"

    ```typescript
    import { ExtractionConfig, KeywordConfig, KeywordAlgorithm } from 'kreuzberg';

    const config: ExtractionConfig = {
      keywords: {
        algorithm: KeywordAlgorithm.Yake,
        maxKeywords: 10,
        minScore: 0.3,
        ngramRange: [1, 3],
        language: "en"
      }
    };
    ```

### Example: RAKE Configuration with Multi-word Phrases

=== "Python"

    ```python
    from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm, RakeParams

    config = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.RAKE,
            max_keywords=15,
            min_score=0.1,
            ngram_range=(1, 4),
            language="en",
            rake_params=RakeParams(
                min_word_length=2,
                max_words_per_phrase=4
            )
        )
    )
    ```

=== "Rust"

    ```rust
    use kreuzberg::{ExtractionConfig, KeywordConfig, KeywordAlgorithm, RakeParams};

    let config = ExtractionConfig {
        keywords: Some(KeywordConfig {
            algorithm: KeywordAlgorithm::Rake,
            max_keywords: 15,
            min_score: 0.1,
            ngram_range: (1, 4),
            language: Some("en".to_string()),
            rake_params: Some(RakeParams {
                min_word_length: 2,
                max_words_per_phrase: 4,
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

### Language Support

Stopword filtering is applied when a language is specified. Common supported languages:

- `en` - English
- `es` - Spanish
- `fr` - French
- `de` - German
- `pt` - Portuguese
- `it` - Italian
- `ru` - Russian
- `ja` - Japanese
- `zh` - Chinese
- `ar` - Arabic

Set `language: None` to disable stopword filtering and extract keywords in any language without filtering.

---

## PdfConfig

PDF-specific extraction configuration.

| Field                      | Type               | Default | Description                                                               |
| -------------------------- | ------------------ | ------- | ------------------------------------------------------------------------- |
| `extract_images`           | `bool`             | `false` | Extract embedded images from PDF pages                                    |
| `extract_metadata`         | `bool`             | `true`  | Extract PDF metadata (title, author, creation date, etc.)                 |
| `passwords`                | `list[str]?`       | `None`  | List of passwords to try for encrypted PDFs (tries in order)              |
| `hierarchy`                | `HierarchyConfig?` | `None`  | Hierarchy extraction configuration (None = hierarchy extraction disabled) |
| `allow_single_column_tables` <span class="version-badge">v4.5.0</span> | `bool`             | `false` | Relax min column count from 2-3 to 1, allowing single-column table extraction |

!!! Note "Bounding boxes require explicit opt-in"
    Element bounding box coordinates are **not** extracted by default. To enable them, set `pdf_options=PdfConfig(hierarchy=HierarchyConfig(enabled=True, include_bbox=True))`. Coordinates are currently only available for **text elements** (headings and body blocks) — table and image regions do not carry per-element bbox data from this path.

### Example

=== "C#"

    --8<-- "snippets/csharp/pdf_config.md"

=== "Go"

    --8<-- "snippets/go/config/pdf_config.md"

=== "Java"

    --8<-- "snippets/java/config/pdf_config.md"

=== "Python"

    --8<-- "snippets/python/config/pdf_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/pdf_config.md"

=== "R"

    --8<-- "snippets/r/config/pdf_config.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/pdf_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/pdf_config.md"

---

## HierarchyConfig

PDF document hierarchy extraction configuration for semantic text structure analysis.

### Overview

HierarchyConfig enables automatic extraction of document hierarchy levels (H1-H6) from PDF text by analyzing font size patterns. This is particularly useful for:

- Building semantic document representations for RAG (Retrieval Augmented Generation) systems
- Automatic table of contents extraction
- Document structure understanding and analysis
- Content organization and outlining

The hierarchy detection works by:

1. Extracting text blocks with font size metadata from the PDF
2. Performing K-means clustering on font sizes to identify distinct size groups
3. Mapping clusters to heading levels (h1-h6) and body text
4. Merging adjacent blocks with the same hierarchy level
5. Optionally including bounding box information for spatial awareness

### Fields

| Field                    | Type          | Default | Description                                                                                                 |
| ------------------------ | ------------- | ------- | ----------------------------------------------------------------------------------------------------------- |
| `enabled`                | `bool`        | `true`  | Enable hierarchy extraction                                                                                 |
| `k_clusters`             | `usize`       | `6`     | Number of font size clusters (1-7). Default 6 provides H1-H6 with body text                                 |
| `include_bbox`           | `bool`        | `true`  | Include bounding box coordinates in output                                                                  |
| `ocr_coverage_threshold` | `Option<f32>` | `None`  | Smart OCR triggering threshold (0.0-1.0). Triggers OCR if text blocks cover less than this fraction of page |

### How It Works

#### Font Size Extraction

Text blocks are extracted from PDFs with their precise font sizes. This metadata is preserved for analysis.

#### K-means Clustering

The font sizes are clustered using K-means algorithm with the specified number of clusters. Each cluster represents a distinct text hierarchy level, from largest fonts (headings) to smallest (body text).

**Cluster-to-Level Mapping:**

- For `k_clusters=6` (recommended): Creates 6 clusters → h1 (largest), h2, h3, h4, h5, body (smallest)
- For `k_clusters=3`: Fast mode with just h1, h3, body (minimal detail)
- For `k_clusters=7`: Maximum detail separating h1-h6 with distinct body text

#### Block Merging

Adjacent blocks with the same hierarchy level are merged to create logical content units. This merge process considers:

- Spatial proximity (vertical and horizontal distance)
- Bounding box overlap ratio
- Text flow direction

#### Output Structure

Each extracted block contains:

- Text content
- Font size (in points)
- Hierarchy level (h1-h6 or body)
- Optional bounding box (left, top, right, bottom in PDF units)

### Use Cases

#### Semantic Document Understanding

Extract hierarchical structure for understanding document semantics and building knowledge graphs:

```text
H1: Document Title
  H2: Section 1
    H3: Subsection 1.1
      Body text...
    H3: Subsection 1.2
      Body text...
  H2: Section 2
    H3: Subsection 2.1
```

#### Automatic Table of Contents Generation

Build dynamic table of contents from extracted hierarchy levels (h1-h3) for document navigation.

#### RAG System Optimization

Use hierarchy information to improve context retrieval by chunking at appropriate heading boundaries rather than arbitrary character counts. This preserves semantic relationships.

#### Document Analysis

Extract and analyze document structure programmatically for compliance checking, content validation, or metadata extraction.

### Configuration Examples

#### Basic Hierarchy Extraction

=== "C#"

    ```csharp title="basic_hierarchy.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

    // Access hierarchy from pages
    if (result.Pages != null)
    {
        foreach (var page in result.Pages)
        {
            if (page.Hierarchy != null)
            {
                Console.WriteLine($"Page {page.PageNumber}: {page.Hierarchy.BlockCount} blocks");
                foreach (var block in page.Hierarchy.Blocks)
                {
                    Console.WriteLine($"  [{block.Level}] {block.Text.Substring(0, 50)}...");
                }
            }
        }
    }
    ```

=== "Go"

    ```go title="basic_hierarchy.go"
    package main

    import (
        "fmt"
        "kreuzberg"
    )

    func main() {
        config := &kreuzberg.ExtractionConfig{
            PdfOptions: &kreuzberg.PdfConfig{
                Hierarchy: &kreuzberg.HierarchyConfig{
                    Enabled: true,
                },
            },
        }

        result, err := kreuzberg.ExtractFileSync("document.pdf", config)
        if err != nil {
            panic(err)
        }

        if result.Pages != nil {
            for _, page := range result.Pages {
                if page.Hierarchy != nil {
                    fmt.Printf("Page %d: %d blocks\n", page.PageNumber, page.Hierarchy.BlockCount)
                    for _, block := range page.Hierarchy.Blocks {
                        fmt.Printf("  [%s] %s...\n", block.Level, block.Text[:50])
                    }
                }
            }
        }
    }
    ```

=== "Java"

    ```java title="BasicHierarchy.java"
    import com.kreuzberg.*;

    public class BasicHierarchy {
        public static void main(String[] args) throws Exception {
            ExtractionConfig config = ExtractionConfig.builder()
                .pdfOptions(PdfConfig.builder()
                    .hierarchy(HierarchyConfig.builder()
                        .enabled(true)
                        .build())
                    .build())
                .build();

            ExtractionResult result = KreuzbergClient.extractFileSync("document.pdf", config);

            if (result.getPages() != null) {
                for (PageContent page : result.getPages()) {
                    if (page.getHierarchy() != null) {
                        System.out.println("Page " + page.getPageNumber() + ": " +
                            page.getHierarchy().getBlockCount() + " blocks");
                        for (HierarchicalBlock block : page.getHierarchy().getBlocks()) {
                            System.out.println("  [" + block.getLevel() + "] " +
                                block.getText().substring(0, 50) + "...");
                        }
                    }
                }
            }
        }
    }
    ```

=== "Python"

    --8<-- "snippets/python/config/pdf_hierarchy_config.md"

=== "Ruby"

    ```ruby title="basic_hierarchy.rb"
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      pdf_options: Kreuzberg::PdfConfig.new(
        hierarchy: Kreuzberg::HierarchyConfig.new(
          enabled: true
        )
      )
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    if result.pages
      result.pages.each do |page|
        if page.hierarchy
          puts "Page #{page.page_number}: #{page.hierarchy.block_count} blocks"
          page.hierarchy.blocks.each do |block|
            puts "  [#{block.level}] #{block.text[0..49]}..."
          end
        end
      end
    end
    ```

=== "Rust"

    --8<-- "snippets/rust/config/pdf_hierarchy_config.md"

=== "TypeScript"

    ```typescript title="basic_hierarchy.ts"
    import { extractFileSync, ExtractionConfig, PdfConfig, HierarchyConfig } from 'kreuzberg';

    const config: ExtractionConfig = {
        pdfOptions: {
            hierarchy: {
                enabled: true
            }
        }
    };

    const result = extractFileSync("document.pdf", config);

    if (result.pages) {
        for (const page of result.pages) {
            if (page.hierarchy) {
                console.log(`Page ${page.pageNumber}: ${page.hierarchy.blockCount} blocks`);
                for (const block of page.hierarchy.blocks) {
                    console.log(`  [${block.level}] ${block.text.substring(0, 50)}...`);
                }
            }
        }
    }
    ```

#### Custom K-Clusters Configuration

Configure clustering granularity for different hierarchy detail levels:

=== "C#"

    ```csharp title="custom_k_clusters.cs"
    using Kreuzberg;

    // Fast mode: 3 clusters (h1, h3, body) - minimal detail
    var fastConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 3  // Fast, identifies main structure only
            }
        }
    };

    // Balanced mode: 6 clusters (h1-h6) - default, recommended
    var balancedConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 6  // Balanced detail
            }
        }
    };

    // Detailed mode: 7 clusters (h1-h6 + distinct body) - maximum detail
    var detailedConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 7  // Maximum detail with body text separation
            }
        }
    };
    ```

=== "Python"

    ```python title="custom_k_clusters.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    # Fast mode: 3 clusters
    fast_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=3  # Fast, identifies main structure only
            )
        )
    )

    # Balanced mode: 6 clusters (recommended)
    balanced_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=6  # Balanced detail
            )
        )
    )

    # Detailed mode: 7 clusters
    detailed_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=7  # Maximum detail with body text separation
            )
        )
    )

    result = extract_file_sync("document.pdf", config=balanced_config)
    ```

=== "Rust"

    ```rust title="custom_k_clusters.rs"
    use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

    fn main() -> kreuzberg::Result<()> {
        // Fast mode: 3 clusters
        let fast_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 3,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Balanced mode: 6 clusters (recommended)
        let balanced_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 6,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Detailed mode: 7 clusters
        let detailed_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 7,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None::<&str>, &balanced_config)?;
        Ok(())
    }
    ```

#### OCR Coverage Threshold

Smart OCR triggering based on text coverage:

=== "C#"

    ```csharp title="ocr_coverage_threshold.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                OcrCoverageThreshold = 0.5f  // Trigger OCR if <50% of page has text
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Python"

    ```python title="ocr_coverage_threshold.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                ocr_coverage_threshold=0.5  # Trigger OCR if <50% of page has text
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

=== "Rust"

    ```rust title="ocr_coverage_threshold.rs"
    use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

    fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    ocr_coverage_threshold: Some(0.5),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None::<&str>, &config)?;
        Ok(())
    }
    ```

#### Disabling Bounding Boxes

Reduce output size by excluding spatial information:

=== "C#"

    ```csharp title="no_bbox.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                IncludeBbox = false  // Exclude bounding boxes
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Python"

    ```python title="no_bbox.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                include_bbox=False  // Exclude bounding boxes
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

### Performance Tuning

#### K-clusters Selection

Choose k_clusters based on your performance vs. detail requirements:

| Setting        | Speed     | Detail                         | Best For                                                      |
| -------------- | --------- | ------------------------------ | ------------------------------------------------------------- |
| `k_clusters=3` | Very Fast | Minimal (h1, h3, body)         | Quick document structure identification, real-time processing |
| `k_clusters=6` | Balanced  | Standard (h1-h6, body)         | General purpose, RAG systems, recommended default             |
| `k_clusters=7` | Moderate  | Detailed (h1-h6 separate body) | Fine-grained content analysis, content organization           |

#### Bounding Box Optimization

**Include bounding boxes** (`include_bbox=true`, default) when:

- Building visually-aware document processors
- Need to correlate text with document position
- Processing layout-sensitive documents (brochures, forms)

**Exclude bounding boxes** (`include_bbox=false`) when:

- Minimizing output size for network transmission
- Bandwidth is constrained
- Spatial information is not needed
- Typical output reduction: 10-15% smaller

#### OCR Integration

The `ocr_coverage_threshold` parameter enables smart OCR triggering:

````text

If (text_block_coverage < ocr_coverage_threshold) {
run_ocr() // Trigger OCR on pages with insufficient text coverage
}

````text

**Common Scenarios:**

- `ocr_coverage_threshold=0.5`: Trigger OCR on scanned pages (<50% text coverage)
- `ocr_coverage_threshold=0.8`: Only OCR pages with very low text (>80% images)
- `ocr_coverage_threshold=None`: Disable smart OCR triggering, rely on `force_ocr` flag

### Output Format

#### PageHierarchy Structure

The extracted hierarchy is returned in `PageContent.hierarchy` when pages are extracted:

```json title="PageHierarchy Output Structure"
{
  "block_count": 12,
  "blocks": [
    {
      "text": "Document Title",
      "font_size": 24.0,
      "level": "h1",
      "bbox": [50.0, 100.0, 500.0, 130.0]
    },
    {
      "text": "Introduction",
      "font_size": 18.0,
      "level": "h2",
      "bbox": [50.0, 150.0, 300.0, 175.0]
    },
    {
      "text": "This is the introductory paragraph with standard body text content.",
      "font_size": 12.0,
      "level": "body",
      "bbox": [50.0, 200.0, 500.0, 250.0]
    },
    {
      "text": "Key Findings",
      "font_size": 18.0,
      "level": "h2",
      "bbox": [50.0, 280.0, 300.0, 305.0]
    }
  ]
}
````

#### Field Meanings

- **block_count**: Total number of hierarchical blocks on the page
- **blocks**: Array of hierarchical blocks
  - **text**: The text content of the block
  - **font_size**: Font size in points (useful for verification and styling)
  - **level**: Hierarchy level - "h1" through "h6" for headings, "body" for body text
  - **bbox**: Optional bounding box as `[left, top, right, bottom]` in PDF units (points). Only present when `include_bbox=true`

#### Accessing Hierarchy in Code

=== "Python"

    ```python
    result = extract_file_sync("document.pdf", config=config)

    for page in result.pages or []:
        if page.hierarchy:
            # Get all h1 headings
            h1_blocks = [b for b in page.hierarchy.blocks if b.level == "h1"]

            # Get all heading levels (h1-h6)
            headings = [b for b in page.hierarchy.blocks if b.level.startswith("h")]

            # Build outline with hierarchy
            for block in page.hierarchy.blocks:
                indent = int(block.level[1]) if block.level.startswith("h") else 0
                print("  " * indent + block.text)
    ```

=== "Rust"

    ```rust
    for page in result.pages.iter().flat_map(|p| p.iter()) {
        if let Some(hierarchy) = &page.hierarchy {
            // Get all h1 headings
            let h1_blocks: Vec<_> = hierarchy.blocks
                .iter()
                .filter(|b| b.level == "h1")
                .collect();

            // Build outline
            for block in &hierarchy.blocks {
                let level = if block.level.starts_with('h') {
                    block.level[1..].parse::<usize>().unwrap_or(0)
                } else {
                    0
                };
                println!("{}{}", "  ".repeat(level), block.text);
            }
        }
    }
    ```

### Best Practices

1. **Always enable page extraction** when using hierarchy:

   ```text
   pages = PageConfig(extract_pages=True)
   ```

   Hierarchy data is only populated when pages are extracted.

2. **Use k_clusters=6 by default** (recommended). It provides good balance between detail and performance for most documents.

3. **Include bounding boxes for RAG systems** that need spatial awareness for relevance ranking.

4. **Test ocr_coverage_threshold** with your document set to find optimal OCR triggering point.

5. **Process hierarchy at chunk boundaries** in RAG systems to preserve semantic relationships in context windows.

### Example: Building a Table of Contents

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig, PageConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(enabled=True, k_clusters=6)
        ),
        pages=PageConfig(extract_pages=True)
    )

    result = extract_file_sync("document.pdf", config=config)

    toc = []
    for page in result.pages or []:
        if page.hierarchy:
            for block in page.hierarchy.blocks:
                if block.level.startswith("h"):
                    level = int(block.level[1])
                    toc.append({
                        "level": level,
                        "text": block.text,
                        "page": page.page_number
                    })

    # Print hierarchical TOC
    for entry in toc:
        indent = "  " * (entry["level"] - 1)
        print(f"{indent}{entry['text']} (p. {entry['page']})")
    ```

---

## HierarchyConfig

PDF document hierarchy extraction configuration for semantic text structure analysis.

### Overview

HierarchyConfig enables automatic extraction of document hierarchy levels (H1-H6) from PDF text by analyzing font size patterns. This is particularly useful for:

- Building semantic document representations for RAG (Retrieval Augmented Generation) systems
- Automatic table of contents extraction
- Document structure understanding and analysis
- Content organization and outlining

The hierarchy detection works by:

1. Extracting text blocks with font size metadata from the PDF
2. Performing K-means clustering on font sizes to identify distinct size groups
3. Mapping clusters to heading levels (h1-h6) and body text
4. Merging adjacent blocks with the same hierarchy level
5. Optionally including bounding box information for spatial awareness

### Fields

| Field                    | Type          | Default | Description                                                                                                 |
| ------------------------ | ------------- | ------- | ----------------------------------------------------------------------------------------------------------- |
| `enabled`                | `bool`        | `true`  | Enable hierarchy extraction                                                                                 |
| `k_clusters`             | `usize`       | `6`     | Number of font size clusters (1-7). Default 6 provides H1-H6 with body text                                 |
| `include_bbox`           | `bool`        | `true`  | Include bounding box coordinates in output                                                                  |
| `ocr_coverage_threshold` | `Option<f32>` | `None`  | Smart OCR triggering threshold (0.0-1.0). Triggers OCR if text blocks cover less than this fraction of page |

### How It Works

#### Font Size Extraction

Text blocks are extracted from PDFs with their precise font sizes. This metadata is preserved for analysis.

#### K-means Clustering

The font sizes are clustered using K-means algorithm with the specified number of clusters. Each cluster represents a distinct text hierarchy level, from largest fonts (headings) to smallest (body text).

**Cluster-to-Level Mapping:**

- For `k_clusters=6` (recommended): Creates 6 clusters → h1 (largest), h2, h3, h4, h5, body (smallest)
- For `k_clusters=3`: Fast mode with just h1, h3, body (minimal detail)
- For `k_clusters=7`: Maximum detail separating h1-h6 with distinct body text

#### Block Merging

Adjacent blocks with the same hierarchy level are merged to create logical content units. This merge process considers:

- Spatial proximity (vertical and horizontal distance)
- Bounding box overlap ratio
- Text flow direction

#### Output Structure

Each extracted block contains:

- Text content
- Font size (in points)
- Hierarchy level (h1-h6 or body)
- Optional bounding box (left, top, right, bottom in PDF units)

### Use Cases

#### Semantic Document Understanding

Extract hierarchical structure for understanding document semantics and building knowledge graphs:

```text
H1: Document Title
  H2: Section 1
    H3: Subsection 1.1
      Body text...
    H3: Subsection 1.2
      Body text...
  H2: Section 2
    H3: Subsection 2.1
```

#### Automatic Table of Contents Generation

Build dynamic table of contents from extracted hierarchy levels (h1-h3) for document navigation.

#### RAG System Optimization

Use hierarchy information to improve context retrieval by chunking at appropriate heading boundaries rather than arbitrary character counts. This preserves semantic relationships.

#### Document Analysis

Extract and analyze document structure programmatically for compliance checking, content validation, or metadata extraction.

### Configuration Examples

#### Basic Hierarchy Extraction

=== "C#"

    ```csharp title="basic_hierarchy.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);

    // Access hierarchy from pages
    if (result.Pages != null)
    {
        foreach (var page in result.Pages)
        {
            if (page.Hierarchy != null)
            {
                Console.WriteLine($"Page {page.PageNumber}: {page.Hierarchy.BlockCount} blocks");
                foreach (var block in page.Hierarchy.Blocks)
                {
                    Console.WriteLine($"  [{block.Level}] {block.Text.Substring(0, 50)}...");
                }
            }
        }
    }
    ```

=== "Go"

    ```go title="basic_hierarchy.go"
    package main

    import (
        "fmt"
        "kreuzberg"
    )

    func main() {
        config := &kreuzberg.ExtractionConfig{
            PdfOptions: &kreuzberg.PdfConfig{
                Hierarchy: &kreuzberg.HierarchyConfig{
                    Enabled: true,
                },
            },
        }

        result, err := kreuzberg.ExtractFileSync("document.pdf", config)
        if err != nil {
            panic(err)
        }

        if result.Pages != nil {
            for _, page := range result.Pages {
                if page.Hierarchy != nil {
                    fmt.Printf("Page %d: %d blocks\n", page.PageNumber, page.Hierarchy.BlockCount)
                    for _, block := range page.Hierarchy.Blocks {
                        fmt.Printf("  [%s] %s...\n", block.Level, block.Text[:50])
                    }
                }
            }
        }
    }
    ```

=== "Java"

    ```java title="BasicHierarchy.java"
    import com.kreuzberg.*;

    public class BasicHierarchy {
        public static void main(String[] args) throws Exception {
            ExtractionConfig config = ExtractionConfig.builder()
                .pdfOptions(PdfConfig.builder()
                    .hierarchy(HierarchyConfig.builder()
                        .enabled(true)
                        .build())
                    .build())
                .build();

            ExtractionResult result = KreuzbergClient.extractFileSync("document.pdf", config);

            if (result.getPages() != null) {
                for (PageContent page : result.getPages()) {
                    if (page.getHierarchy() != null) {
                        System.out.println("Page " + page.getPageNumber() + ": " +
                            page.getHierarchy().getBlockCount() + " blocks");
                        for (HierarchicalBlock block : page.getHierarchy().getBlocks()) {
                            System.out.println("  [" + block.getLevel() + "] " +
                                block.getText().substring(0, 50) + "...");
                        }
                    }
                }
            }
        }
    }
    ```

=== "Python"

    --8<-- "snippets/python/config/pdf_hierarchy_config.md"

=== "Ruby"

    ```ruby title="basic_hierarchy.rb"
    require 'kreuzberg'

    config = Kreuzberg::ExtractionConfig.new(
      pdf_options: Kreuzberg::PdfConfig.new(
        hierarchy: Kreuzberg::HierarchyConfig.new(
          enabled: true
        )
      )
    )

    result = Kreuzberg.extract_file_sync("document.pdf", config: config)

    if result.pages
      result.pages.each do |page|
        if page.hierarchy
          puts "Page #{page.page_number}: #{page.hierarchy.block_count} blocks"
          page.hierarchy.blocks.each do |block|
            puts "  [#{block.level}] #{block.text[0..49]}..."
          end
        end
      end
    end
    ```

=== "Rust"

    --8<-- "snippets/rust/config/pdf_hierarchy_config.md"

=== "TypeScript"

    ```typescript title="basic_hierarchy.ts"
    import { extractFileSync, ExtractionConfig, PdfConfig, HierarchyConfig } from 'kreuzberg';

    const config: ExtractionConfig = {
        pdfOptions: {
            hierarchy: {
                enabled: true
            }
        }
    };

    const result = extractFileSync("document.pdf", config);

    if (result.pages) {
        for (const page of result.pages) {
            if (page.hierarchy) {
                console.log(`Page ${page.pageNumber}: ${page.hierarchy.blockCount} blocks`);
                for (const block of page.hierarchy.blocks) {
                    console.log(`  [${block.level}] ${block.text.substring(0, 50)}...`);
                }
            }
        }
    }
    ```

#### Custom K-Clusters Configuration

Configure clustering granularity for different hierarchy detail levels:

=== "C#"

    ```csharp title="custom_k_clusters.cs"
    using Kreuzberg;

    // Fast mode: 3 clusters (h1, h3, body) - minimal detail
    var fastConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 3  // Fast, identifies main structure only
            }
        }
    };

    // Balanced mode: 6 clusters (h1-h6) - default, recommended
    var balancedConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 6  // Balanced detail
            }
        }
    };

    // Detailed mode: 7 clusters (h1-h6 + distinct body) - maximum detail
    var detailedConfig = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                KClusters = 7  // Maximum detail with body text separation
            }
        }
    };
    ```

=== "Python"

    ```python title="custom_k_clusters.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    # Fast mode: 3 clusters
    fast_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=3  # Fast, identifies main structure only
            )
        )
    )

    # Balanced mode: 6 clusters (recommended)
    balanced_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=6  # Balanced detail
            )
        )
    )

    # Detailed mode: 7 clusters
    detailed_config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                k_clusters=7  # Maximum detail with body text separation
            )
        )
    )

    result = extract_file_sync("document.pdf", config=balanced_config)
    ```

=== "Rust"

    ```rust title="custom_k_clusters.rs"
    use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

    fn main() -> kreuzberg::Result<()> {
        // Fast mode: 3 clusters
        let fast_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 3,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Balanced mode: 6 clusters (recommended)
        let balanced_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 6,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Detailed mode: 7 clusters
        let detailed_config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    k_clusters: 7,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None::<&str>, &balanced_config)?;
        Ok(())
    }
    ```

#### OCR Coverage Threshold

Smart OCR triggering based on text coverage:

=== "C#"

    ```csharp title="ocr_coverage_threshold.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                OcrCoverageThreshold = 0.5f  // Trigger OCR if <50% of page has text
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Python"

    ```python title="ocr_coverage_threshold.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                ocr_coverage_threshold=0.5  # Trigger OCR if <50% of page has text
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

=== "Rust"

    ```rust title="ocr_coverage_threshold.rs"
    use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig};

    fn main() -> kreuzberg::Result<()> {
        let config = ExtractionConfig {
            pdf_options: Some(PdfConfig {
                hierarchy: Some(HierarchyConfig {
                    ocr_coverage_threshold: Some(0.5),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let result = extract_file_sync("document.pdf", None::<&str>, &config)?;
        Ok(())
    }
    ```

#### Disabling Bounding Boxes

Reduce output size by excluding spatial information:

=== "C#"

    ```csharp title="no_bbox.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        PdfOptions = new PdfConfig
        {
            Hierarchy = new HierarchyConfig
            {
                Enabled = true,
                IncludeBbox = false  // Exclude bounding boxes
            }
        }
    };

    var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
    ```

=== "Python"

    ```python title="no_bbox.py"
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(
                enabled=True,
                include_bbox=False  // Exclude bounding boxes
            )
        )
    )

    result = extract_file_sync("document.pdf", config=config)
    ```

### Performance Tuning

#### K-clusters Selection

Choose k_clusters based on your performance vs. detail requirements:

| Setting        | Speed     | Detail                         | Best For                                                      |
| -------------- | --------- | ------------------------------ | ------------------------------------------------------------- |
| `k_clusters=3` | Very Fast | Minimal (h1, h3, body)         | Quick document structure identification, real-time processing |
| `k_clusters=6` | Balanced  | Standard (h1-h6, body)         | General purpose, RAG systems, recommended default             |
| `k_clusters=7` | Moderate  | Detailed (h1-h6 separate body) | Fine-grained content analysis, content organization           |

#### Bounding Box Optimization

**Include bounding boxes** (`include_bbox=true`, default) when:

- Building visually-aware document processors
- Need to correlate text with document position
- Processing layout-sensitive documents (brochures, forms)

**Exclude bounding boxes** (`include_bbox=false`) when:

- Minimizing output size for network transmission
- Bandwidth is constrained
- Spatial information is not needed
- Typical output reduction: 10-15% smaller

#### OCR Integration

The `ocr_coverage_threshold` parameter enables smart OCR triggering:

````text

If (text_block_coverage < ocr_coverage_threshold) {
run_ocr() // Trigger OCR on pages with insufficient text coverage
}

````text

**Common Scenarios:**

- `ocr_coverage_threshold=0.5`: Trigger OCR on scanned pages (<50% text coverage)
- `ocr_coverage_threshold=0.8`: Only OCR pages with very low text (>80% images)
- `ocr_coverage_threshold=None`: Disable smart OCR triggering, rely on `force_ocr` flag

### Output Format

#### PageHierarchy Structure

The extracted hierarchy is returned in `PageContent.hierarchy` when pages are extracted:

```json title="PageHierarchy Output Structure"
{
  "block_count": 12,
  "blocks": [
    {
      "text": "Document Title",
      "font_size": 24.0,
      "level": "h1",
      "bbox": [50.0, 100.0, 500.0, 130.0]
    },
    {
      "text": "Introduction",
      "font_size": 18.0,
      "level": "h2",
      "bbox": [50.0, 150.0, 300.0, 175.0]
    },
    {
      "text": "This is the introductory paragraph with standard body text content.",
      "font_size": 12.0,
      "level": "body",
      "bbox": [50.0, 200.0, 500.0, 250.0]
    },
    {
      "text": "Key Findings",
      "font_size": 18.0,
      "level": "h2",
      "bbox": [50.0, 280.0, 300.0, 305.0]
    }
  ]
}
````

#### Field Meanings

- **block_count**: Total number of hierarchical blocks on the page
- **blocks**: Array of hierarchical blocks
  - **text**: The text content of the block
  - **font_size**: Font size in points (useful for verification and styling)
  - **level**: Hierarchy level - "h1" through "h6" for headings, "body" for body text
  - **bbox**: Optional bounding box as `[left, top, right, bottom]` in PDF units (points). Only present when `include_bbox=true`

#### Accessing Hierarchy in Code

=== "Python"

    ```python
    result = extract_file_sync("document.pdf", config=config)

    for page in result.pages or []:
        if page.hierarchy:
            # Get all h1 headings
            h1_blocks = [b for b in page.hierarchy.blocks if b.level == "h1"]

            # Get all heading levels (h1-h6)
            headings = [b for b in page.hierarchy.blocks if b.level.startswith("h")]

            # Build outline with hierarchy
            for block in page.hierarchy.blocks:
                indent = int(block.level[1]) if block.level.startswith("h") else 0
                print("  " * indent + block.text)
    ```

=== "Rust"

    ```rust
    for page in result.pages.iter().flat_map(|p| p.iter()) {
        if let Some(hierarchy) = &page.hierarchy {
            // Get all h1 headings
            let h1_blocks: Vec<_> = hierarchy.blocks
                .iter()
                .filter(|b| b.level == "h1")
                .collect();

            // Build outline
            for block in &hierarchy.blocks {
                let level = if block.level.starts_with('h') {
                    block.level[1..].parse::<usize>().unwrap_or(0)
                } else {
                    0
                };
                println!("{}{}", "  ".repeat(level), block.text);
            }
        }
    }
    ```

### Best Practices

1. **Always enable page extraction** when using hierarchy:

   ```text
   pages = PageConfig(extract_pages=True)
   ```

   Hierarchy data is only populated when pages are extracted.

2. **Use k_clusters=6 by default** (recommended). It provides good balance between detail and performance for most documents.

3. **Include bounding boxes for RAG systems** that need spatial awareness for relevance ranking.

4. **Test ocr_coverage_threshold** with your document set to find optimal OCR triggering point.

5. **Process hierarchy at chunk boundaries** in RAG systems to preserve semantic relationships in context windows.

### Example: Building a Table of Contents

=== "Python"

    ```python
    from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig, HierarchyConfig, PageConfig

    config = ExtractionConfig(
        pdf_options=PdfConfig(
            hierarchy=HierarchyConfig(enabled=True, k_clusters=6)
        ),
        pages=PageConfig(extract_pages=True)
    )

    result = extract_file_sync("document.pdf", config=config)

    toc = []
    for page in result.pages or []:
        if page.hierarchy:
            for block in page.hierarchy.blocks:
                if block.level.startswith("h"):
                    level = int(block.level[1])
                    toc.append({
                        "level": level,
                        "text": block.text,
                        "page": page.page_number
                    })

    # Print hierarchical TOC
    for entry in toc:
        indent = "  " * (entry["level"] - 1)
        print(f"{indent}{entry['text']} (p. {entry['page']})")
    ```

---

## PageConfig

Configuration for page extraction and tracking.

Controls whether to extract per-page content and how to mark page boundaries in the combined text output.

### Configuration

| Field                 | Type     | Default                              | Description                                              |
| --------------------- | -------- | ------------------------------------ | -------------------------------------------------------- |
| `extract_pages`       | `bool`   | `false`                              | Extract pages as separate array in results               |
| `insert_page_markers` | `bool`   | `false`                              | Insert page markers in combined content string           |
| `marker_format`       | `String` | `"\n\n<!-- PAGE {page_num} -->\n\n"` | Template for page markers (use `{page_num}` placeholder) |

### Example

=== "C#"

    ```csharp title="page_config.cs"
    var config = new ExtractionConfig
    {
        Pages = new PageConfig
        {
            ExtractPages = true,
            InsertPageMarkers = true,
            MarkerFormat = "\n\n--- Page {page_num} ---\n\n"
        }
    };
    ```

=== "Go"

    ```go title="page_config.go"
    config := &ExtractionConfig{
        Pages: &PageConfig{
            ExtractPages:      true,
            InsertPageMarkers: true,
            MarkerFormat:      "\n\n--- Page {page_num} ---\n\n",
        },
    }
    ```

=== "Java"

    ```java title="PageConfig.java"
    var config = ExtractionConfig.builder()
        .pages(PageConfig.builder()
            .extractPages(true)
            .insertPageMarkers(true)
            .markerFormat("\n\n--- Page {page_num} ---\n\n")
            .build())
        .build();
    ```

=== "Python"

    ```python title="page_config.py"
    config = ExtractionConfig(
        pages=PageConfig(
            extract_pages=True,
            insert_page_markers=True,
            marker_format="\n\n--- Page {page_num} ---\n\n"
        )
    )
    ```

=== "Ruby"

    ```ruby title="page_config.rb"
    config = ExtractionConfig.new(
      pages: PageConfig.new(
        extract_pages: true,
        insert_page_markers: true,
        marker_format: "\n\n--- Page {page_num} ---\n\n"
      )
    )
    ```

=== "Rust"

    ```rust title="page_config.rs"
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            marker_format: "\n\n--- Page {page_num} ---\n\n".to_string(),
        }),
        ..Default::default()
    };
    ```

=== "TypeScript"

    ```typescript title="page_config.ts"
    const config: ExtractionConfig = {
      pages: {
        extractPages: true,
        insertPageMarkers: true,
        markerFormat: "\n\n--- Page {page_num} ---\n\n"
      }
    };
    ```

### Field Details

**extract_pages**: When `true`, populates `ExtractionResult.pages` with per-page content. Each page contains its text, tables, and images separately.

**insert_page_markers**: When `true`, inserts page markers into the combined `content` string at page boundaries. Useful for LLMs to understand document structure.

**marker_format**: Template string for page markers. Use `{page_num}` placeholder for the page number. Default HTML comment format is LLM-friendly.

### Format Support

- **PDF**: Full byte-accurate page tracking with O(1) lookup performance
- **PPTX**: Slide boundary tracking with per-slide content
- **DOCX**: Best-effort page break detection using explicit page breaks
- **Other formats**: Page tracking not available (returns `None`/`null`)

---

## ImageExtractionConfig

Configuration for extracting and processing images from documents.

| Field                 | Type   | Default | Description                                              |
| --------------------- | ------ | ------- | -------------------------------------------------------- |
| `extract_images`        | `bool` | `true`  | Extract images from documents                            |
| `target_dpi`            | `int`  | `300`   | Target DPI for extracted/normalized images               |
| `max_image_dimension`   | `int`  | `4096`  | Maximum image dimension (width or height) in pixels      |
| `inject_placeholders`   | `bool` | `true`  | Inject image reference placeholders (for example `![Image](embedded:p1_i0)`) into markdown output. Set to `false` to extract images as data without modifying the text content. |
| `auto_adjust_dpi`       | `bool` | `true`  | Automatically adjust DPI based on image size and content |
| `min_dpi`               | `int`  | `72`    | Minimum DPI when auto-adjusting                          |
| `max_dpi`               | `int`  | `600`   | Maximum DPI when auto-adjusting                          |

### Example

=== "C#"

    --8<-- "snippets/csharp/image_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr/image_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr/image_extraction.md"

=== "Python"

    --8<-- "snippets/python/utils/image_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/image_extraction.md"

=== "R"

    --8<-- "snippets/r/ocr/image_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/image_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/image_extraction.md"

---

## ImagePreprocessingConfig

Image preprocessing configuration for improving OCR quality on scanned documents.

| Field                 | Type   | Default  | Description                                                        |
| --------------------- | ------ | -------- | ------------------------------------------------------------------ |
| `target_dpi`          | `int`  | `300`    | Target DPI for OCR processing (300 standard, 600 for small text)   |
| `auto_rotate`         | `bool` | `true`   | Auto-detect and correct image rotation                             |
| `deskew`              | `bool` | `true`   | Correct skew (tilted images)                                       |
| `denoise`             | `bool` | `false`  | Apply noise reduction filter                                       |
| `contrast_enhance`    | `bool` | `false`  | Enhance image contrast for better text visibility                  |
| `binarization_method` | `str`  | `"otsu"` | Binarization method: `"otsu"`, `"sauvola"`, `"adaptive"`, `"none"` |
| `invert_colors`       | `bool` | `false`  | Invert colors (useful for white text on black background)          |

### Example

=== "C#"

    --8<-- "snippets/csharp/image_preprocessing.md"

=== "Go"

    --8<-- "snippets/go/ocr/image_preprocessing.md"

=== "Java"

    --8<-- "snippets/java/ocr/image_preprocessing.md"

=== "Python"

    --8<-- "snippets/python/utils/image_preprocessing.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/image_preprocessing.md"

=== "R"

    --8<-- "snippets/r/ocr/image_preprocessing.md"

=== "Rust"

    --8<-- "snippets/rust/metadata/image_preprocessing.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/image_preprocessing.md"

---

## PostProcessorConfig

Configuration for the post-processing pipeline that runs after extraction.

| Field                 | Type         | Default | Description                                                               |
| --------------------- | ------------ | ------- | ------------------------------------------------------------------------- |
| `enabled`             | `bool`       | `true`  | Enable post-processing pipeline                                           |
| `enabled_processors`  | `list[str]?` | `None`  | Specific processors to enable (if None, all enabled by default)           |
| `disabled_processors` | `list[str]?` | `None`  | Specific processors to disable (takes precedence over enabled_processors) |

Built-in post-processors include:

- `deduplication` - Remove duplicate text blocks
- `whitespace_normalization` - Normalize whitespace and line breaks
- `mojibake_fix` - Fix mojibake (encoding corruption)
- `quality_scoring` - Score and filter low-quality text

### Example

=== "C#"

    --8<-- "snippets/csharp/postprocessor_config.md"

=== "Go"

    --8<-- "snippets/go/config/postprocessor_config.md"

=== "Java"

    --8<-- "snippets/java/config/postprocessor_config.md"

=== "Python"

    --8<-- "snippets/python/config/postprocessor_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/postprocessor_config.md"

=== "R"

    --8<-- "snippets/r/config/postprocessor_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/postprocessor_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/postprocessor_config.md"

---

## TokenReductionConfig

Configuration for reducing token count in extracted text, useful for optimizing LLM context windows.

| Field                      | Type   | Default | Description                                                                   |
| -------------------------- | ------ | ------- | ----------------------------------------------------------------------------- |
| `mode`                     | `str`  | `"off"` | Reduction mode: `"off"`, `"light"`, `"moderate"`, `"aggressive"`, `"maximum"` |
| `preserve_important_words` | `bool` | `true`  | Preserve important words (capitalized, technical terms) during reduction      |

### Reduction Modes

- `off`: No token reduction
- `light`: Remove redundant whitespace and line breaks (~5-10% reduction)
- `moderate`: Light + remove stopwords in low-information contexts (~15-25% reduction)
- `aggressive`: Moderate + abbreviate common phrases (~30-40% reduction)
- `maximum`: Aggressive + remove all stopwords (~50-60% reduction, may impact quality)

### Example

=== "C#"

    --8<-- "snippets/csharp/token_reduction.md"

=== "Go"

    --8<-- "snippets/go/config/token_reduction_config.md"

=== "Java"

    --8<-- "snippets/java/config/token_reduction_config.md"

=== "Python"

    --8<-- "snippets/python/config/token_reduction_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/token_reduction_config.md"

=== "R"

    --8<-- "snippets/r/config/token_reduction_config.md"

=== "Rust"

    --8<-- "snippets/rust/advanced/token_reduction_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/token_reduction_config.md"

---

## LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Configuration for ONNX-based document layout detection. Analyzes PDF pages to identify structural regions such as tables, figures, headers, and text blocks.

**Feature Gate**: Requires the `layout-detection` Cargo feature. Layout detection is only available when this feature is enabled.

!!! Warning "`preset` removed"
    The `preset` field was removed. If present in a config file it is silently ignored. The RT-DETR v2 model is now the only layout detection model.

### Fields

| Field                  | Type       | Default  | Description                                                                                   |
| ---------------------- | ---------- | -------- | --------------------------------------------------------------------------------------------- |
| `confidence_threshold` | `float?`   | `None`   | Confidence threshold override (0.0-1.0). If None, uses the model's built-in default threshold |
| `apply_heuristics`     | `bool`     | `true`   | Apply postprocessing heuristics (containment filtering, deduplication)                         |
| `table_model`          | `str?`     | `None` (uses `"tatr"`) | Table structure recognition model. Options: `"tatr"` (30MB, default), `"slanet_wired"` (365MB, bordered tables), `"slanet_wireless"` (365MB, borderless tables), `"slanet_plus"` (7.78MB, lightweight), `"slanet_auto"` (~737MB, classifier-routed). See [Table Structure Models](../guides/layout-detection.md#table-structure-models). |

!!! Note "Table detection requires layout detection"
    Table extraction only runs when `layout` is set in `ExtractionConfig`. Setting only `table_model` has no effect without an enclosing `LayoutDetectionConfig`.

### Configuration Examples

=== "Python"

    ```python title="layout_detection_config.py"
    from kreuzberg import ExtractionConfig, LayoutDetectionConfig

    config = ExtractionConfig(
        layout=LayoutDetectionConfig(
            confidence_threshold=0.5,
            apply_heuristics=True,
            table_model="slanet_auto",  # or "tatr", "slanet_wired", "slanet_wireless", "slanet_plus"
        )
    )
    ```

=== "TypeScript"

    ```typescript title="layout_detection_config.ts"
    import { extract } from "kreuzberg";

    const result = await extract("document.pdf", {
      layout: {
        confidenceThreshold: 0.5,
        applyHeuristics: true,
        tableModel: "slanet_auto", // or "tatr", "slanet_wired", "slanet_wireless", "slanet_plus"
      },
    });
    ```

=== "Rust"

    ```rust title="layout_detection_config.rs"
    use kreuzberg::core::{ExtractionConfig, LayoutDetectionConfig};

    let config = ExtractionConfig {
        layout: Some(LayoutDetectionConfig {
            confidence_threshold: Some(0.5),
            apply_heuristics: true,
            table_model: Some("slanet_auto".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

### Configuration File Examples

=== "TOML"

    ```toml title="kreuzberg.toml"
    [layout]
    confidence_threshold = 0.5
    apply_heuristics = true
    # table_model = "slanet_auto"
    ```

=== "YAML"

    ```yaml title="kreuzberg.yaml"
    layout:
      confidence_threshold: 0.5
      apply_heuristics: true
      # table_model: slanet_auto
    ```

---

## AccelerationConfig <span class="version-badge">v4.5.0</span>

Controls hardware acceleration for ONNX Runtime inference (layout detection and embeddings).

### Fields

| Field       | Type      | Default  | Description                                                            |
| ----------- | --------- | -------- | ---------------------------------------------------------------------- |
| `provider`  | `str`     | `"auto"` | Execution provider: `"auto"`, `"cpu"`, `"coreml"`, `"cuda"`, `"tensorrt"` |
| `device_id` | `int`     | `0`      | GPU device ID (for CUDA/TensorRT)                                      |

### Provider Behavior

- `auto`: CoreML on macOS, CUDA on Linux, CPU elsewhere
- `cpu`: CPU-only inference (always available)
- `coreml`: Apple CoreML (macOS Neural Engine / GPU)
- `cuda`: NVIDIA CUDA GPU acceleration
- `tensorrt`: NVIDIA TensorRT (optimized CUDA inference)

`cuda` and `tensorrt` only work when the Kreuzberg build was compiled against an ONNX Runtime that ships those execution providers. If a requested provider isn't compiled in or isn't available at runtime, ORT falls back to CPU silently. To verify which provider is actually selected, run with `RUST_LOG=ort=info` and check the startup log.

### Platform Defaults

| Platform        | `provider="auto"` resolves to |
| --------------- | ----------------------------- |
| macOS (arm64)   | `coreml`                      |
| macOS (x86_64)  | `coreml`                      |
| Linux (x86_64)  | `cuda` if available, else `cpu` |
| Linux (aarch64) | `cpu`                         |
| Windows         | `cuda` if available, else `cpu` |

The `device_id` field only matters for `cuda` and `tensorrt`. Set it to the GPU index (`0`, `1`, ...) when running on multi-GPU hosts; it is ignored for every other provider.

### Configuration Examples

=== "Python"

    ```python title="acceleration_config.py"
    from kreuzberg import ExtractionConfig, AccelerationConfig

    # Force CUDA on GPU 0; falls back to CPU if CUDA isn't compiled in
    config = ExtractionConfig(
        acceleration=AccelerationConfig(provider="cuda", device_id=0)
    )

    # macOS: explicitly use CoreML for ONNX inference
    coreml_config = ExtractionConfig(
        acceleration=AccelerationConfig(provider="coreml")
    )
    ```

=== "TypeScript"

    ```typescript title="acceleration_config.ts"
    import { extract } from "kreuzberg";

    const result = await extract("document.pdf", {
      acceleration: { provider: 'cuda', deviceId: 0 },
    });
    ```

=== "Rust"

    ```rust title="acceleration_config.rs"
    use kreuzberg::core::{ExtractionConfig, AccelerationConfig};

    let config = ExtractionConfig {
        acceleration: Some(AccelerationConfig {
            provider: "cuda".to_string(),
            device_id: 0,
        }),
        ..Default::default()
    };
    ```

### Configuration File Examples

=== "TOML"

    ```toml title="kreuzberg.toml"
    [acceleration]
    provider = "cpu"
    device_id = 0
    ```

=== "YAML"

    ```yaml title="kreuzberg.yaml"
    acceleration:
      provider: cpu
      device_id: 0
    ```

---

## ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Controls thread pool and concurrency limits for Rayon parallelism, ONNX Runtime intra-op threading, and batch extraction semaphore.

### Fields

| Field        | Type    | Default | Description                                                                   |
| ------------ | ------- | ------- | ----------------------------------------------------------------------------- |
| `max_threads` | `int?` | `None`  | Maximum number of threads for Rayon thread pool, ONNX intra-op, batch concurrency |

### Overview

Use `ConcurrencyConfig` to constrain resource usage on systems with limited hardware. When set, `max_threads` caps:

- **Rayon thread pool size** for text extraction and parsing parallelism
- **ONNX Runtime intra-op parallelism** for layout detection and embeddings inference
- **Batch extraction semaphore** for limiting concurrent file extractions

Setting `max_threads: None` disables concurrency limits and allows libraries to use all available cores (default behavior).

### Configuration Examples

=== "Python"

    ```python title="concurrency_config.py"
    from kreuzberg import ExtractionConfig, ConcurrencyConfig

    # Limit to 4 threads for constrained hardware
    config = ExtractionConfig(
        concurrency=ConcurrencyConfig(max_threads=4)
    )
    ```

=== "TypeScript"

    ```typescript title="concurrency_config.ts"
    import { extract } from "kreuzberg";

    const result = await extract("document.pdf", {
      concurrency: { maxThreads: 4 },
    });
    ```

=== "Rust"

    ```rust title="concurrency_config.rs"
    use kreuzberg::core::{ExtractionConfig, ConcurrencyConfig};

    let config = ExtractionConfig {
        concurrency: Some(ConcurrencyConfig {
            max_threads: Some(4),
        }),
        ..Default::default()
    };
    ```

=== "Go"

    ```go title="concurrency_config.go"
    package main

    import "kreuzberg"

    config := &kreuzberg.ExtractionConfig{
        Concurrency: &kreuzberg.ConcurrencyConfig{
            MaxThreads: intPtr(4),
        },
    }
    ```

=== "Java"

    ```java title="ConcurrencyConfig.java"
    ConcurrencyConfig concurrency = new ConcurrencyConfig(4);
    ExtractionConfig config = new ExtractionConfig(
        /* ... other fields ... */
        Optional.of(concurrency)
    );
    ```

=== "C#"

    ```csharp title="concurrency_config.cs"
    using Kreuzberg;

    var config = new ExtractionConfig
    {
        Concurrency = new ConcurrencyConfig { MaxThreads = 4 }
    };
    ```

---

## TreeSitterConfig

Configuration for tree-sitter language pack integration. Controls grammar caching and code analysis options when extracting source code files. Requires the `tree-sitter` feature flag.

### Fields

| Field       | Type                       | Default | Description                                                                                      |
| ----------- | -------------------------- | ------- | ------------------------------------------------------------------------------------------------ |
| `enabled`   | `bool`                     | `true`  | Enable code intelligence processing. When `false`, tree-sitter analysis is skipped even if config is present |
| `cache_dir` | `PathBuf?`                 | `None`  | Custom cache directory for downloaded grammars. Default: `~/.cache/tree-sitter-language-pack/v{version}/libs/` |
| `languages` | `Vec<String>?`             | `None`  | Languages to pre-download on init (for example, `["python", "rust"]`)                                   |
| `groups`    | `Vec<String>?`             | `None`  | Language groups to pre-download (for example, `["web", "systems", "scripting"]`)                        |
| `process`   | `TreeSitterProcessConfig`  | default | Processing options for code analysis                                                             |

### TreeSitterProcessConfig

Controls which analysis features are enabled when extracting code files.

| Field            | Type      | Default | Description                                                         |
| ---------------- | --------- | ------- | ------------------------------------------------------------------- |
| `structure`      | `bool`    | `true`  | Extract structural items (functions, classes, structs, etc.)        |
| `imports`        | `bool`    | `true`  | Extract import statements                                           |
| `exports`        | `bool`    | `true`  | Extract export statements                                           |
| `comments`       | `bool`    | `false` | Extract comments                                                    |
| `docstrings`     | `bool`    | `false` | Extract docstrings                                                  |
| `symbols`        | `bool`    | `false` | Extract symbol definitions (variables, constants, type aliases)     |
| `diagnostics`    | `bool`    | `false` | Include parse diagnostics (errors and warnings from tree-sitter)    |
| `chunk_max_size` | `usize?`  | `None`  | Maximum chunk size in bytes. `None` uses the default chunking size  |
| `content_mode`   | `CodeContentMode` | `chunks` | Controls how code content is rendered in the `content` field: `chunks` (semantic chunks, default), `raw` (raw source code), or `structure` (function/class headings + docstrings, no code bodies) |

### Configuration Examples

=== "TOML"

    ```toml title="kreuzberg.toml"
    [tree_sitter]
    languages = ["python", "rust", "typescript"]
    groups = ["web"]

    [tree_sitter.process]
    structure = true
    imports = true
    exports = true
    comments = true
    docstrings = true
    symbols = false
    diagnostics = false
    ```

=== "Rust"

    ```rust title="tree_sitter_config.rs"
    use kreuzberg::{ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig};

    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig {
            process: TreeSitterProcessConfig {
                structure: true,
                imports: true,
                exports: true,
                comments: true,
                docstrings: true,
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    };
    ```

=== "Python"

    ```python title="tree_sitter_config.py"
    import kreuzberg

    config = kreuzberg.ExtractionConfig(
        tree_sitter={
            "process": {
                "structure": True,
                "imports": True,
                "exports": True,
                "comments": True,
                "docstrings": True,
            }
        }
    )
    ```

=== "TypeScript"

    ```typescript title="tree_sitter_config.ts"
    import { ExtractionConfig } from "@kreuzberg/node";

    const config: ExtractionConfig = {
      treeSitter: {
        process: {
          structure: true,
          imports: true,
          exports: true,
          comments: true,
          docstrings: true,
        },
      },
    };
    ```

=== "Go"

    ```go title="tree_sitter_config.go"
    config := &kreuzberg.ExtractionConfig{
        TreeSitter: &kreuzberg.TreeSitterConfig{
            Process: &kreuzberg.TreeSitterProcessConfig{
                Structure:  boolPtr(true),
                Imports:    boolPtr(true),
                Exports:    boolPtr(true),
                Comments:   boolPtr(true),
                Docstrings: boolPtr(true),
            },
        },
    }
    ```

---

## Configuration File Examples

### TOML Format

```toml title="kreuzberg.toml"
use_cache = true
enable_quality_processing = true
force_ocr = false

[ocr]
backend = "tesseract"
language = "eng+fra"

[ocr.tesseract_config]
psm = 6
oem = 1
min_confidence = 0.8
enable_table_detection = true

[ocr.tesseract_config.preprocessing]
target_dpi = 300
denoise = true
deskew = true
contrast_enhance = true
binarization_method = "otsu"

[pdf_options]
extract_images = true
extract_metadata = true
passwords = ["password1", "password2"]

[images]
extract_images = true
target_dpi = 200
max_image_dimension = 4096

[chunking]
max_characters = 1000
overlap = 200

[language_detection]
enabled = true
min_confidence = 0.8
detect_multiple = false

[token_reduction]
mode = "moderate"
preserve_important_words = true

[layout]
preset = "fast"

[postprocessor]
enabled = true
```

### YAML Format

```yaml title="kreuzberg.yaml"
# kreuzberg.yaml
use_cache: true
enable_quality_processing: true
force_ocr: false

ocr:
  backend: tesseract
  language: eng+fra
  tesseract_config:
    psm: 6
    oem: 1
    min_confidence: 0.8
    enable_table_detection: true
    preprocessing:
      target_dpi: 300
      denoise: true
      deskew: true
      contrast_enhance: true
      binarization_method: otsu

pdf_options:
  extract_images: true
  extract_metadata: true
  passwords:
    - password1
    - password2

images:
  extract_images: true
  target_dpi: 200
  max_image_dimension: 4096

chunking:
  max_characters: 1000
  overlap: 200

language_detection:
  enabled: true
  min_confidence: 0.8
  detect_multiple: false

token_reduction:
  mode: moderate
  preserve_important_words: true

layout:
  preset: fast

postprocessor:
  enabled: true
```

### JSON Format

```json title="kreuzberg.json"
{
  "use_cache": true,
  "enable_quality_processing": true,
  "force_ocr": false,
  "ocr": {
    "backend": "tesseract",
    "language": "eng+fra",
    "tesseract_config": {
      "psm": 6,
      "oem": 1,
      "min_confidence": 0.8,
      "enable_table_detection": true,
      "preprocessing": {
        "target_dpi": 300,
        "denoise": true,
        "deskew": true,
        "contrast_enhance": true,
        "binarization_method": "otsu"
      }
    }
  },
  "pdf_options": {
    "extract_images": true,
    "extract_metadata": true,
    "passwords": ["password1", "password2"]
  },
  "images": {
    "extract_images": true,
    "target_dpi": 200,
    "max_image_dimension": 4096
  },
  "chunking": {
    "max_characters": 1000,
    "overlap": 200
  },
  "language_detection": {
    "enabled": true,
    "min_confidence": 0.8,
    "detect_multiple": false
  },
  "token_reduction": {
    "mode": "moderate",
    "preserve_important_words": true
  },
  "layout": {
    "preset": "fast"
  },
  "postprocessor": {
    "enabled": true
  }
}
```

For complete working examples, see the [examples directory](https://github.com/kreuzberg-dev/kreuzberg/tree/main/examples).

---

## Best Practices

### When to Use Config Files vs Programmatic Config

**Use config files when:**

- Settings are shared across multiple scripts/applications
- Configuration needs to be version controlled
- Non-developers need to modify settings
- Deploying to multiple environments (dev/staging/prod)

**Use programmatic config when:**

- Settings vary per execution or are computed dynamically
- Configuration depends on runtime conditions
- Building SDKs or libraries that wrap Kreuzberg
- Rapid prototyping and experimentation

### Performance Considerations

**Caching:**

- Keep `use_cache=true` for repeated processing of the same files
- Cache is automatically invalidated when files change
- Cache location: platform-specific global cache (for example, `~/.cache/kreuzberg/` on Linux, `~/Library/Caches/kreuzberg/` on macOS), configurable via `KREUZBERG_CACHE_DIR` env var or `cache_dir` option

**OCR Settings:**

- Lower `target_dpi` (for example, 150-200) for faster processing of low-quality scans
- Higher `target_dpi` (for example, 400-600) for small text or high-quality documents
- Disable `enable_table_detection` if tables aren't needed (10-20% speedup)
- Use `psm=6` for clean single-column documents (faster than `psm=3`)

**Batch Processing:**

- Set `max_concurrent_extractions` to balance speed and memory usage
- Default (num_cpus \* 2) works well for most systems
- Reduce for memory-constrained environments
- Increase for I/O-bound workloads on systems with fast storage

**Token Reduction:**

- Use `"light"` or `"moderate"` modes for minimal quality impact
- `"aggressive"` and `"maximum"` modes may affect semantic meaning
- Benchmark with your specific LLM to measure quality vs. cost tradeoff

### Security Considerations

**API Keys and Secrets:**

- Never commit config files containing API keys or passwords to version control
- Use environment variables for sensitive data:

  ```bash title="Terminal"
  export KREUZBERG_OCR_API_KEY="your-key-here"
  ```

- Add `kreuzberg.toml` to `.gitignore` if it contains secrets
- Use separate config files for development vs. production

**PDF Passwords:**

- `passwords` field attempts passwords in order until one succeeds
- Passwords are not logged or cached
- Use environment variables for sensitive passwords:

  ```python title="secure_config.py"
  import os
  config = PdfConfig(passwords=[os.getenv("PDF_PASSWORD")])
  ```

**File System Access:**

- Kreuzberg only reads files you explicitly pass to extraction functions
- Cache directory permissions should be restricted to the running user
- Temporary files are automatically cleaned up after extraction

**Data Privacy:**

- Extraction results are never sent to external services (except explicit OCR backends)
- Tesseract OCR runs locally with no network access
- EasyOCR and PaddleOCR may download models on first run (cached locally)
- Consider disabling cache for sensitive documents requiring ephemeral processing

---

## ApiSizeLimits

Configuration for API server request and file upload size limits.

| Field                       | Type  | Default     | Description                                                                   |
| --------------------------- | ----- | ----------- | ----------------------------------------------------------------------------- |
| `max_request_body_bytes`    | `int` | `104857600` | Maximum size of entire request body in bytes (100 MB default)                 |
| `max_multipart_field_bytes` | `int` | `104857600` | Maximum size of individual file in multipart upload in bytes (100 MB default) |

### About Size Limits

Size limits protect your server from resource exhaustion and memory spikes. Both limits default to 100 MB, suitable for typical document processing workloads. Users can configure higher limits via environment variables for processing larger files.

**Default Configuration:**

- Total request body: 100 MB (104,857,600 bytes)
- Individual file: 100 MB (104,857,600 bytes)

**Environment Variable Configuration:**

```bash title="Terminal"
# Set multipart field limit to 200 MB via environment variable
export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=209715200
kreuzberg serve -H 0.0.0.0 -p 8000
```

### Example

=== "C#"

    ```csharp
    using Kreuzberg;
    using Kreuzberg.Api;

    // Default limits: 100 MB for both request body and individual files
    var limits = new ApiSizeLimits();

    // Custom limits: 200 MB for both request body and individual files
    var customLimits = ApiSizeLimits.FromMB(200, 200);

    // Or specify byte values directly
    var customLimits2 = new ApiSizeLimits
    {
        MaxRequestBodyBytes = 200 * 1024 * 1024,
        MaxMultipartFieldBytes = 200 * 1024 * 1024
    };
    ```

=== "Go"

    ```go
    import "kreuzberg"

    // Default limits: 100 MB for both request body and individual files
    limits := kreuzberg.NewApiSizeLimits(
        100 * 1024 * 1024,
        100 * 1024 * 1024,
    )

    // Or use convenience method for custom limits
    limits := kreuzberg.ApiSizeLimitsFromMB(200, 200)
    ```

=== "Java"

    ```java
    import com.kreuzberg.api.ApiSizeLimits;

    // Default limits: 100 MB for both request body and individual files
    ApiSizeLimits limits = new ApiSizeLimits();

    // Custom limits via convenience method
    ApiSizeLimits limits = ApiSizeLimits.fromMB(200, 200);

    // Or specify byte values
    ApiSizeLimits limits = new ApiSizeLimits(
        200 * 1024 * 1024,
        200 * 1024 * 1024
    );
    ```

=== "Python"

    ```python
    from kreuzberg.api import ApiSizeLimits

    # Default limits: 100 MB for both request body and individual files
    limits = ApiSizeLimits()

    # Custom limits via convenience method
    limits = ApiSizeLimits.from_mb(200, 200)

    # Or specify byte values
    limits = ApiSizeLimits(
        max_request_body_bytes=200 * 1024 * 1024,
        max_multipart_field_bytes=200 * 1024 * 1024
    )
    ```

=== "Ruby"

    ```ruby
    require 'kreuzberg'

    # Default limits: 100 MB for both request body and individual files
    limits = Kreuzberg::Api::ApiSizeLimits.new

    # Custom limits via convenience method
    limits = Kreuzberg::Api::ApiSizeLimits.from_mb(200, 200)

    # Or specify byte values
    limits = Kreuzberg::Api::ApiSizeLimits.new(
      max_request_body_bytes: 200 * 1024 * 1024,
      max_multipart_field_bytes: 200 * 1024 * 1024
    )
    ```

=== "Rust"

    ```rust
    use kreuzberg::api::ApiSizeLimits;

    // Default limits: 100 MB for both request body and individual files
    let limits = ApiSizeLimits::default();

    // Custom limits via convenience method
    let limits = ApiSizeLimits::from_mb(200, 200);

    // Or specify byte values
    let limits = ApiSizeLimits::new(
        200 * 1024 * 1024,  // max_request_body_bytes
        200 * 1024 * 1024,  // max_multipart_field_bytes
    );
    ```

=== "TypeScript"

    ```typescript
    import { ApiSizeLimits } from 'kreuzberg';

    // Default limits: 100 MB for both request body and individual files
    const limits = new ApiSizeLimits();

    // Custom limits via convenience method
    const limits = ApiSizeLimits.fromMb(200, 200);

    // Or specify byte values
    const limits = new ApiSizeLimits({
        maxRequestBodyBytes: 200 * 1024 * 1024,
        maxMultipartFieldBytes: 200 * 1024 * 1024
    });
    ```

### Configuration Scenarios

| Use Case                                      | Recommended Limit | Rationale                                            |
| --------------------------------------------- | ----------------- | ---------------------------------------------------- |
| Small documents (standard PDFs, Office files) | 100 MB (default)  | Optimal for typical business documents               |
| Medium documents (large scans, batches)       | 200 MB            | Good balance for batching without excessive memory   |
| Large documents (archives, high-res scans)    | 500-1000 MB       | Suitable for specialized workflows with adequate RAM |
| Development/testing                           | 50 MB             | Conservative limit to catch issues early             |
| Memory-constrained environments               | 50 MB             | Prevents out-of-memory errors on limited systems     |

For comprehensive documentation including memory impact calculations, reverse proxy configuration, and troubleshooting, see the [File Size Limits Reference](./file-size-limits.md).

---

## Related Documentation

- [Configuration Guide](../guides/configuration.md) - Usage guide with examples
- [API Server Guide](../guides/api-server.md) - HTTP API server setup and deployment
- [File Size Limits Reference](./file-size-limits.md) - Complete size limits documentation with performance tuning
- [OCR Guide](../guides/ocr.md) - OCR-specific configuration and troubleshooting
- [Examples Directory](https://github.com/kreuzberg-dev/kreuzberg/tree/main/examples) - Complete working examples
