# Kreuzberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://github.com/kreuzberg-dev/alef">
    <img src="https://img.shields.io/badge/bindings%20by-alef%20%D7%90-007ec6" alt="Bindings by alef">
  </a>
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
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
  <a href="https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg/v5">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v5*" alt="Go">
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
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://pub.dev/packages/kreuzberg">
    <img src="https://img.shields.io/pub/v/kreuzberg?label=Dart&color=007ec6" alt="Dart">
  </a>
  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg-android">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg-android?label=Kotlin&color=007ec6" alt="Kotlin">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/swift">
    <img src="https://img.shields.io/badge/Swift-SPM-007ec6" alt="Swift">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/tree/main/packages/zig">
    <img src="https://img.shields.io/badge/Zig-package-007ec6" alt="Zig">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C FFI">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-ghcr.io-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/charts%2Fkreuzberg">
    <img src="https://img.shields.io/badge/Helm-ghcr.io-007ec6?logo=helm&logoColor=white" alt="Helm">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-Elastic--2.0-007ec6" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-007ec6" alt="Documentation">
  </a>
  <a href="https://docs.kreuzberg.dev/demo.html">
    <img src="https://img.shields.io/badge/Live%20Demo-open-007ec6" alt="Live Demo">
  </a>
  <a href="https://huggingface.co/Kreuzberg">
    <img src="https://img.shields.io/badge/Hugging%20Face-Kreuzberg-007ec6" alt="Hugging Face">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-007ec6?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Extract text, tables, images, and metadata from 91+ file formats and 248 programming languages including PDF, Office documents, and images. Native Python bindings with async/await support, multiple OCR backends (Tesseract, EasyOCR, PaddleOCR), and extensible plugin system.

**Powered by a Rust core** – Native performance for document extraction.

## Installation

```bash
pip install kreuzberg
```

### With OCR Support

```bash
pip install "kreuzberg[easyocr]"
pip install "kreuzberg[paddleocr]"
```

### All Features

```bash
pip install "kreuzberg[all]"
```

## Quick Start

### Basic Usage

```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main() -> None:
    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True
    )
    result = await extract_file("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```

### Simple Extraction

```python title="Python"
import asyncio
from pathlib import Path
from kreuzberg import extract_file

async def main() -> None:
    file_path: Path = Path("document.pdf")

    result = await extract_file(file_path)

    print(f"Content: {result.content}")
    print(f"Format: {result.metadata.format.format_type if result.metadata.format else None}")
    print(f"Tables: {len(result.tables)}")

asyncio.run(main())
```

### Reading Content

```python title="Python"
import asyncio
from kreuzberg import extract_file

async def main() -> None:
    result = await extract_file("document.pdf")

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
from kreuzberg import extract_file, ExtractionConfig, OcrConfig, TesseractConfig

async def main() -> None:
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract_file("scanned.pdf", config=config)
    print(result.content)
    print(f"Detected Languages: {result.detected_languages}")

asyncio.run(main())
```

### EasyOCR (GPU-Accelerated)

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

result = extract_file_sync(
    "photo.jpg",
    config=config,
    easyocr_kwargs={"use_gpu": True}
)
```

### PaddleOCR (Complex Layouts)

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="ch")
)

result = extract_file_sync(
    "invoice.pdf",
    config=config,
)
```

## Table Extraction

```python
from kreuzberg import extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        tesseract_config=TesseractConfig(
            enable_table_detection=True
        )
    )
)

result = extract_file_sync("invoice.pdf", config=config)

for table in result.tables:
    print(table.markdown)
    print(table.cells)
```

## Configuration

### Complete Configuration Example

```python
from kreuzberg import (
    extract_file_sync,
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

result = extract_file_sync("document.pdf", config=config)
```

### HTML Conversion Options & Batch Concurrency

```python
from kreuzberg import ExtractionConfig

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
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")

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
from kreuzberg import extract_file_sync, ExtractionConfig, PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2", "password3"]
    )
)

result = extract_file_sync("protected.pdf", config=config)
```

## Language Detection

```python
from kreuzberg import extract_file_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract_file_sync("multilingual.pdf", config=config)
print(result.detected_languages)
```

## Text Chunking

```python
from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=200,
    )
)

result = extract_file_sync("long_document.pdf", config=config)

for chunk in result.chunks:
    print(chunk)
```

## Extract from Bytes

```python
from kreuzberg import extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")
print(result.content)
```

## API Reference

### Extraction Functions

- `extract_file(file_path, mime_type=None, config=None, **kwargs)` – Async extraction
- `extract_file_sync(file_path, mime_type=None, config=None, **kwargs)` – Sync extraction
- `extract_bytes(data, mime_type, config=None, **kwargs)` – Async extraction from bytes
- `extract_bytes_sync(data, mime_type, config=None, **kwargs)` – Sync extraction from bytes
- `batch_extract_files(paths, config=None, **kwargs)` – Async batch extraction
- `batch_extract_files_sync(paths, config=None, **kwargs)` – Sync batch extraction
- `batch_extract_bytes(data_list, mime_types, config=None, **kwargs)` – Async batch from bytes
- `batch_extract_bytes_sync(data_list, mime_types, config=None, **kwargs)` – Sync batch from bytes

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

- `KreuzbergError` – Base exception
- `ValidationError` – Invalid configuration or input
- `ParsingError` – Document parsing failure
- `OCRError` – OCR processing failure
- `MissingDependencyError` – Missing optional dependency

## Examples

### Custom Processing

```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")

text = result.content
text = text.lower()
text = text.replace("old", "new")

print(text)
```

### Multiple Files with Progress

```python
from kreuzberg import extract_file_sync
from pathlib import Path

files = list(Path("documents").glob("*.pdf"))
results = []

for i, file in enumerate(files, 1):
    print(f"Processing {i}/{len(files)}: {file.name}")
    result = extract_file_sync(str(file))
    results.append((file.name, result))

for name, result in results:
    print(f"{name}: {len(result.content)} characters")
```

### Filter by Language

```python
from kreuzberg import extract_file_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(enabled=True)
)

result = extract_file_sync("document.pdf", config=config)

if result.detected_languages and "en" in result.detected_languages:
    print("English document detected")
    print(result.content)
```

## System Requirements

### ONNX Runtime (for embeddings)

If using embeddings functionality, ONNX Runtime version 1.22.x must be installed:

```bash
# macOS
brew install onnxruntime

# Ubuntu/Debian (download from GitHub - Debian packages may have older versions)
# Download from https://github.com/microsoft/onnxruntime/releases

# Windows
# Download from https://github.com/microsoft/onnxruntime/releases
```

**Important:** Kreuzberg requires ONNX Runtime version 1.22.x for embeddings.

Without ONNX Runtime, embeddings will raise `MissingDependencyError` with installation instructions.

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

### Import Error: No module named '\_kreuzberg'

This usually means the Rust extension wasn't built correctly. Try:

```bash
pip install --force-reinstall --no-cache-dir kreuzberg
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

For comprehensive documentation, visit [https://kreuzberg.dev](https://kreuzberg.dev)

## Part of Kreuzberg.dev

- [Kreuzberg Cloud](https://github.com/kreuzberg-dev/kreuzberg-cloud) — managed extraction API with SDKs, dashboards, and observability.
- [kreuzcrawl](https://github.com/kreuzberg-dev/kreuzcrawl) — web crawling and scraping with HTML→Markdown and headless-Chrome fallback.
- [html-to-markdown](https://github.com/kreuzberg-dev/html-to-markdown) — fast, lossless HTML→Markdown engine.
- [liter-llm](https://github.com/kreuzberg-dev/liter-llm) — universal LLM API client with native bindings for 14 languages and 143 providers.
- [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack) — tree-sitter grammars and code-intelligence primitives.
- [alef](https://github.com/kreuzberg-dev/alef) — the polyglot binding generator that produces this README and all per-language bindings.
- [Discord](https://discord.gg/xt9WY3GnKR) — community, roadmap, announcements.

## License

Elastic-2.0 License - see [LICENSE](../../LICENSE) for details.
