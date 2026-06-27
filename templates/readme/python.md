# Xberg

{% include 'partials/badges.html.jinja' %}

{{ description }}

## What This Package Provides

- **Python-native extraction** — async APIs for URI, bytes, and batch inputs.
- **Structured results** — an `ExtractionResult` envelope with `ExtractedDocument` items, errors, and summary counts.
- **OCR choices** — Tesseract, PaddleOCR, Candle, and VLM OCR where configured.
- **Same Rust engine as every binding** — behavior matches the Node.js, Ruby, Go, Java, .NET, PHP, Elixir, R, Dart, Swift, Zig, WASM, and C FFI packages.

## Installation

```bash
pip install xberg
```

### With OCR Support

```bash
pip install "xberg[paddleocr]"
```

### All Features

```bash
pip install "xberg[all]"
```

## Quick Start

### Basic Usage

```python
import asyncio

from xberg import ExtractInput, extract


async def main() -> None:
    output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
    document = output.results[0]

    print(document.content)
    print(f"Results: {output.summary.results}")


asyncio.run(main())
```

### Simple Extraction

```python
from xberg import ExtractInput, extract

output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
document = output.results[0]

print(document.content)
```

### Reading Content

```python
from xberg import ExtractInput, extract

output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
document = output.results[0]

print(document.content[:500])
```

## OCR Support

### Using OCR

```python
from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng"),
    force_ocr=True,
)

output = await extract(ExtractInput(kind="uri", uri="scanned.pdf"), config)
document = output.results[0]

print(document.content)
```

### PaddleOCR (Complex Layouts)

```python
from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="ch")
)

output = await extract(ExtractInput(kind="uri", uri="invoice.pdf"), config)
document = output.results[0]
```

## Table Extraction

```python
from xberg import ExtractInput, ExtractionConfig, OcrConfig, TesseractConfig, extract

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        tesseract_config=TesseractConfig(
            enable_table_detection=True
        )
    )
)

output = await extract(ExtractInput(kind="uri", uri="invoice.pdf"), config)
document = output.results[0]

for table in document.tables:
    print(table.markdown)
    print(table.cells)
```

## Configuration

### Complete Configuration Example

```python
from xberg import (
    ExtractInput,
    extract,
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ChunkingConfig,
    ImageExtractionConfig,
    PdfConfig,
    TokenReductionOptions,
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
    token_reduction=TokenReductionOptions(
        mode="moderate",
        preserve_important_words=True,
    ),
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=False,
    ),
)

output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config)
document = output.results[0]
```

### HTML Conversion Options & Batch Concurrency

```python
from xberg import ExtractionConfig, HtmlOutputConfig

config = ExtractionConfig(
    max_concurrent_extractions=8,
    html_output=HtmlOutputConfig(
        theme="default",
        class_prefix="xberg",
        embed_css=True,
    ),
)
```

## Metadata Extraction

```python
from xberg import ExtractInput, extract

output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
document = output.results[0]

if document.images:
    print(f"Extracted {len(document.images)} inline images")

if document.chunks:
    print(f"First chunk tokens: {document.chunks[0].metadata.token_count}")

if document.metadata:
    print(document.metadata.title)
    print(document.metadata.language)
    print(document.metadata.format)

print(f"Errors: {output.summary.errors}")
```

## Password-Protected PDFs

```python
from xberg import ExtractInput, ExtractionConfig, PdfConfig, extract

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2", "password3"]
    )
)

output = await extract(ExtractInput(kind="uri", uri="protected.pdf"), config)
document = output.results[0]
```

## Language Detection

```python
from xberg import ExtractInput, ExtractionConfig, LanguageDetectionConfig, extract

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

output = await extract(ExtractInput(kind="uri", uri="multilingual.pdf"), config)
document = output.results[0]

print(document.detected_languages)
```

## Text Chunking

```python
from xberg import ExtractInput, ExtractionConfig, ChunkingConfig, extract

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    )
)

output = await extract(ExtractInput(kind="uri", uri="long_document.pdf"), config)
document = output.results[0]

for chunk in document.chunks:
    print(chunk.content)
```

## Extract from Bytes

```python
from xberg import ExtractInput, extract

with open("document.pdf", "rb") as f:
    data = f.read()

output = await extract(ExtractInput(kind="bytes", bytes=data, mime_type="application/pdf"))
document = output.results[0]

print(document.content)
```

## API Reference

### Extraction Functions

- `await extract(input: ExtractInput, config=None)` – Extract one URI or bytes input.
- `await extract_batch(inputs: list[ExtractInput], config=None)` – Extract multiple URI or bytes inputs.
- `ExtractInput(kind="uri", uri="document.pdf")` – Local path, `file://`, or HTTP(S) URI input.
- `ExtractInput(kind="bytes", bytes=data, mime_type="application/pdf")` – In-memory bytes input.

### Configuration Classes

- `ExtractionConfig` – Main configuration
- `OcrConfig` – OCR settings
- `TesseractConfig` – Tesseract-specific options
- `ChunkingConfig` – Text chunking settings
- `HtmlOutputConfig` – HTML rendering settings
- `ImageExtractionConfig` – Image extraction settings
- `PdfConfig` – PDF-specific options
- `TokenReductionOptions` – Token reduction settings
- `LanguageDetectionConfig` – Language detection settings

### Result Types

- `ExtractionResult` – Envelope with `results`, `errors`, and `summary`.
- `ExtractedDocument` – Per-document item at `output.results[0]` with `content`, `metadata`, `tables`, and chunks.
- `Table` – Table with `cells`, `markdown`, and `page_number`.
- `Metadata` – Typed document metadata.

### Exceptions

- `XbergError` – Base exception
- `ValidationError` – Invalid configuration or input
- `ParsingError` – Document parsing failure
- `OCRError` – OCR processing failure
- `MissingDependencyError` – Missing optional dependency

## Examples

### Custom Processing

```python
from xberg import ExtractInput, extract

output = await extract(ExtractInput(kind="uri", uri="document.pdf"))
document = output.results[0]

text = document.content
text = text.lower()
text = text.replace("old", "new")

print(text)
```

### Multiple Files with Progress

```python
from pathlib import Path

from xberg import ExtractInput, extract_batch

files = list(Path("documents").glob("*.pdf"))

inputs = [
    ExtractInput(kind="uri", uri=str(file))
    for file in files
]

output = await extract_batch(inputs)

for file, document in zip(files, output.results):
    print(f"{file.name}: {len(document.content)} characters")
```

### Filter by Language

```python
from xberg import ExtractInput, ExtractionConfig, LanguageDetectionConfig, extract

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config)
document = output.results[0]

if document.detected_languages and "en" in document.detected_languages:
    print("English document detected")
    print(document.content)
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
