# Configuration Reference

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
| `host`                      | `String`        | `"127.0.0.1"` | Server host address (e.g., "127.0.0.1", "0.0.0.0")                                                       |
| `port`                      | `u16`           | `8000`        | Server port number (1-65535)                                                                             |
| `cors_origins`              | `Vec<String>`   | empty         | CORS allowed origins. Empty list allows all origins.                                                     |
| `max_request_body_bytes`    | `usize`         | `104857600`   | Maximum request body size in bytes (100 MB default)                                                      |
| `max_multipart_field_bytes` | `usize`         | `104857600`   | Maximum multipart field size in bytes (100 MB default)                                                   |
| `max_upload_mb`             | `Option<usize>` | `None`        | **Legacy**: Use `max_multipart_field_bytes` instead. Automatically converted for backward compatibility. |

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

# Legacy field (in MB)
export KREUZBERG_MAX_UPLOAD_SIZE_MB="200"

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
ENV KREUZBERG_MAX_UPLOAD_SIZE_MB="500"

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
| `ocr`                        | `OcrConfig?`               | `None`                 | OCR configuration (if None, OCR disabled)                                                                                                                                                        |
| `pdf_options`                | `PdfConfig?`               | `None`                 | PDF-specific configuration options                                                                                                                                                               |
| `images`                     | `ImageExtractionConfig?`   | `None`                 | Image extraction configuration                                                                                                                                                                   |
| `chunking`                   | `ChunkingConfig?`          | `None`                 | Text chunking configuration for splitting into chunks                                                                                                                                            |
| `token_reduction`            | `TokenReductionConfig?`    | `None`                 | Token reduction configuration for optimizing LLM context                                                                                                                                         |
| `language_detection`         | `LanguageDetectionConfig?` | `None`                 | Automatic language detection configuration                                                                                                                                                       |
| `postprocessor`              | `PostProcessorConfig?`     | `None`                 | Post-processing pipeline configuration                                                                                                                                                           |
| `pages`                      | `PageConfig?`              | `None`                 | Page extraction and tracking configuration                                                                                                                                                       |
| `max_concurrent_extractions` | `int?`                     | `None`                 | Maximum concurrent batch extractions (defaults to num_cpus \* 2)                                                                                                                                 |
| `result_format`              | `OutputFormat`             | `Unified`              | Result structure format: `Unified` (content in single field) or `ElementBased` (semantic elements array)                                                                                         |
| `output_format`              | `OutputFormat`             | `Plain`                | Output format for extracted text content (Plain, Markdown, Djot, Html, Structured)                                                                                                               |
| `html_options`               | `ConversionOptions`        | `None`                 | HTML to Markdown conversion options (heading styles, list formatting, code block styles). Only available with `html` feature.                                                                    |
| `security_limits`            | `SecurityLimits?`          | `None` (uses defaults) | Archive security thresholds: max archive size (500MB), compression ratio (100:1), file count (10K), nesting depth, content size, XML depth, table cells. Only available with `archives` feature. |
| `include_document_structure` | `bool`                     | `false`                | Enable structured document model output. When true, the `document` field on ExtractionResult is populated with a tree-based representation of document content.                                  |

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

## OcrConfig

Configuration for OCR (Optical Character Recognition) processing on images and scanned PDFs.

| Field              | Type               | Default       | Description                                                           |
| ------------------ | ------------------ | ------------- | --------------------------------------------------------------------- |
| `backend`          | `str`              | `"tesseract"` | OCR backend to use: `"tesseract"`, `"easyocr"`, `"paddleocr"`         |
| `language`         | `str`              | `"eng"`       | Language code(s) for OCR, e.g., `"eng"`, `"eng+fra"`, `"eng+deu+fra"` |
| `tesseract_config` | `TesseractConfig?` | `None`        | Tesseract-specific configuration options                              |

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

## TesseractConfig

Tesseract OCR engine configuration with fine-grained control over recognition parameters.

| Field                                | Type                        | Default      | Description                                              |
| ------------------------------------ | --------------------------- | ------------ | -------------------------------------------------------- |
| `language`                           | `str`                       | `"eng"`      | Language code(s), e.g., `"eng"`, `"eng+fra"`             |
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
| `chunker_type`   | `ChunkerType`      | `Text`  | Type of chunker: `Text` or `Markdown`                                             |

**Note:** `max_chars` and `max_overlap` are accepted as aliases for `max_characters` and `overlap` respectively for backwards compatibility.

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
| `model`                  | `EmbeddingModelType` | `Preset { name: "balanced" }`    | Embedding model selection (preset, fastembed, or custom)                       |
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

### Cache Directory

Model files are cached locally to avoid re-downloading on subsequent runs.

**Default cache location:**

```
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

=== "Rust - FastEmbed Model"

    ```rust title="embedding_fastembed.rs"
    use kreuzberg::core::{EmbeddingConfig, EmbeddingModelType};

    // Explicit FastEmbed model specification
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
| `language`     | `Option<String>`     | `Some("en")`          | Language code for stopword filtering (e.g., "en", "de", "fr"), `None` disables filtering |
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

```
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
| `language`     | `Option<String>`     | `Some("en")`          | Language code for stopword filtering (e.g., "en", "de", "fr"), `None` disables filtering |
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

```
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

| Field              | Type               | Default | Description                                                               |
| ------------------ | ------------------ | ------- | ------------------------------------------------------------------------- |
| `extract_images`   | `bool`             | `false` | Extract embedded images from PDF pages                                    |
| `extract_metadata` | `bool`             | `true`  | Extract PDF metadata (title, author, creation date, etc.)                 |
| `passwords`        | `list[str]?`       | `None`  | List of passwords to try for encrypted PDFs (tries in order)              |
| `hierarchy`        | `HierarchyConfig?` | `None`  | Hierarchy extraction configuration (None = hierarchy extraction disabled) |

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

```
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
                    fmt.Printf("Page %d: %d blocks

", page.PageNumber, page.Hierarchy.BlockCount)
for \_, block := range page.Hierarchy.Blocks {
fmt.Printf(" [%s] %s...
", block.Level, block.Text[:50])
}
}
}
}
}

````

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

````

if (text_block_coverage < ocr_coverage_threshold) {
run_ocr() // Trigger OCR on pages with insufficient text coverage
}

````

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

   ```
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

```
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
                    fmt.Printf("Page %d: %d blocks

", page.PageNumber, page.Hierarchy.BlockCount)
for \_, block := range page.Hierarchy.Blocks {
fmt.Printf(" [%s] %s...
", block.Level, block.Text[:50])
}
}
}
}
}

````

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

````

if (text_block_coverage < ocr_coverage_threshold) {
run_ocr() // Trigger OCR on pages with insufficient text coverage
}

````

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

   ```
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
| `extract_images`      | `bool` | `true`  | Extract images from documents                            |
| `target_dpi`          | `int`  | `300`   | Target DPI for extracted/normalized images               |
| `max_image_dimension` | `int`  | `4096`  | Maximum image dimension (width or height) in pixels      |
| `auto_adjust_dpi`     | `bool` | `true`  | Automatically adjust DPI based on image size and content |
| `min_dpi`             | `int`  | `72`    | Minimum DPI when auto-adjusting                          |
| `max_dpi`             | `int`  | `600`   | Maximum DPI when auto-adjusting                          |

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
- Cache location: `.kreuzberg/` (relative to current working directory, configurable via `cache_dir` option)

**OCR Settings:**

- Lower `target_dpi` (e.g., 150-200) for faster processing of low-quality scans
- Higher `target_dpi` (e.g., 400-600) for small text or high-quality documents
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
# Set both limits to 200 MB via environment variable
export KREUZBERG_MAX_UPLOAD_SIZE_MB=200
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
