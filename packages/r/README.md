<div align="center">

# kreuzberg

[![Docs](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![R](https://img.shields.io/badge/R-%3E%3D%204.2-blue)](https://cran.r-project.org/)

**Extract text, tables, images, and metadata from 75+ file formats including PDF, Office documents, and images. R bindings with idiomatic R API and native performance via extendr.**

[Documentation](https://kreuzberg.dev) · [API Reference](https://kreuzberg.dev/reference/api-r/) · [Discord](https://discord.gg/kreuzberg)

</div>

## Installation

```r
# Install from source (requires Rust toolchain)
# install.packages("kreuzberg")

# Or install from GitHub
# remotes::install_github("kreuzberg-dev/kreuzberg", subdir = "packages/r")

# Or install from r-universe
# install.packages("kreuzberg", repos = "https://kreuzberg-dev.r-universe.dev")
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
