# Rust API Reference

Complete reference for the Kreuzberg Rust API.

## Installation

Add to your `Cargo.toml`:

```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
tokio = { version = "1", features = ["rt", "macros"] }
```

**With specific features:**

```toml title="Cargo.toml"
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "ocr", "chunking", "api"] }
```

**Available features:**

- `default` - Includes `tokio-runtime` and `simd-utf8` (sync file APIs require `tokio-runtime`)
- `tokio-runtime` - Enables async and sync file APIs: `extract_file`, `extract_file_sync`, `extract_bytes`, `batch_extract_file`, `batch_extract_file_sync`, `batch_extract_bytes`
- `simd-utf8` - SIMD-accelerated UTF-8 validation
- `pdf` - PDF extraction support
- `ocr` - OCR support with Tesseract
- `paddle-ocr` - PaddleOCR backend (requires `ocr`; not available on WASM)
- `chunking` - Text chunking algorithms
- `embeddings` - Chunk embedding generation via ONNX models
- `language-detection` - Language detection
- `keywords-yake` - YAKE keyword extraction
- `keywords-rake` - RAKE keyword extraction
- `quality` - Unicode normalization, encoding detection, stopwords
- `api` - HTTP API server support
- `mcp` - Model Context Protocol server support
- `mcp-http` - MCP over HTTP (enables `mcp` and `api`)
- `excel` - Excel/spreadsheet extraction
- `office` - Office formats (DOCX, ODT, RTF, etc.)
- `html` - HTML to Markdown conversion
- `xml` - XML extraction
- `archives` - ZIP, TAR, 7Z extraction
- `email` - EML/MSG email extraction
- `otel` - OpenTelemetry instrumentation
- `wasm-target` - WASM-friendly feature set (pdf, html, xml, email, language-detection, chunking, quality, office)
- `full` - All format and server features
- `server` - PDF, excel, html, ocr, paddle-ocr, chunking, api, mcp
- `cli` - Feature set for CLI usage

## Core Functions

### Extract_file_sync()

Extract content from a file (synchronous, blocking). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub fn extract_file_sync(
    file_path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

- `file_path` (impl AsRef<Path>): Path to the file to extract
- `mime_type` (Option<&str>): Optional MIME type hint. If None, MIME type is auto-detected
- `config` (&ExtractionConfig): Extraction configuration reference

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Errors:**

- `KreuzbergError::Io` - File system errors (file not found, permission denied, etc.)
- `KreuzbergError::Validation` - Invalid configuration or file path
- `KreuzbergError::Parsing` - Document parsing failure
- `KreuzbergError::Ocr` - OCR processing failure
- `KreuzbergError::MissingDependency` - Required system dependency not found

**Examples:**

```rust title="basic_extraction.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    // Extract a document synchronously with default configuration
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    println!("Content: {}", result.content);
    if let Some(ref pages) = result.metadata.pages {
        println!("Pages: {}", pages.total_count);
    }

    Ok(())
}
```

```rust title="with_ocr.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig};

fn main() -> kreuzberg::Result<()> {
    // Configure OCR for scanned documents
    let config = ExtractionConfig {
        ocr: Some(OcrConfig::default()),
        force_ocr: false,
        ..Default::default()
    };

    let result = extract_file_sync("scanned.pdf", None, &config)?;
    println!("Extracted: {}", result.content);

    Ok(())
}
```

---

### Extract_file()

Extract content from a file (asynchronous). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub async fn extract_file(
    file_path: impl AsRef<Path>,
    mime_type: Option<&str>,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

Same as [`extract_file_sync()`](#extract_file_sync).

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Examples:**

```rust title="async_extraction.rs"
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Extract a document asynchronously
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;

    println!("Content: {}", result.content);
    Ok(())
}
```

---

### Extract_bytes_sync()

Extract content from bytes (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn extract_bytes_sync(
    data: &[u8],
    mime_type: &str,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

- `data` (&[u8]): File content as byte slice
- `mime_type` (&str): MIME type of the data (required for format detection)
- `config` (&ExtractionConfig): Extraction configuration reference

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

**Examples:**

```rust title="byte_extraction.rs"
use kreuzberg::{extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    // Extract from in-memory byte array
    let data = fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&data, "application/pdf", &config)?;

    println!("Content: {}", result.content);
    Ok(())
}
```

---

### Extract_bytes()

Extract content from bytes (asynchronous). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub async fn extract_bytes(
    data: &[u8],
    mime_type: &str,
    config: &ExtractionConfig
) -> Result<ExtractionResult>
```

**Parameters:**

Same as [`extract_bytes_sync()`](#extract_bytes_sync).

**Returns:**

- `Result<ExtractionResult>`: Result containing extraction result or error

---

### Batch_extract_file_sync()

Extract content from multiple files in parallel (synchronous, blocking). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub fn batch_extract_file_sync(
    paths: &[impl AsRef<Path>],
    mime_types: Option<&[&str]>,
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

- `paths` (&[impl AsRef<Path>]): Slice of file paths to extract
- `mime_types` (Option<&[&str]>): Optional MIME type hints (must match paths length if provided)
- `config` (&ExtractionConfig): Extraction configuration applied to all files

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

**Examples:**

```rust title="batch_processing.rs"
use kreuzberg::{batch_extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    // Process multiple files in parallel for better performance
    let paths = ["doc1.pdf", "doc2.docx", "doc3.xlsx"];
    let config = ExtractionConfig::default();
    let results = batch_extract_file_sync(&paths, None, &config)?;

    // Display results for each file
    for (i, result) in results.iter().enumerate() {
        println!("{}: {} characters", paths[i], result.content.len());
    }

    Ok(())
}
```

---

### Batch_extract_file()

Extract content from multiple files in parallel (asynchronous). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub async fn batch_extract_file(
    paths: &[impl AsRef<Path>],
    mime_types: Option<&[&str]>,
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

Same as [`batch_extract_file_sync()`](#batch_extract_file_sync).

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

**Examples:**

```rust title="async_batch_processing.rs"
use kreuzberg::{batch_extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    // Process multiple files asynchronously in parallel
    let files = ["doc1.pdf", "doc2.docx", "doc3.xlsx"];
    let config = ExtractionConfig::default();
    let results = batch_extract_file(&files, None, &config).await?;

    // Print extracted content from each file
    for result in results {
        println!("{}", result.content);
    }

    Ok(())
}
```

---

### Batch_extract_bytes_sync()

Extract content from multiple byte arrays in parallel (synchronous, blocking).

**Signature:**

```rust title="Rust"
pub fn batch_extract_bytes_sync(
    data_list: &[&[u8]],
    mime_types: &[&str],
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

- `data_list` (&[&[u8]]): Slice of file contents as byte slices
- `mime_types` (&[&str]): Slice of MIME types (must match data_list length)
- `config` (&ExtractionConfig): Extraction configuration applied to all items

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

---

### Batch_extract_bytes()

Extract content from multiple byte arrays in parallel (asynchronous). **Requires the `tokio-runtime` feature.**

**Signature:**

```rust title="Rust"
pub async fn batch_extract_bytes(
    data_list: &[&[u8]],
    mime_types: &[&str],
    config: &ExtractionConfig
) -> Result<Vec<ExtractionResult>>
```

**Parameters:**

Same as [`batch_extract_bytes_sync()`](#batch_extract_bytes_sync).

**Returns:**

- `Result<Vec<ExtractionResult>>`: Result containing vector of extraction results

---

### FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All fields are `Option<T>` — `None` means "use the batch-level default." See [Configuration Reference](configuration.md#fileextractionconfig) for full field documentation.

**Definition:**

```rust title="Rust"
pub struct FileExtractionConfig {
    pub enable_quality_processing: Option<bool>,
    pub ocr: Option<OcrConfig>,
    pub force_ocr: Option<bool>,
    pub chunking: Option<ChunkingConfig>,
    pub images: Option<ImageExtractionConfig>,
    pub pdf_options: Option<PdfConfig>,          // requires "pdf" feature
    pub token_reduction: Option<TokenReductionConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub pages: Option<PageConfig>,
    pub keywords: Option<KeywordConfig>,         // requires keywords feature
    pub postprocessor: Option<PostProcessorConfig>,
    pub html_options: Option<ConversionOptions>, // requires "html" feature
    pub result_format: Option<OutputFormat>,
    pub output_format: Option<OutputFormat>,
    pub include_document_structure: Option<bool>,
    pub layout: Option<LayoutDetectionConfig>,   // requires "layout-detection" feature
}
```

---

## Configuration

### ExtractionConfig

Main configuration struct for extraction operations.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionConfig {
    pub chunking: Option<ChunkingConfig>,
    pub enable_quality_processing: bool,
    pub force_ocr: bool,
    pub html_options: Option<html_to_markdown_rs::ConversionOptions>,
    pub images: Option<ImageExtractionConfig>,
    pub include_document_structure: bool,
    pub keywords: Option<KeywordConfig>,
    pub language_detection: Option<LanguageDetectionConfig>,
    pub layout: Option<LayoutDetectionConfig>,
    pub max_concurrent_extractions: Option<usize>,
    pub ocr: Option<OcrConfig>,
    pub output_format: OutputFormat,
    pub pages: Option<PageConfig>,
    pub pdf_options: Option<PdfConfig>,
    pub postprocessor: Option<PostProcessorConfig>,
    pub result_format: crate::types::OutputFormat,
    pub security_limits: Option<SecurityLimits>,
    pub token_reduction: Option<TokenReductionConfig>,
    pub use_cache: bool,
}
```

**Note:** `result_format` uses `crate::types::OutputFormat` (Unified | ElementBased) for result structure. `output_format` uses the re-exported `OutputFormat` from config (Plain | Markdown | Djot | Html | Structured) for content format.

**Fields:**

- `chunking` (Option<ChunkingConfig>): Text chunking configuration. Default: None
- `concurrency` (Option<ConcurrencyConfig>) <span class="version-badge">v4.5.0</span>: Concurrency configuration. Default: None
- `enable_quality_processing` (bool): Enable quality post-processing. Default: true
- `force_ocr` (bool): Force OCR even for text-based PDFs. Default: false
- `html_options` (Option<[ConversionOptions](https://docs.html-to-markdown.kreuzberg.dev/reference/configuration/)>): HTML conversion options from [html-to-markdown](https://docs.html-to-markdown.kreuzberg.dev) (when feature `html`). Default: None
- `images` (Option<ImageExtractionConfig>): Image extraction from documents. Default: None
- `include_document_structure` (bool): Populate `document` field with hierarchical DocumentStructure. Default: false
- `keywords` (Option<KeywordConfig>): Keyword extraction (requires `keywords-yake` or `keywords-rake`). Default: None
- `language_detection` (Option<LanguageDetectionConfig>): Language detection configuration. Default: None
- `layout` (Option<LayoutDetectionConfig>): Layout detection settings (requires `layout-detection` feature). Default: None
- `max_concurrent_extractions` (Option<usize>): Max concurrent extractions in batch; None = (num_cpus × 1.5).ceil(). Default: None
- `ocr` (Option<OcrConfig>): OCR configuration. Default: None (no OCR)
- `output_format` (OutputFormat): Content format: Plain, Markdown, Djot, Html, or Structured. Default: Plain
- `pages` (Option<PageConfig>): Page extraction and tracking. Default: None
- `pdf_options` (Option<PdfConfig>): PDF-specific configuration (requires `pdf` feature). Default: None
- `postprocessor` (Option<PostProcessorConfig>): Post-processing configuration. Default: None
- `result_format` (types::OutputFormat): Result structure: Unified or ElementBased. Default: Unified
- `security_limits` (Option<SecurityLimits>): Archive extraction limits (when feature `archives`). See [SecurityLimits](#securitylimits). Default: None
- `token_reduction` (Option<TokenReductionConfig>): Token reduction configuration. Default: None
- `use_cache` (bool): Enable caching of extraction results. Default: true

**Methods:**

- `needs_image_processing(&self) -> bool`: Returns true if OCR or image extraction is enabled (used to skip image decompression when not needed).

**Example:**

```rust title="advanced_config.rs"
use kreuzberg::{ExtractionConfig, OcrConfig, PdfConfig};

// Configure extraction with OCR and PDF-specific options
let config = ExtractionConfig {
    ocr: Some(OcrConfig::default()),
    force_ocr: false,
    pdf_options: Some(PdfConfig {
        passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
        extract_images: true,
        extract_metadata: true,
        hierarchy: None,
    }),
    ..Default::default()
};
```

---

### OcrConfig

OCR processing configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrConfig {
    pub backend: String,
    pub language: String,
    pub tesseract_config: Option<TesseractConfig>,
    pub output_format: Option<OutputFormat>,
    pub paddle_ocr_config: Option<serde_json::Value>,
    pub element_config: Option<OcrElementConfig>,
}
```

**Fields:**

- `backend` (String): OCR backend. Options: "tesseract", "easyocr", "paddleocr". Default: "tesseract"
- `language` (String): Language code for OCR (ISO 639-3) (for example "eng", "deu"). Default: "eng"
- `tesseract_config` (Option<TesseractConfig>): Tesseract-specific configuration. Default: None
- `output_format` (Option<OutputFormat>): Output format for OCR results. Default: None
- `paddle_ocr_config` (Option<serde_json::Value>): PaddleOCR-specific options (when backend is "paddleocr"). Supports `model_tier` <span class="version-badge">v4.5.0</span> ("mobile" default, or "server" for max accuracy) and `padding` <span class="version-badge">v4.5.0</span> (0-100, default 10). Default: None
- `element_config` (Option<OcrElementConfig>): OCR element extraction (bounding boxes, confidence). Default: None

**Methods:**

- `validate(&self) -> Result<(), KreuzbergError>`: Validates that the configured backend is supported (tesseract, easyocr, paddleocr). Returns `Err(KreuzbergError::Validation)` if the backend is not recognized.

**Example:**

```rust title="ocr_config.rs"
use kreuzberg::OcrConfig;

// Configure OCR backend and language settings
let ocr_config = OcrConfig {
    backend: "tesseract".to_string(),
    language: "eng".to_string(),
    tesseract_config: None,
    ..Default::default()
};
```

---

### TesseractConfig

Tesseract OCR backend configuration. Provides fine-grained control over the Tesseract engine (PSM, OEM, table detection, preprocessing, caching, and tessedit variables).

**Definition (main fields):**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TesseractConfig {
    pub language: String,
    pub psm: i32,
    pub output_format: String,           // "text" or "markdown"
    pub oem: i32,
    pub min_confidence: f64,
    pub preprocessing: Option<ImagePreprocessingConfig>,
    pub enable_table_detection: bool,
    pub table_min_confidence: f64,
    pub table_column_threshold: i32,
    pub table_row_threshold_ratio: f64,
    pub use_cache: bool,
    pub tessedit_char_whitelist: String,  // empty = all allowed
    pub tessedit_char_blacklist: String,
    // ... additional tessedit/textord fields
}
```

**Fields (summary):**

- `language` (String): Language code (for example "eng", "deu"). Default: "eng"
- `psm` (i32): Page segmentation mode (0-13). Default: 3
- `output_format` (String): "text" or "markdown". Default: "markdown"
- `oem` (i32): OCR engine mode (0-3). Default: 3
- `min_confidence` (f64): Minimum confidence (0.0-100.0). Default: 0.0
- `preprocessing` (Option<ImagePreprocessingConfig>): Image preprocessing before OCR. Default: None
- `enable_table_detection` (bool): Enable table detection. Default: true
- `table_min_confidence` (f64): Table detection confidence threshold (0.0-1.0). Default: 0.0
- `table_column_threshold` (i32): Column threshold in pixels. Default: 50
- `table_row_threshold_ratio` (f64): Row threshold ratio. Default: 0.5
- `tessedit_char_whitelist` (String): Allowed characters (empty = all). Default: ""
- `tessedit_char_blacklist` (String): Forbidden characters. Default: ""
- `use_cache` (bool): Enable OCR result caching. Default: true

**Example:**

```rust title="tesseract_config.rs"
use kreuzberg::{ExtractionConfig, OcrConfig, TesseractConfig};

// Configure Tesseract with custom settings for numeric extraction
let config = ExtractionConfig {
    ocr: Some(OcrConfig {
        backend: "tesseract".to_string(),
        language: "eng".to_string(),
        tesseract_config: Some(TesseractConfig {
            psm: 6,
            enable_table_detection: true,
            tessedit_char_whitelist: "0123456789".to_string(),
            tessedit_char_blacklist: String::new(),
            ..Default::default()
        }),
    }),
    ..Default::default()
};
```

---

### PdfConfig

PDF-specific configuration (requires `pdf` feature).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    pub allow_single_column_tables: bool,
    pub extract_images: bool,
    pub passwords: Option<Vec<String>>,
    pub extract_metadata: bool,
    pub hierarchy: Option<HierarchyConfig>,
}
```

**Fields:**

- `allow_single_column_tables` (bool) <span class="version-badge">v4.5.0</span>: Allow extraction of single-column tables. Default: false
- `extract_images` (bool): Extract images from PDF. Default: false
- `passwords` (Option<Vec<String>>): List of passwords to try for encrypted PDFs. Default: None
- `extract_metadata` (bool): Extract PDF metadata. Default: true
- `hierarchy` (Option<HierarchyConfig>): Hierarchy extraction (H1-H6 from font clustering). Default: None

**Example:**

```rust title="pdf_config.rs"
use kreuzberg::PdfConfig;

let pdf_config = PdfConfig {
    allow_single_column_tables: false,
    extract_images: true,
    passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
    extract_metadata: true,
    hierarchy: None,
};
```

---

### ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Concurrency configuration for controlling parallel extraction.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    pub max_threads: Option<usize>,
}
```

**Fields:**

- `max_threads` (Option<usize>): Maximum number of concurrent threads. Default: None (use system default)

**Example:**

```rust title="concurrency_config.rs"
use kreuzberg::{ExtractionConfig, ConcurrencyConfig};

let config = ExtractionConfig {
    concurrency: Some(ConcurrencyConfig {
        max_threads: Some(4),
    }),
    ..Default::default()
};
```

---

### HierarchyConfig

PDF hierarchy extraction (heading levels from font size clustering). Used when `PdfConfig.hierarchy` is set.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyConfig {
    pub enabled: bool,
    pub k_clusters: usize,
    pub include_bbox: bool,
    pub ocr_coverage_threshold: Option<f32>,
}
```

**Fields:**

- `enabled` (bool): Enable hierarchy extraction. Default: true
- `k_clusters` (usize): Number of font size clusters (1-7, typically 6 for H1-H6). Default: 6
- `include_bbox` (bool): Include bounding box in hierarchy blocks. Default: true
- `ocr_coverage_threshold` (Option<f32>): Trigger OCR when text blocks cover less than this fraction of page (0.0-1.0). Default: None

---

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Layout detection configuration (requires `layout-detection` feature). Analyzes document structure using RT-DETR/YOLO models.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDetectionConfig {
    pub preset: String,
    pub confidence_threshold: Option<f32>,
    pub apply_heuristics: bool,
}
```

**Fields:**

- `preset` (String): Model selection preset. `"fast"` (YOLOv8) or `"accurate"` (RT-DETR). Default: `"fast"`
- `confidence_threshold` (Option<f32>): Confidence threshold for layout detection (0.0-1.0). Default: None (use model default)
- `apply_heuristics` (bool): Apply post-processing heuristics to improve layout grouping. Default: true

---

### OcrElementConfig

OCR element extraction configuration (bounding geometry, confidence, hierarchy).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcrElementConfig {
    pub include_elements: bool,
    pub min_level: OcrElementLevel,   // Word | Line | Block | Page
    pub min_confidence: f64,
    pub build_hierarchy: bool,
}
```

**Fields:**

- `include_elements` (bool): Populate `ExtractionResult.ocr_elements`. Default: false
- `min_level` (OcrElementLevel): Minimum level to include (Word, Line, Block, Page). Default: Line
- `min_confidence` (f64): Minimum recognition confidence (0.0-1.0). Default: 0.0
- `build_hierarchy` (bool): Populate `parent_id` from spatial containment (Tesseract). Default: false

---

### ChunkingConfig

Text chunking configuration for splitting long documents (character-based, with optional embeddings).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    pub max_characters: usize,
    pub overlap: usize,
    pub trim: bool,
    pub chunker_type: ChunkerType,
    pub sizing: ChunkSizing,
    pub embedding: Option<EmbeddingConfig>,
    pub preset: Option<String>,
}

pub enum ChunkerType {
    Text,
    Markdown,
}
```

**Fields:**

- `max_characters` (usize): Maximum characters per chunk. Default: 1000
- `overlap` (usize): Overlap between chunks in characters. Default: 200
- `trim` (bool): Trim whitespace from chunk boundaries. Default: true
- `chunker_type` (ChunkerType): Text or Markdown-aware splitter. Default: Text
- `sizing` (ChunkSizing): How chunk size is measured. Default: `Characters` (Unicode character count). Set to `Tokenizer { model }` for token-based sizing (requires `chunking-tokenizers` feature).
- `embedding` (Option<EmbeddingConfig>): Optional embedding generation for chunks. Default: None
- `preset` (Option<String>): Named preset overriding individual settings. Default: None

---

### EmbeddingConfig

Embedding generation for text chunks (requires `embeddings` feature).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model: EmbeddingModelType,
    pub normalize: bool,
    pub batch_size: usize,
    pub show_download_progress: bool,
    pub cache_dir: Option<PathBuf>,
}
```

**Fields:**

- `model` (EmbeddingModelType): Model to use. Default: Preset { name: "balanced" }
- `normalize` (bool): Normalize embedding vectors (for cosine similarity). Default: true
- `batch_size` (usize): Batch size for embedding generation. Default: 32
- `show_download_progress` (bool): Show model download progress. Default: false
- `cache_dir` (Option<PathBuf>): Custom cache directory; default `~/.cache/kreuzberg/embeddings/`. Default: None

**EmbeddingModelType** variants: `Preset { name: String }`, `FastEmbed { model, dimensions }` (with `embeddings`), `Custom { model_id, dimensions }`.

---

### SecurityLimits

Archive extraction security limits (requires `archives` feature). Prevents decompression bombs and DoS.

**Definition:**

```rust title="Rust"
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityLimits {
    pub max_archive_size: usize,
    pub max_compression_ratio: usize,
    pub max_files_in_archive: usize,
    pub max_nesting_depth: usize,
    pub max_entity_length: usize,
    pub max_content_size: usize,
    pub max_iterations: usize,
    pub max_xml_depth: usize,
    pub max_table_cells: usize,
}
```

**Fields:**

- `max_archive_size` (usize): Maximum uncompressed archive size in bytes. Default: 500 MB
- `max_compression_ratio` (usize): Max compression ratio before flagging (for example 100:1). Default: 100
- `max_files_in_archive` (usize): Max files in archive. Default: 10,000
- `max_nesting_depth` (usize): Max nesting depth. Default: 100
- `max_entity_length` (usize): Max entity/string length. Default: 32
- `max_content_size` (usize): Max string growth per document. Default: 100 MB
- `max_iterations` (usize): Max iterations per operation. Default: 10,000,000
- `max_xml_depth` (usize): Max XML depth. Default: 100
- `max_table_cells` (usize): Max cells per table. Default: 100,000

---

### LanguageDetectionConfig

Language detection configuration.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetectionConfig {
    pub enabled: bool,
    pub min_confidence: f64,
    pub detect_multiple: bool,
}
```

**Fields:**

- `enabled` (bool): Enable language detection. Default: true
- `min_confidence` (f64): Minimum confidence threshold (0.0-1.0). Default: 0.8
- `detect_multiple` (bool): Detect multiple languages in the document. Default: false

---

### TokenReductionConfig

Token reduction configuration for reducing token count in extracted text.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenReductionConfig {
    pub mode: String,                      // "off" | "light" | "moderate" | "aggressive" | "maximum"
    pub preserve_important_words: bool,
}
```

**Fields:**

- `mode` (String): Reduction mode. Default: "off"
- `preserve_important_words` (bool): Preserve capitalized and technical terms. Default: true

---

### PostProcessorConfig

Post-processor pipeline configuration (enable/disable, whitelist/blacklist).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostProcessorConfig {
    pub enabled: bool,
    pub enabled_processors: Option<Vec<String>>,
    pub disabled_processors: Option<Vec<String>>,
}
```

**Fields:**

- `enabled` (bool): Enable post-processors. Default: true
- `enabled_processors` (Option<Vec<String>>): Whitelist of processor names to run (None = all enabled). Default: None
- `disabled_processors` (Option<Vec<String>>): Blacklist of processor names to skip. Default: None

**Methods:**

- `build_lookup_sets(&mut self)`: Pre-compute HashSets for O(1) processor name lookups.

---

### ImageExtractionConfig

Image extraction from documents (PDF, Office, etc.).

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageExtractionConfig {
    pub extract_images: bool,
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}
```

**Fields:**

- `extract_images` (bool): Extract images from documents. Default: true
- `target_dpi` (i32): Target DPI for image normalization. Default: 300
- `max_image_dimension` (i32): Maximum width or height in pixels. Default: 4096
- `auto_adjust_dpi` (bool): Automatically adjust DPI based on content. Default: true
- `min_dpi` (i32): Minimum DPI threshold. Default: 72
- `max_dpi` (i32): Maximum DPI threshold. Default: 600

---

### PageConfig

Page extraction and page-marker options.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageConfig {
    pub extract_pages: bool,
    pub insert_page_markers: bool,
    pub marker_format: String,   // use {page_num} placeholder
}
```

**Fields:**

- `extract_pages` (bool): Populate `ExtractionResult.pages` with per-page content. Default: false
- `insert_page_markers` (bool): Insert page markers into the main content string. Default: false
- `marker_format` (String): Format string for markers (for example `"\n\n<!-- PAGE {page_num} -->\n\n"`). Default: `"\n\n<!-- PAGE {page_num} -->\n\n"`

---

## Results & Types

### ExtractionResult

Result struct returned by all extraction functions.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub annotations: Option<Vec<PdfAnnotation>>,
    pub chunks: Option<Vec<Chunk>>,
    pub content: String,
    pub detected_languages: Option<Vec<String>>,
    pub djot_content: Option<DjotContent>,
    pub document: Option<DocumentStructure>,
    pub elements: Option<Vec<Element>>,
    pub extracted_keywords: Option<Vec<ExtractedKeyword>>,
    pub images: Option<Vec<ExtractedImage>>,
    pub metadata: Metadata,
    pub mime_type: Cow<'static, str>,
    pub ocr_elements: Option<Vec<OcrElement>>,
    pub pages: Option<Vec<PageContent>>,
    pub processing_warnings: Vec<ProcessingWarning>,
    pub quality_score: Option<f64>,
    pub tables: Vec<Table>,
}
```

**Fields:**

- `annotations` (Option<Vec<PdfAnnotation>>): Extracted PDF annotations and highlights
- `chunks` (Option<Vec<Chunk>>): Text chunks when chunking is configured
- `content` (String): Extracted text content
- `detected_languages` (Option<Vec<String>>): Detected language codes (ISO 639-1)
- `djot_content` (Option<DjotContent>): Rich Djot structure when extracting Djot documents
- `document` (Option<DocumentStructure>): Hierarchical document tree when `include_document_structure` is true
- `elements` (Option<Vec<Element>>): Semantic elements (headings, paragraphs, etc.)
- `extracted_keywords` (Option<Vec<ExtractedKeyword>>): Extracted keywords (RAKE/YAKE)
- `images` (Option<Vec<ExtractedImage>>): Extracted images when image extraction is configured
- `metadata` (Metadata): Document metadata (format-specific fields)
- `mime_type` (Cow<'static, str>): MIME type of the processed document
- `ocr_elements` (Option<Vec<OcrElement>>): Granular OCR text blocks with bounding boxes
- `pages` (Option<Vec<PageContent>>): Per-page content when page extraction is enabled
- `processing_warnings` (Vec<ProcessingWarning>): Non-fatal warnings encountered during extraction
- `quality_score` (Option<f64>): Document quality estimation score
- `tables` (Vec<Table>): Vector of extracted tables

**Example:**

```rust title="result_access.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;

    // Access extraction result fields
    println!("Content: {}", result.content);
    println!("MIME type: {}", result.mime_type);
    println!("Tables: {}", result.tables.len());

    // Display detected languages if available
    if let Some(langs) = result.detected_languages {
        println!("Languages: {}", langs.join(", "));
    }

    Ok(())
}
```

---

### Chunk

A text chunk with optional embedding and metadata (when chunking is enabled).

**Definition:**

```rust title="Rust"
pub struct Chunk {
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: ChunkMetadata,
}
```

**Fields:**

- `content` (String): The text content of this chunk
- `embedding` (Option<Vec<f32>>): Embedding vector (when `ChunkingConfig.embedding` is set)
- `metadata` (ChunkMetadata): Byte offsets, chunk index, page range, token count

---

### ExtractedImage

Extracted image from a document (raw bytes and metadata).

**Definition:**

```rust title="Rust"
pub struct ExtractedImage {
    pub data: Bytes,
    pub format: Cow<'static, str>,
    pub image_index: usize,
    pub page_number: Option<usize>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub colorspace: Option<String>,
    pub bits_per_component: Option<u32>,
    pub is_mask: bool,
    pub description: Option<String>,
    pub ocr_result: Option<Box<ExtractionResult>>,
}
```

**Fields:**

- `data` (Bytes): Raw image bytes (PNG, JPEG, WebP, etc.)
- `format` (Cow<'static, str>): Image format (for example "jpeg", "png")
- `image_index` (usize): Zero-based position in document
- `page_number` (Option<usize>): Page/slide number (1-indexed)
- `width` / `height` (Option<u32>): Dimensions in pixels
- `colorspace` (Option<String>): for example "RGB", "CMYK", "Gray"
- `bits_per_component` (Option<u32>): for example 8, 16
- `is_mask` (bool): Whether this image is a mask. Default: false
- `description` (Option<String>): Optional description
- `ocr_result` (Option<Box<ExtractionResult>>): Nested OCR result if image was OCRed

#### Pages

**Type**: `Option<Vec<PageContent>>`

Per-page extracted content when page extraction is enabled via `PageConfig.extract_pages = true`.

Each page contains:

- `page_number` (usize): Page number (1-indexed)
- `content` (String): Text content for that page
- `tables` (Vec<Arc<Table>>): Tables on that page
- `images` (Vec<Arc<ExtractedImage>>): Images on that page
- `hierarchy` (Option<PageHierarchy>): Heading levels (H1-H6) when hierarchy extraction is enabled
- `is_blank` (Option<bool>): Whether the page is considered blank (no meaningful text/tables/images)

**Example:**

```rust title="page_extraction.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, PageConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_file_sync("document.pdf", None, &config)?;

    if let Some(pages) = result.pages {
        for page in pages {
            println!("Page {}:", page.page_number);
            println!("  Content: {} chars", page.content.len());
            println!("  Tables: {}", page.tables.len());
            println!("  Images: {}", page.images.len());
        }
    }

    Ok(())
}
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```rust title="iterate_pages.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, PageConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            insert_page_markers: true,
            marker_format: "\n\n--- Page {page_num} ---\n\n".to_string(),
        }),
        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None, &config)?;

    // Access combined content with page markers
    println!("Combined content with markers:");
    println!("{}", &result.content[..result.content.len().min(500)]);
    println!();

    // Access per-page content
    if let Some(pages) = result.pages {
        for page in pages {
            println!("Page {}:", page.page_number);
            println!("  {}", &page.content[..page.content.len().min(100)]);
            if !page.tables.is_empty() {
                println!("  Found {} table(s)", page.tables.len());
            }
            if !page.images.is_empty() {
                println!("  Found {} image(s)", page.images.len());
            }
        }
    }

    Ok(())
}
```

---

### Metadata

Document metadata with format-specific fields.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub abstract_text: Option<String>,

    /// Deprecated: Prefer using typed fields instead of dynamic access.
    /// This index signature may be removed in a future version.
    pub additional: HashMap<String, serde_json::Value>,

    pub authors: Option<Vec<String>>,
    pub category: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
    pub document_version: Option<String>,
    pub error: Option<ErrorMetadata>,
    pub extraction_duration_ms: Option<u64>,
    pub format: Option<FormatMetadata>,
    pub image_preprocessing: Option<ImagePreprocessingMetadata>,
    pub json_schema: Option<serde_json::Value>,
    pub keywords: Option<Vec<String>>,
    pub language: Option<String>,
    pub modified_at: Option<String>,
    pub modified_by: Option<String>,
    pub output_format: Option<String>,
    pub pages: Option<PageStructure>,
    pub subject: Option<String>,
    pub tags: Vec<String>,
    pub title: Option<String>,
}
```

**Example:**

```rust title="metadata_access.rs"
let result = extract_file_sync("document.pdf", None, &config)?;
let metadata = &result.metadata;

// Access common and format-specific metadata
if let Some(title) = &metadata.title {
    println!("Title: {}", title);
}
// Format-specific data is in metadata.format (FormatMetadata enum)
// Serialized JSON includes a "format_type" discriminator
```

**Fields (summary):**

- `title`: Document title
- `subject`: Document subject
- `authors`: Document authors
- `keywords`: Document keywords
- `language`: Document language
- `created_at`: Document creation date
- `modified_at`: Document modification date
- `created_by`: Document creator
- `modified_by`: Document modifier

See the Types Reference for complete metadata field documentation.

---

### Table

Extracted table structure.

**Definition:**

```rust title="Rust"
#[derive(Debug, Clone)]
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub markdown: String,
    pub page_number: usize,
}
```

**Fields:**

- `cells` (Vec<Vec<String>>): 2D vector of table cells (rows x columns)
- `markdown` (String): Table rendered as markdown
- `page_number` (usize): Page number where table was found (1-indexed)

**Example:**

```rust title="table_processing.rs"
let result = extract_file_sync("invoice.pdf", None, &config)?;

// Process all extracted tables
for table in &result.tables {
    println!("Table on page {}:", table.page_number);
    println!("{}", table.markdown);
    println!();
}
```

---

### Element (element-based output)

When `result_format` is `ElementBased`, `ExtractionResult.elements` contains semantic elements.

**Types:**

- **Element**: `element_id` (ElementId), `element_type` (ElementType), `text` (String), `metadata` (ElementMetadata)
- **ElementType**: Title, NarrativeText, Heading, ListItem, Table, Image, PageBreak, CodeBlock, BlockQuote, Footer, Header
- **ElementId**: Opaque string ID. Use `ElementId::new(s)?` to construct; implements `AsRef<str>`, `Display`
- **ElementMetadata**: `page_number`, `filename`, `coordinates` (Option<BoundingBox>), `element_index`, `additional`
- **BoundingBox**: `x0`, `y0`, `x1`, `y1` (f64) for left, bottom, right, top

---

### OcrElement (OCR element-based output)

When `OcrElementConfig.include_elements` is true, `ExtractionResult.ocr_elements` contains structured OCR results.

**OcrElement fields:** `text`, `geometry` (OcrBoundingGeometry), `confidence` (OcrConfidence), `level` (OcrElementLevel), `rotation` (Option<OcrRotation>), `page_number`, `parent_id`, `backend_metadata`.

**Related types:** OcrBoundingGeometry (Rectangle or Quadrilateral; methods `to_aabb()`, `center()`, `overlaps()`), OcrConfidence (`detection`, `recognition`; `from_tesseract()`, `from_paddle()`), OcrRotation (`angle_degrees`, `confidence`; `from_paddle()`), OcrElementLevel (Word, Line, Block, Page).

---

### DocumentStructure

When `include_document_structure` is true, `ExtractionResult.document` contains a hierarchical tree: **DocumentStructure** (root with `children: Vec<DocumentNode>`), **DocumentNode** (content layer, node content, children, bounding box, page number), **ContentLayer** (Body, Header, Footer, Footnote), **NodeContent** (text, table grid, annotations). Used for heading-driven sections, table grids, and inline annotations.

---

### ChunkMetadata

Metadata for a single text chunk.

**Definition:**

```rust title="Rust"
pub struct ChunkMetadata {
    pub byte_start: usize,
    pub byte_end: usize,
    pub token_count: Option<usize>,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub first_page: Option<usize>,
    pub last_page: Option<usize>,
    pub heading_context: Option<HeadingContext>,
}
```

**Fields:**

- `byte_start` (usize): UTF-8 byte offset in content (inclusive)
- `byte_end` (usize): UTF-8 byte offset in content (exclusive)
- `token_count` (Option<usize>): Token count from embedding tokenizer (if embeddings enabled)
- `chunk_index` (usize): Zero-based index of this chunk in the document
- `total_chunks` (usize): Total number of chunks in the document
- `first_page` (Option<usize>): First page this chunk spans (1-indexed, when page tracking enabled)
- `last_page` (Option<usize>): Last page this chunk spans (1-indexed, when page tracking enabled)
- `heading_context` (Option<HeadingContext>): Heading hierarchy when using Markdown chunker. Only populated when `ChunkerType::Markdown` is used.

**Page tracking:** When `PageStructure.boundaries` is available and chunking is enabled, `first_page` and `last_page` are automatically calculated based on byte offsets.

**Example:**

```rust title="chunk_metadata.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, ChunkingConfig, PageConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        chunking: Some(ChunkingConfig {
            max_characters: 500,
            overlap: 50,
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("document.pdf", None, &config)?;

    if let Some(chunks) = result.chunks {
        for chunk in chunks {
            let meta = &chunk.metadata;
            let page_info = match (meta.first_page, meta.last_page) {
                (Some(first), Some(last)) if first == last => {
                    format!(" (page {})", first)
                }
                (Some(first), Some(last)) => {
                    format!(" (pages {}-{})", first, last)
                }
                _ => String::new(),
            };

            println!(
                "Chunk [{}:{}] index {}/{} {}",
                meta.byte_start,
                meta.byte_end,
                meta.chunk_index,
                meta.total_chunks,
                page_info
            );
            if let Some(ctx) = &chunk.metadata.heading_context {
                for heading in &ctx.headings {
                    println!("  h{}: {}", heading.level, heading.text);
                }
            }
        }
    }

    Ok(())
}
```

---

## Embeddings

### Embed_texts()

Generate embeddings for a list of texts.

**Signature:**

```rust
pub fn embed_texts<T: AsRef<str>>(
    texts: &[T],
    config: &EmbeddingConfig,
) -> Result<Vec<Vec<f32>>>
```

**Parameters:**

- `texts`: Slice of strings to embed.
- `config`: Embedding configuration.

**Returns:** `Vec<Vec<f32>>` — one embedding vector per input text.

Requires the `embeddings` feature.

**Example:**

--8<-- "snippets/rust/utils/standalone_embed.md"

---

### Embed_texts_async()

Async variant of `embed_texts()`. Requires both the `embeddings` and `tokio-runtime` features.

**Signature:**

```rust
pub async fn embed_texts_async(
    texts: Vec<String>,
    config: &EmbeddingConfig,
) -> Result<Vec<Vec<f32>>>
```

Same return type as `embed_texts()`.

---

## PDF Rendering

!!! Info "Added in v4.6.2"

### Render_pdf_page_to_png()

Render a single page of a PDF as a PNG image.

**Signature:**

```rust title="Rust"
pub fn render_pdf_page_to_png(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<u8>, KreuzbergError>
```

**Parameters:**

- `pdf_bytes` (&[u8]): Raw PDF file bytes
- `page_index` (usize): Zero-based page index to render
- `dpi` (Option<i32>): Resolution for rendering (default 150 if None)
- `password` (Option<&str>): Optional password for encrypted PDFs

**Returns:**

- `Result<Vec<u8>, KreuzbergError>`: PNG-encoded bytes or an error

**Example:**

```rust title="render_page.rs"
use kreuzberg::pdf::render_pdf_page_to_png;

let pdf_bytes = std::fs::read("document.pdf")?;
let png = render_pdf_page_to_png(&pdf_bytes, 0, Some(150), None)?;
std::fs::write("first_page.png", &png)?;
```

---

### Render_pdf_file_to_png_pages()

Render each page of a PDF as a PNG image.

**Signature:**

```rust title="Rust"
pub fn render_pdf_file_to_png_pages(
    pdf_bytes: &[u8],
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<Vec<u8>>, KreuzbergError>
```

**Parameters:**

- `pdf_bytes` (&[u8]): Raw PDF file bytes
- `dpi` (Option<i32>): Resolution for rendering (default 150 if None)
- `password` (Option<&str>): Optional password for encrypted PDFs

**Returns:**

- `Result<Vec<Vec<u8>>, KreuzbergError>`: Vector of PNG-encoded byte vectors, one per page

**Example:**

```rust title="render_all_pages.rs"
use kreuzberg::pdf::render_pdf_file_to_png_pages;

let pdf_bytes = std::fs::read("document.pdf")?;
let pages = render_pdf_file_to_png_pages(&pdf_bytes, Some(150), None)?;
for (i, png) in pages.iter().enumerate() {
    std::fs::write(format!("page_{i}.png"), png)?;
}
```

---

### PdfPageIterator

A more memory-efficient alternative to `render_pdf_file_to_png_pages` when memory is a concern or when pages should be processed as they are rendered (for example, sending each page to a vision model for OCR). Renders one page at a time, releasing memory for each page before rendering the next.

**Signature:**

```rust title="Rust"
pub struct PdfPageIterator { ... }

impl PdfPageIterator {
    pub fn from_file(path: &str, dpi: Option<i32>, password: Option<&str>) -> Result<Self, KreuzbergError>;
    pub fn page_count(&self) -> usize;
}

impl Iterator for PdfPageIterator {
    type Item = Result<(usize, Vec<u8>), KreuzbergError>;
}
```

**Example:**

```rust title="iterate_pages.rs"
use kreuzberg::pdf::PdfPageIterator;

let iter = PdfPageIterator::from_file("document.pdf", Some(150), None)?;
for result in iter {
    let (page_index, png) = result?;
    std::fs::write(format!("page_{page_index}.png"), &png)?;
}
```

---

## Error Handling

### KreuzbergError

All errors are returned as `KreuzbergError` enum. Many variants carry `{ message, source }` for chaining.

**Definition (summary):**

```rust title="error_handling.rs"
#[derive(Debug, thiserror::Error)]
pub enum KreuzbergError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {message}")]
    Validation { message: String, source: Option<Box<dyn Error + Send + Sync>> },

    #[error("Parsing error: {message}")]
    Parsing { message: String, source: Option<Box<dyn Error + Send + Sync>> },

    #[error("OCR error: {message}")]
    Ocr { message: String, source: Option<Box<dyn Error + Send + Sync>> },

    #[error("Cache error: {message}")]
    Cache { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Image processing error: {message}")]
    ImageProcessing { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Serialization error: {message}")]
    Serialization { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Missing dependency: {0}")]
    MissingDependency(String),

    #[error("Plugin error in '{plugin_name}': {message}")]
    Plugin { message: String, plugin_name: String },

    #[error("Lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("{0}")]
    Other(String),
}
```

**Error Handling:**

```rust title="error_handling.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig, KreuzbergError};

fn process_file(path: &str) -> kreuzberg::Result<String> {
    let config = ExtractionConfig::default();

    match extract_file_sync(path, None, &config) {
        Ok(result) => Ok(result.content),
        Err(KreuzbergError::Io(e)) => {
            eprintln!("File system error: {}", e);
            Err(KreuzbergError::Io(e))
        }
        Err(KreuzbergError::Validation { message, .. }) => {
            eprintln!("Invalid input: {}", message);
            Err(KreuzbergError::validation(message))
        }
        Err(KreuzbergError::Parsing { message, .. }) => {
            eprintln!("Failed to parse document: {}", message);
            Err(KreuzbergError::parsing(message))
        }
        Err(e) => Err(e),
    }
}
```

**Using the `?` operator:**

```rust title="simple_error_handling.rs"
fn main() -> kreuzberg::Result<()> {
    // Use ? operator for simple error propagation
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```

See [Error Handling Reference](errors.md) for detailed error documentation.

---

## Plugin System

### Document Extractors

Register custom document extractors for new file formats. Extractors implement both `Plugin` (name, version, initialize, shutdown) and `DocumentExtractor` (extract_bytes, extract_file, supported_mime_types, priority).

**Trait (summary):**

```rust title="Rust"
pub trait Plugin {
    fn name(&self) -> &str;
    fn version(&self) -> String;
    fn initialize(&self) -> Result<()> { Ok(()) }
    fn shutdown(&self) -> Result<()> { Ok(()) }
}

#[async_trait]
pub trait DocumentExtractor: Plugin + Send + Sync {
    async fn extract_bytes(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
        -> Result<ExtractionResult>;
    async fn extract_file(&self, path: &Path, mime_type: &str, config: &ExtractionConfig)
        -> Result<ExtractionResult>;
    fn supported_mime_types(&self) -> &[&str];
    fn priority(&self) -> i32;
}
```

**Registration:**

Either use the registry directly or the helper:

```rust title="plugin_registration.rs"
use kreuzberg::plugins::registry::get_document_extractor_registry;
use std::sync::Arc;

let registry = get_document_extractor_registry();
let mut reg = registry.write().unwrap();
reg.register(Arc::new(MyCustomExtractor))?;
```

Or: `kreuzberg::plugins::register_extractor(Arc::new(MyCustomExtractor))?`. The registry also provides `get(mime_type)`, `list()`, `remove(name)`, and `shutdown_all()`.

---

## MIME Type Detection

### Detect_mime_type()

Detect MIME type from file path (by extension).

**Signature:**

```rust title="Rust"
pub fn detect_mime_type(
    file_path: impl AsRef<Path>,
    check_exists: bool
) -> Result<String>
```

**Parameters:**

- `file_path` (impl AsRef<Path>): Path to the file (used for extension only when `check_exists` is false)
- `check_exists` (bool): If true, returns `Err(KreuzbergError::Io)` when the file does not exist; if false, only the path extension is used and the file need not exist

**Returns:**

- `Result<String>`: Detected MIME type string, or error if extension is unknown or (when `check_exists` is true) file not found

**Example:**

```rust title="mime_detection.rs"
use kreuzberg::detect_mime_type;

// Detect MIME type from file path (file must exist)
let mime_type = detect_mime_type("document.pdf", true)?;
println!("MIME type: {}", mime_type); // "application/pdf"

// Detect from path only, without checking existence
let mime_type = detect_mime_type("document.pdf", false)?;
```

---

### Validate_mime_type()

Validate that a MIME type is supported. Returns the validated (possibly normalized) MIME type string, or an error if unsupported.

**Signature:**

```rust title="Rust"
pub fn validate_mime_type(mime_type: &str) -> Result<String>
```

**Returns:**

- `Result<String>`: The validated MIME type string, or `KreuzbergError::UnsupportedFormat` if not supported

**Example:**

```rust title="mime_validation.rs"
use kreuzberg::validate_mime_type;

let mime = validate_mime_type("application/pdf")?;
println!("PDF is supported: {}", mime);
```

---

### Detect_mime_type_from_bytes()

Detect MIME type from raw bytes (magic numbers / content sniffing).

**Signature:**

```rust title="Rust"
pub fn detect_mime_type_from_bytes(content: &[u8]) -> Result<String>
```

**Example:**

```rust title="mime_from_bytes.rs"
use kreuzberg::detect_mime_type_from_bytes;

let data = std::fs::read("document.pdf")?;
let mime = detect_mime_type_from_bytes(&data)?;
```

---

### Detect_or_validate()

Get MIME type from path or validate a provided MIME type. Returns the MIME type if path is given (from extension) or if the provided MIME is valid.

**Signature:**

```rust title="Rust"
pub fn detect_or_validate(path: Option<&Path>, mime_type: Option<&str>) -> Result<String>
```

---

### Get_extensions_for_mime()

Return file extensions associated with a MIME type.

**Signature:**

```rust title="Rust"
pub fn get_extensions_for_mime(mime_type: &str) -> Result<Vec<String>>
```

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. Requires the `liter-llm` feature flag. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `StructuredExtractionConfig` to extract structured data from documents using an LLM:

--8<-- "snippets/rust/llm/structured_extraction.md"

The `structured_output` field on `ExtractionResult` contains the JSON string conforming to the provided schema:

```rust title="access_structured_output.rs"
let result = extract_file("paper.pdf", None, &config).await?;
if let Some(ref output) = result.structured_output {
    let data: serde_json::Value = serde_json::from_str(output)?;
    println!("Title: {}", data["title"]);
}
```

### VLM OCR

Use a vision-language model as an OCR backend by setting `backend` to `"vlm"` with a `vlm_config`:

```rust title="vlm_ocr.rs"
use kreuzberg::{ExtractionConfig, OcrConfig, LlmConfig};

let config = ExtractionConfig {
    force_ocr: true,
    ocr: Some(OcrConfig {
        backend: "vlm".to_string(),
        vlm_config: Some(LlmConfig {
            model: "openai/gpt-4o-mini".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file("scan.pdf", None, &config).await?;
```

### LLM Embeddings

Generate embeddings using an LLM provider via `EmbeddingModelType::Llm`:

```rust title="llm_embeddings.rs"
use kreuzberg::{embed, EmbeddingConfig, EmbeddingModelType, LlmConfig};

let config = EmbeddingConfig {
    model_type: EmbeddingModelType::Llm,
    llm: Some(LlmConfig {
        model: "openai/text-embedding-3-small".to_string(),
        ..Default::default()
    }),
    ..Default::default()
};

let vectors = embed(&["hello world"], &config).await?;
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## Code Intelligence

Kreuzberg uses [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) to parse and analyze source code files across 248 programming languages. When extracting code files, the result metadata includes structural analysis, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available via `FormatMetadata::Code` in the `metadata.format` field. Requires the `tree-sitter` feature flag.

```rust title="code_intelligence.rs"
use kreuzberg::{
    extract_file_sync, ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig,
    types::FormatMetadata,
};

fn main() -> kreuzberg::Result<()> {
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

    let result = extract_file_sync("app.py", None, &config)?;

    if let Some(FormatMetadata::Code(ref code)) = result.metadata.format {
        println!("Language: {}", code.language);
        println!("Functions/classes: {}", code.structure.len());
        println!("Imports: {}", code.imports.len());

        for item in &code.structure {
            println!("  {:?}: {:?} at line {}", item.kind, item.name, item.span.start_line);
        }

        for chunk in &code.chunks {
            println!("Chunk: {}...", &chunk.content[..50.min(chunk.content.len())]);
        }
    }

    Ok(())
}
```

For configuration details, see the [Code Intelligence Guide](../guides/code-intelligence.md).

---

## Complete Documentation

For complete Rust API documentation with all types, traits, and functions:

```bash title="Terminal"
cargo doc --open --no-deps
```

Or visit [docs.rs/Kreuzberg](https://docs.rs/kreuzberg)
