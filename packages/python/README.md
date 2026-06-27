# Xberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://github.com/xberg-io/alef">
    <img src="https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6" alt="Bindings">
  </a>
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/xberg">
    <img src="https://img.shields.io/crates/v/xberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://pypi.org/project/xberg/">
    <img src="https://img.shields.io/pypi/v/xberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@xberg-io/xberg">
    <img src="https://img.shields.io/npm/v/@xberg-io/xberg?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@xberg-io/xberg-wasm">
    <img src="https://img.shields.io/npm/v/@xberg-io/xberg-wasm?label=WASM&color=007ec6" alt="WASM">
  </a>
  <a href="https://central.sonatype.com/artifact/io.xberg/xberg">
    <img src="https://img.shields.io/maven-central/v/io.xberg/xberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/go">
    <img src="https://img.shields.io/github/v/tag/xberg-io/xberg?label=Go&color=007ec6&filter=v1*" alt="Go">
  </a>
  <a href="https://www.nuget.org/packages/Xberg/">
    <img src="https://img.shields.io/nuget/v/Xberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/xberg-io/xberg">
    <img src="https://img.shields.io/packagist/v/xberg-io/xberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/xberg">
    <img src="https://img.shields.io/gem/v/xberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://hex.pm/packages/xberg">
    <img src="https://img.shields.io/hexpm/v/xberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://xberg-io.r-universe.dev/xberg">
    <img src="https://img.shields.io/badge/R-xberg-007ec6" alt="R">
  </a>
  <a href="https://pub.dev/packages/xberg">
    <img src="https://img.shields.io/pub/v/xberg?label=Dart&color=007ec6" alt="Dart">
  </a>
  <a href="https://central.sonatype.com/artifact/io.xberg/xberg-android">
    <img src="https://img.shields.io/maven-central/v/io.xberg/xberg-android?label=Kotlin&color=007ec6" alt="Kotlin">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/swift">
    <img src="https://img.shields.io/badge/Swift-SPM-007ec6" alt="Swift">
  </a>
  <a href="https://github.com/xberg-io/xberg/tree/main/packages/zig">
    <img src="https://img.shields.io/badge/Zig-package-007ec6" alt="Zig">
  </a>
  <a href="https://github.com/xberg-io/xberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI">
  </a>
  <a href="https://github.com/xberg-io/xberg/pkgs/container/xberg">
    <img src="https://img.shields.io/badge/Docker-ghcr.io-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <!-- Project Info -->
  <a href="https://github.com/xberg-io/xberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License">
  </a>
  <a href="https://docs.xberg.io">
    <img src="https://img.shields.io/badge/Docs-xberg-007ec6" alt="Documentation">
  </a>
  <a href="https://huggingface.co/xberg-io">
    <img src="https://img.shields.io/badge/Hugging%20Face-Xberg-007ec6" alt="Hugging Face">
  </a>
</div>

<div align="center" style="display: flex; flex-wrap: wrap; gap: 12px; justify-content: center; margin: 28px 0 24px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Chat-007ec6?logo=discord&logoColor=white" alt="Join Discord">
  </a>
  <a href="https://docs.xberg.io/demo.html">
    <img height="22" src="https://img.shields.io/badge/Live%20Demo-Open-007ec6?logo=webassembly&logoColor=white" alt="Live Demo">
  </a>
  <a href="https://github.com/xberg-io/xberg/stargazers">
    <img height="22" src="https://img.shields.io/github/stars/xberg-io/xberg?style=social" alt="GitHub Stars">
  </a>
</div>

Extract text, tables, images, metadata, and code intelligence from 96 file formats and 306 programming languages including PDF, Office documents, images, and audio/video transcripts where native transcription is available. Native Python bindings with async/await support, multiple OCR backends (Tesseract, EasyOCR, PaddleOCR), and extensible plugin system.

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

```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig

async def main() -> None:
    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True
    )
    result = await extract("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```

### Simple Extraction

<!-- snippet not found: getting-started/extract.md -->

### Reading Content

```python title="Python"
import asyncio
from xberg import extract

async def main() -> None:
    result = await extract("document.pdf")

    content: str = result.content
    tables: int = len(result.tables)
    format_type: str | None = result.metadata.format.format_type if result.metadata.format else None

    print(f"Content length: {len(content)} characters")
    print(f"Tables found: {tables}")
    print(f"Format: {format_type}")

asyncio.run(main())
```

## OCR Support

### Using OCR

```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, OcrConfig, TesseractConfig

async def main() -> None:
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract("scanned.pdf", config=config)
    print(result.content)
    print(f"Detected Languages: {result.detected_languages}")

asyncio.run(main())
```

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

MIT License - see [LICENSE](../../LICENSE) for details.
