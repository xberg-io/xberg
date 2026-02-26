# kreuzberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://pypi.org/project/kreuzberg/">
    <img src="https://img.shields.io/pypi/v/kreuzberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/node">
    <img src="https://img.shields.io/npm/v/@kreuzberg/node?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/wasm">
    <img src="https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM&color=007ec6" alt="WASM">
  </a>

  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://www.nuget.org/packages/Kreuzberg/">
    <img src="https://img.shields.io/nuget/v/Kreuzberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/kreuzberg/kreuzberg">
    <img src="https://img.shields.io/packagist/v/kreuzberg/kreuzberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/kreuzberg">
    <img src="https://img.shields.io/gem/v/kreuzberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Documentation">
  </a>
</div>

<img width="3384" height="573" alt="Linkedin- Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Extract text, tables, images, and metadata from 75+ file formats including PDF, Office documents, and images. R bindings with idiomatic R API and native performance via extendr.

## Installation

```r
# Install from r-universe (recommended)
install.packages("kreuzberg", repos = "https://kreuzberg-dev.r-universe.dev")

# Or install from GitHub
# remotes::install_github("kreuzberg-dev/kreuzberg", subdir = "packages/r")
```

### System Requirements

- **R** >= 4.2
- **Rust** >= 1.91 (with Cargo)
- **ONNX Runtime** (for ML features)
- **Tesseract** (optional, for OCR)

## Quick Start

```r
library(kreuzberg)

# Extract text from a PDF
result <- extract_file_sync("document.pdf")
cat(result$content)

# Extract with OCR
config <- extraction_config(
  force_ocr = TRUE,
  ocr = ocr_config(backend = "tesseract", language = "eng")
)
result <- extract_file_sync("scanned.pdf", config = config)

# Batch extraction
results <- batch_extract_files_sync(c("doc1.pdf", "doc2.docx", "data.xlsx"))
for (r in results) {
  cat(sprintf("Type: %s, Length: %d\n", r$mime_type, nchar(r$content)))
}
```

## Features

### Supported File Formats (75+)

| Category | Formats |
|---|---|
| Documents | PDF, DOCX, DOC, RTF, ODT, EPUB |
| Spreadsheets | XLSX, XLS, ODS, CSV, TSV |
| Presentations | PPTX, PPT, ODP |
| Images | PNG, JPG, TIFF, BMP, WebP, GIF |
| Web | HTML, XML, XHTML |
| Data | JSON, YAML, TOML |
| Email | EML, MSG |
| Archives | ZIP, TAR, GZ |
| Code | Markdown, plain text, source code |

### Key Capabilities

- **Text extraction** from 75+ formats
- **Table extraction** with structure preservation
- **OCR support** via Tesseract and PaddleOCR
- **Text chunking** for RAG pipelines
- **Language detection**
- **Keyword extraction**
- **Quality scoring**
- **Plugin system** for custom extractors, validators, and post-processors

## OCR Support

```r
library(kreuzberg)

# Tesseract OCR (multi-language)
config <- extraction_config(
  force_ocr = TRUE,
  ocr = ocr_config(backend = "tesseract", language = "eng+deu+fra", dpi = 300L)
)
result <- extract_file_sync("scan.png", config = config)

# PaddleOCR
config <- extraction_config(
  force_ocr = TRUE,
  ocr = ocr_config(backend = "paddle-ocr", language = "en")
)
result <- extract_file_sync("scan.png", config = config)
```

## Configuration

```r
library(kreuzberg)

# Full configuration
config <- extraction_config(
  force_ocr = TRUE,
  ocr = ocr_config(backend = "tesseract", language = "eng"),
  chunking = chunking_config(max_characters = 1000L, overlap = 200L),
  output_format = "markdown",
  enable_quality_processing = TRUE,
  language_detection = list(enabled = TRUE),
  keywords = list(enabled = TRUE)
)

result <- extract_file_sync("document.pdf", config = config)

# Discover config from kreuzberg.toml
config <- discover()
```

## Error Handling

```r
library(kreuzberg)

result <- tryCatch(
  extract_file_sync("document.xyz"),
  UnsupportedFileType = function(e) {
    cat("Unsupported:", conditionMessage(e), "\n")
    NULL
  },
  ValidationError = function(e) {
    cat("Validation:", conditionMessage(e), "\n")
    NULL
  },
  kreuzberg_error = function(e) {
    cat("Error:", conditionMessage(e), "\n")
    NULL
  }
)
```

## Plugin System

```r
library(kreuzberg)

# Register custom post-processor
register_post_processor("cleanup", function(content) {
  gsub("\\s+", " ", trimws(content))
})

# Register custom validator
register_validator("min_length", function(content) {
  nchar(content) >= 10L
})

# List registered plugins
cat("OCR backends:", paste(list_ocr_backends(), collapse = ", "), "\n")
cat("Validators:", paste(list_validators(), collapse = ", "), "\n")
```

## Documentation

- [Full Documentation](https://kreuzberg.dev)
- [R API Reference](https://kreuzberg.dev/reference/api-r/)
- [Configuration Guide](https://kreuzberg.dev/guides/configuration/)
- [OCR Guide](https://kreuzberg.dev/guides/ocr/)
- [Plugin Guide](https://kreuzberg.dev/guides/plugins/)

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT - see [LICENSE](LICENSE) for details.

## Support

- [Discord](https://discord.gg/kreuzberg)
- [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
- [GitHub Discussions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
