# Xberg

{% include 'partials/badges.html.jinja' %}

{{ description }}

## What This Package Provides

- **Python-native extraction** — sync and async APIs for files, bytes, URLs, and batch ingestion.
- **Structured results** — text, tables, images, metadata, language detection, chunks, and warnings in typed Python objects.
- **OCR choices** — Tesseract, EasyOCR, PaddleOCR, and VLM OCR where configured.
- **Same Rust engine as every binding** — behavior matches the Node.js, Ruby, Go, Java, .NET, PHP, Elixir, R, Dart, Swift, Zig, WASM, and C FFI packages.

## Installation

```bash
pip install xberg
```

### With OCR Support

```bash
pip install "xberg[easyocr]"
pip install "xberg[paddleocr]"
```

### All Features

```bash
pip install "xberg[all]"
```

## Quick Start

### Basic Usage

{{ 'getting-started/basic_usage.md' | include_snippet('python') }}

### Simple Extraction

{{ 'getting-started/extract.md' | include_snippet('python') }}

### Reading Content

{{ 'getting-started/read_content.md' | include_snippet('python') }}

## OCR Support

### Using OCR

{{ 'getting-started/extract_with_ocr.md' | include_snippet('python') }}

### EasyOCR (GPU-Accelerated)

```python
from xberg import extract, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

result = extract(
    "photo.jpg",
    config=config,
    easyocr_kwargs={"use_gpu": True}
)
```

### PaddleOCR (Complex Layouts)

```python
from xberg import extract, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="ch")
)

result = extract(
    "invoice.pdf",
    config=config,
)
```

## Table Extraction

```python
from xberg import extract, ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        tesseract_config=TesseractConfig(
            enable_table_detection=True
        )
    )
)

result = extract("invoice.pdf", config=config)

for table in result.tables:
    print(table.markdown)
    print(table.cells)
```

## Configuration

### Complete Configuration Example

```python
from xberg import (
    extract,
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ChunkingConfig,
    ImageExtractionConfig,
    PdfConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
)

config = ExtractionConfig(
    use_cache=True,
    enable_quality_processing=True,
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            enable_table_detection=True,
            min_confidence=50.0,
        ),
    ),
    force_ocr=False,
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    ),
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
    ),
    pdf_options=PdfConfig(
        extract_images=True,
        passwords=["password1", "password2"],
        extract_metadata=True,
    ),
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_important_words=True,
    ),
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=False,
    ),
)

result = extract("document.pdf", config=config)
```

### HTML Conversion Options & Batch Concurrency

```python
from xberg import ExtractionConfig

config = ExtractionConfig(
    max_concurrent_extractions=8,
    html_options={
        "extract_metadata": True,
        "wrap": True,
        "wrap_width": 100,
        "strip_tags": ["script", "style"],
        "preprocessing": {"enabled": True, "preset": "standard"},
    },
)
```

## Metadata Extraction

```python
from xberg import extract

result = extract("document.pdf")

if result.images:
    print(f"Extracted {len(result.images)} inline images")

if result.chunks:
    print(f"First chunk tokens: {result.chunks[0]['metadata']['token_count']}")

print(result.metadata.get("pdf", {}))
print(result.metadata.get("language"))
print(result.metadata.get("format"))

if "pdf" in result.metadata:
    pdf_meta = result.metadata["pdf"]
    print(f"Title: {pdf_meta.get('title')}")
    print(f"Author: {pdf_meta.get('author')}")
    print(f"Pages: {pdf_meta.get('page_count')}")
    print(f"Created: {pdf_meta.get('creation_date')}")
```

## Password-Protected PDFs

```python
from xberg import extract, ExtractionConfig, PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2", "password3"]
    )
)

result = extract("protected.pdf", config=config)
```

## Language Detection

```python
from xberg import extract, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract("multilingual.pdf", config=config)
print(result.detected_languages)
```

## Text Chunking

```python
from xberg import extract, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    )
)

result = extract("long_document.pdf", config=config)

for chunk in result.chunks:
    print(chunk)
```

## Extract from Bytes

```python
from xberg import ExtractInput, extract

with open("document.pdf", "rb") as f:
    data = f.read()

result = await extract(ExtractInput.bytes(data, mime_type="application/pdf"))
print(result.content)
```

## API Reference

### Extraction Functions

- `extract(input: ExtractInput, config=None, **kwargs)` – Extract one file or byte input
- `extract_batch(inputs: list[ExtractInput], config=None, **kwargs)` – Extract mixed file and byte inputs
- `ExtractInput.file(path, mime_type=None, **overrides)` – File path input
- `ExtractInput.bytes(data, mime_type, **overrides)` – In-memory bytes input

### Configuration Classes

- `ExtractionConfig` – Main configuration
- `OcrConfig` – OCR settings
- `TesseractConfig` – Tesseract-specific options
- `ChunkingConfig` – Text chunking settings
- `ImageExtractionConfig` – Image extraction settings
- `PdfConfig` – PDF-specific options
- `TokenReductionConfig` – Token reduction settings
- `LanguageDetectionConfig` – Language detection settings

### Result Types

- `ExtractionResult` – Main result object with `content`, `metadata`, `tables`, `detected_languages`, `chunks`
- `ExtractedTable` – Table with `cells`, `markdown`, `page_number`
- `Metadata` – Typed metadata dictionary

### Exceptions

- `XbergError` – Base exception
- `ValidationError` – Invalid configuration or input
- `ParsingError` – Document parsing failure
- `OCRError` – OCR processing failure
- `MissingDependencyError` – Missing optional dependency

## Examples

### Custom Processing

```python
from xberg import extract

result = extract("document.pdf")

text = result.content
text = text.lower()
text = text.replace("old", "new")

print(text)
```

### Multiple Files with Progress

```python
from xberg import extract
from pathlib import Path

files = list(Path("documents").glob("*.pdf"))
results = []

for i, file in enumerate(files, 1):
    print(f"Processing {i}/{len(files)}: {file.name}")
    result = extract(str(file))
    results.append((file.name, result))

for name, result in results:
    print(f"{name}: {len(result.content)} characters")
```

### Filter by Language

```python
from xberg import extract, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract("document.pdf", config=config)

if result.detected_languages and "en" in result.detected_languages:
    print("English document detected")
    print(result.content)
```

## System Requirements

### ONNX Runtime (for ORT-dependent features)

If using embeddings or other ORT-dependent inference features, ONNX Runtime version 1.24+ must be installed:

```bash
# macOS
brew install onnxruntime

# Ubuntu/Debian (download from GitHub - Debian packages may have older versions)
# Download from https://github.com/microsoft/onnxruntime/releases

# Windows
# Download from https://github.com/microsoft/onnxruntime/releases
```

**Important:** Xberg requires ONNX Runtime version 1.24+ for embeddings and other ORT-dependent inference features.

Without ONNX Runtime, ORT-dependent features will raise `MissingDependencyError` with installation instructions.

### Tesseract OCR (Required for OCR)

```bash
brew install tesseract
```

```bash
sudo apt-get install tesseract-ocr
```

### Pandoc (Optional, for some formats)

```bash
brew install pandoc
```

```bash
sudo apt-get install pandoc
```

## Troubleshooting

### Import Error: No module named '\_xberg'

This usually means the Rust extension wasn't built correctly. Try:

```bash
pip install --force-reinstall --no-cache-dir xberg
```

### OCR Not Working

Make sure Tesseract is installed:

```bash
tesseract --version
```

### Memory Issues with Large PDFs

Use streaming or enable chunking:

```python
config = ExtractionConfig(
    chunking=ChunkingConfig(max_chars=1000)
)
```

## PDFium Integration

PDF extraction is powered by PDFium, which is automatically bundled with this package. No system installation required.

### Platform Support

| Platform       | Status | Notes   |
| -------------- | ------ | ------- |
| Linux x86_64   | ✅     | Bundled |
| macOS ARM64    | ✅     | Bundled |
| macOS x86_64   | ✅     | Bundled |
| Windows x86_64 | ✅     | Bundled |

### Binary Size Impact

PDFium adds approximately 8-15 MB to the package size depending on platform. This ensures consistent PDF extraction across all environments without external dependencies.

## Documentation

For comprehensive documentation, visit [https://xberg.io](https://xberg.io)

## Part of Xberg.dev

- [crawlberg](https://github.com/xberg-io/crawlberg) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/xberg-io/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/xberg-io/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/xberg-io/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/xberg-io/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

{{ license }} License - see [LICENSE](../../LICENSE) for details.
