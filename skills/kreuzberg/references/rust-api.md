# Kreuzberg Rust API Reference

Complete API reference for the Kreuzberg document extraction library in Rust.

## Setup

Add to your `Cargo.toml`:

```toml
[dependencies]
kreuzberg = { version = "4", features = [
    "tokio-runtime",
    "pdf",
    "ocr",
    "chunking",
    "embeddings",
    "language-detection",
    "keywords-yake",
    "keywords-rake",
    "api",
    "mcp"
] }
tokio = { version = "1", features = ["full"] }
```

### Core Features

- **tokio-runtime**: Enables async/sync extraction (default). Required for `extract_file_sync`, `batch_extract_file_sync`, `batch_extract_file`
- **pdf**: PDF extraction with PDFium
- **ocr**: Tesseract-based OCR for scanned documents
- **chunking**: Text chunking for RAG pipelines
- **embeddings**: Vector embeddings generation
- **language-detection**: Detect document language
- **keywords-yake** / **keywords-rake**: Extract keywords using YAKE or RAKE
- **api**: HTTP API with Axum
- **mcp**: Model Context Protocol support

---

## Core Extraction Functions

### `extract_file` (async)

Extract content from a file path.

```rust
pub async fn extract_file(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult>
```

**Always available.** Requires async context (`#[tokio::main]`, `tokio::spawn`, etc.).

```rust
use kreuzberg::{extract_file, ExtractionConfig};
use std::path::Path;

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;
    println!("Content: {}", result.content);
    Ok(())
}
```

### `extract_bytes` (async)

Extract content from byte data.

```rust
pub async fn extract_bytes(
    data: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractionResult>
```

**Always available.** Requires async context.

```rust
#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let pdf_bytes = std::fs::read("document.pdf")?;
    let result = extract_bytes(&pdf_bytes, "application/pdf", &config).await?;
    Ok(())
}
```

### `extract_file_sync` (sync)

Synchronous wrapper around `extract_file`.

```rust
pub fn extract_file_sync(
    path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig,
) -> Result<ExtractionResult>
```

**Requires tokio-runtime feature.** Blocks the current thread using a global Tokio runtime.

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Content: {}", result.content);
    Ok(())
}
```

### `extract_bytes_sync` (sync)

Synchronous wrapper around `extract_bytes`.

```rust
pub fn extract_bytes_sync(
    content: &[u8],
    mime_type: &str,
    config: &ExtractionConfig,
) -> Result<ExtractionResult>
```

**Always available.** Works in sync and async contexts.

```rust
fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let bytes = b"Hello, world!";
    let result = extract_bytes_sync(bytes, "text/plain", &config)?;
    Ok(())
}
```

### `batch_extract_file` (async, parallel)

Extract multiple files concurrently.

```rust
pub async fn batch_extract_file(
    paths: Vec<impl AsRef<Path>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>>
```

**Requires tokio-runtime feature.** Processes files in parallel with automatic concurrency management (defaults to `num_cpus * 1.5`).

```rust
#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let paths = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let results = batch_extract_file(paths, &config).await?;
    println!("Processed {} files", results.len());
    Ok(())
}
```

### `batch_extract_bytes` (async, parallel)

Extract multiple byte arrays concurrently.

```rust
pub async fn batch_extract_bytes(
    contents: Vec<(Vec<u8>, String)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>>
```

**Requires tokio-runtime feature.** Each tuple is `(bytes, mime_type)`.

```rust
#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let contents = vec![
        (b"PDF content".to_vec(), "application/pdf".to_string()),
        (b"Text content".to_vec(), "text/plain".to_string()),
    ];
    let results = batch_extract_bytes(contents, &config).await?;
    Ok(())
}
```

### `batch_extract_file_sync` (sync, parallel)

Synchronous wrapper for batch file extraction.

```rust
pub fn batch_extract_file_sync(
    paths: Vec<impl AsRef<Path>>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>>
```

**Requires tokio-runtime feature.** Uses global runtime for concurrency.

```rust
fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let paths = vec!["doc1.pdf", "doc2.pdf"];
    let results = batch_extract_file_sync(paths, &config)?;
    Ok(())
}
```

### `batch_extract_bytes_sync` (sync, parallel)

Synchronous wrapper for batch byte extraction.

```rust
pub fn batch_extract_bytes_sync(
    contents: Vec<(Vec<u8>, String)>,
    config: &ExtractionConfig,
) -> Result<Vec<ExtractionResult>>
```

**Always available.** Each tuple is `(bytes, mime_type)`.

```rust
fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let contents = vec![
        (b"content 1".to_vec(), "text/plain".to_string()),
        (b"content 2".to_vec(), "text/plain".to_string()),
    ];
    let results = batch_extract_bytes_sync(contents, &config)?;
    Ok(())
}
```

---

## Configuration

### `ExtractionConfig`

Main configuration struct for all extraction operations.

```rust
pub struct ExtractionConfig {
    /// Enable caching (default: true)
    pub use_cache: bool,

    /// Enable quality post-processing (default: true)
    pub enable_quality_processing: bool,

    /// OCR configuration (None = OCR disabled)
    pub ocr: Option<OcrConfig>,

    /// Force OCR even for searchable PDFs (default: false)
    pub force_ocr: bool,

    /// Text chunking configuration (None = disabled)
    pub chunking: Option<ChunkingConfig>,

    /// Image extraction configuration (None = disabled)
    pub images: Option<ImageExtractionConfig>,

    /// PDF-specific options (requires pdf feature)
    #[cfg(feature = "pdf")]
    pub pdf_options: Option<PdfConfig>,

    /// Token reduction configuration (None = disabled)
    pub token_reduction: Option<TokenReductionConfig>,

    /// Language detection configuration (None = disabled)
    pub language_detection: Option<LanguageDetectionConfig>,

    /// Page extraction configuration (None = disabled)
    pub pages: Option<PageConfig>,

    /// Keyword extraction configuration (requires keywords-yake or keywords-rake)
    #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
    pub keywords: Option<KeywordConfig>,

    /// Post-processor configuration (None = use defaults)
    pub postprocessor: Option<PostProcessorConfig>,

    /// HTML to Markdown conversion options (requires html feature)
    #[cfg(feature = "html")]
    pub html_options: Option<ConversionOptions>,

    /// Maximum concurrent extractions in batch (None = num_cpus * 1.5)
    pub max_concurrent_extractions: Option<usize>,

    /// Result structure format (default: Unified)
    /// Uses types::OutputFormat (Unified | ElementBased)
    pub result_format: types::OutputFormat,

    /// Security limits for archives (requires archives feature)
    #[cfg(feature = "archives")]
    pub security_limits: Option<SecurityLimits>,

    /// Content output format (default: Plain)
    /// Uses config::OutputFormat (Plain | Markdown | Djot | Html)
    pub output_format: OutputFormat,
}
```

#### Creating Configs

```rust
use kreuzberg::{ExtractionConfig, OcrConfig, ChunkingConfig, OutputFormat};

// Default configuration
let config = ExtractionConfig::default();

// With OCR
let config = ExtractionConfig {
    ocr: Some(OcrConfig {
        backend: "tesseract".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};

// With chunking
let config = ExtractionConfig {
    chunking: Some(ChunkingConfig {
        max_characters: 512,
        overlap: 50,
        ..Default::default()
    }),
    output_format: OutputFormat::Markdown,
    ..Default::default()
};
```

---

## Output Formats

There are two separate enums both named `OutputFormat` in different modules:

### Content `OutputFormat` (`core::config::formats::OutputFormat`)

Controls the format of the `content` field text. Used by `ExtractionConfig::output_format`.

```rust
pub enum OutputFormat {
    /// Plain text (default)
    Plain,
    /// Markdown formatted
    Markdown,
    /// Djot markup format
    Djot,
    /// HTML format
    Html,
}
```

### Result `OutputFormat` (`types::extraction::OutputFormat`)

Controls the result structure. Used by `ExtractionConfig::result_format`.

```rust
pub enum OutputFormat {
    /// Unified format with all content in `content` field (default)
    Unified,
    /// Element-based format with semantic element extraction
    ElementBased,
}
```

```rust
use kreuzberg::{ExtractionConfig, OutputFormat};

let config = ExtractionConfig {
    output_format: OutputFormat::Markdown,  // content format (Plain/Markdown/Djot/Html)
    // result_format uses types::OutputFormat (Unified/ElementBased) — defaults to Unified
    ..Default::default()
};
```

---

## Extraction Result

### `ExtractionResult`

Result returned by all extraction functions.

```rust
pub struct ExtractionResult {
    /// Main extracted content
    pub content: String,

    /// Document MIME type
    pub mime_type: Cow<'static, str>,

    /// Metadata about extraction
    pub metadata: Metadata,

    /// Extracted tables (HTML/Markdown)
    pub tables: Vec<Table>,

    /// Detected languages (if language-detection enabled)
    pub detected_languages: Option<Vec<String>>,

    /// Text chunks (if chunking enabled)
    pub chunks: Option<Vec<Chunk>>,

    /// Extracted images (if image extraction enabled)
    pub images: Option<Vec<ExtractedImage>>,

    /// Per-page content (if page extraction enabled)
    pub pages: Option<Vec<PageContent>>,

    /// Semantic elements (if element-based format enabled)
    pub elements: Option<Vec<Element>>,

    /// Djot document structure (if extracting Djot)
    pub djot_content: Option<DjotContent>,
}
```

### `Chunk`

Text chunk with optional embedding.

```rust
pub struct Chunk {
    /// Chunk text content
    pub content: String,

    /// Optional embedding vector
    pub embedding: Option<Vec<f32>>,

    /// Chunk metadata
    pub metadata: ChunkMetadata,
}

pub struct ChunkMetadata {
    pub byte_start: usize,
    pub byte_end: usize,
    pub token_count: Option<usize>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub first_page: Option<usize>,
    pub last_page: Option<usize>,
}
```

### `ExtractedImage`

Image extracted from document.

```rust
pub struct ExtractedImage {
    /// Raw image bytes
    pub data: Bytes,

    /// Format: "jpeg", "png", "webp", etc.
    pub format: Cow<'static, str>,

    /// Zero-indexed position
    pub image_index: usize,

    /// Page number (1-indexed)
    pub page_number: Option<usize>,

    /// Image dimensions
    pub width: Option<u32>,
    pub height: Option<u32>,

    /// Colorspace: "RGB", "CMYK", "Gray"
    pub colorspace: Option<String>,

    /// Bits per component
    pub bits_per_component: Option<u32>,

    /// Whether this is a mask image
    pub is_mask: bool,

    /// Image description
    pub description: Option<String>,

    /// Nested OCR result (if OCRed)
    pub ocr_result: Option<Box<ExtractionResult>>,
}
```

---

## Error Handling

### `KreuzbergError` enum

```rust
pub enum KreuzbergError {
    /// File system errors (always bubble up)
    Io(std::io::Error),

    /// Document parsing errors
    Parsing {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// OCR processing errors
    Ocr {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration/input validation errors
    Validation {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Cache operation errors
    Cache {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Image processing errors
    ImageProcessing {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Serialization errors (JSON, MessagePack)
    Serialization {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Missing system dependency (Tesseract, LibreOffice)
    MissingDependency(String),

    /// Plugin-specific errors
    Plugin {
        message: String,
        plugin_name: String,
    },

    /// Mutex/RwLock poisoning
    LockPoisoned(String),

    /// Unsupported MIME type or format
    UnsupportedFormat(String),

    /// Other errors
    Other(String),
}
```

#### Error Constructors

```rust
use kreuzberg::KreuzbergError;

// Create errors
let err = KreuzbergError::parsing("invalid PDF");
let err = KreuzbergError::ocr("Tesseract failed");
let err = KreuzbergError::validation("config invalid");
let err = KreuzbergError::unsupported_format("application/unknown");
let err = KreuzbergError::missing_dependency("tesseract");

// With source
let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
let err = KreuzbergError::parsing_with_source("corrupt PDF", source);
```

#### Handling Errors

```rust
use kreuzberg::extract_file;

match extract_file("doc.pdf", None, &config).await {
    Ok(result) => println!("Success: {}", result.content),
    Err(kreuzberg::KreuzbergError::Io(e)) => {
        println!("File error: {}", e);
    }
    Err(kreuzberg::KreuzbergError::UnsupportedFormat(fmt)) => {
        println!("Unsupported: {}", fmt);
    }
    Err(e) => println!("Other error: {}", e),
}
```

---

## MIME Type Detection

### `detect_mime_type`

Detect MIME type from file path.

```rust
pub fn detect_mime_type(path: impl AsRef<Path>) -> Result<String>
```

```rust
use kreuzberg::detect_mime_type;

let mime = detect_mime_type("document.pdf")?;
assert_eq!(mime, "application/pdf");
```

### `detect_mime_type_from_bytes`

Detect MIME type from byte data.

```rust
pub fn detect_mime_type_from_bytes(data: &[u8]) -> Result<String>
```

### `validate_mime_type`

Check if a MIME type is supported.

```rust
pub fn validate_mime_type(mime_type: &str) -> Result<()>
```

```rust
use kreuzberg::validate_mime_type;

validate_mime_type("application/pdf")?;  // OK
validate_mime_type("application/unknown")?;  // Error
```

### `get_extensions_for_mime`

Get file extensions for a MIME type.

```rust
pub fn get_extensions_for_mime(mime_type: &str) -> Vec<String>
```

```rust
use kreuzberg::get_extensions_for_mime;

let exts = get_extensions_for_mime("application/pdf");
// ["pdf"]

let exts = get_extensions_for_mime("text/plain");
// ["txt", "text"]
```

### MIME Type Constants

```rust
use kreuzberg::{
    PDF_MIME_TYPE,
    PLAIN_TEXT_MIME_TYPE,
    HTML_MIME_TYPE,
    MARKDOWN_MIME_TYPE,
    JSON_MIME_TYPE,
    XML_MIME_TYPE,
    DOCX_MIME_TYPE,
    POWER_POINT_MIME_TYPE,
    EXCEL_MIME_TYPE,
};

assert_eq!(PDF_MIME_TYPE, "application/pdf");
assert_eq!(PLAIN_TEXT_MIME_TYPE, "text/plain");
```

---

## Plugin Registry

Access extractors, OCR backends, and validators.

### `get_document_extractor_registry`

Get all available document extractors.

```rust
pub fn get_document_extractor_registry() -> Arc<RwLock<DocumentExtractorRegistry>>
```

### `get_ocr_backend_registry`

Get all available OCR backends.

```rust
pub fn get_ocr_backend_registry() -> Arc<RwLock<OcrBackendRegistry>>
```

### `get_post_processor_registry`

Get all available post-processors.

```rust
pub fn get_post_processor_registry() -> Arc<RwLock<PostProcessorRegistry>>
```

### `get_validator_registry`

Get all available validators.

```rust
pub fn get_validator_registry() -> Arc<RwLock<ValidatorRegistry>>
```

---

## Complete Example

```rust
use kreuzberg::{
    extract_file, ExtractionConfig, OutputFormat,
    ChunkingConfig, OcrConfig, LanguageDetectionConfig,
};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Configure extraction
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        chunking: Some(ChunkingConfig {
            max_characters: 512,
            overlap: 50,
            ..Default::default()
        }),
        language_detection: Some(LanguageDetectionConfig::default()),
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            ..Default::default()
        }),
        force_ocr: false,
        ..Default::default()
    };

    // Extract from file
    let result = extract_file("document.pdf", None, &config).await?;

    // Use results
    println!("Content:\n{}", result.content);
    println!("MIME: {}", result.mime_type);

    if let Some(langs) = result.detected_languages {
        println!("Languages: {:?}", langs);
    }

    if let Some(chunks) = result.chunks {
        println!("Chunks: {}", chunks.len());
        for chunk in chunks {
            println!("  - {}", &chunk.content[..50.min(chunk.content.len())]);
        }
    }

    if let Some(images) = result.images {
        println!("Images: {}", images.len());
    }

    if let Some(pages) = result.pages {
        println!("Pages: {}", pages.len());
    }

    Ok(())
}
```

---

## Result Type Alias

```rust
pub type Result<T> = std::result::Result<T, KreuzbergError>;
```

All fallible operations return `Result<T>` where errors are `KreuzbergError`.

---

## Feature Flags Summary

| Feature | Availability | Dependencies |
|---------|--------------|--------------|
| tokio-runtime | Default | Tokio runtime for async/sync |
| pdf | Default | PDFium |
| ocr | Optional | Tesseract |
| chunking | Optional | text-splitter |
| embeddings | Optional | FastEmbed, requires tokio-runtime |
| language-detection | Optional | whatlang |
| keywords-yake | Optional | yake-rust |
| keywords-rake | Optional | rake |
| api | Optional | Axum, requires tokio-runtime |
| mcp | Optional | Model Context Protocol, requires tokio-runtime |

---

## Version

This reference is for Kreuzberg 4.x.
