# langchain-kreuzberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://pypi.org/project/langchain-kreuzberg/">
    <img src="https://img.shields.io/pypi/v/langchain-kreuzberg?label=PyPI&color=007ec6" alt="PyPI">
  </a>
  <a href="https://pypi.org/project/langchain-kreuzberg/">
    <img src="https://img.shields.io/pypi/pyversions/langchain-kreuzberg?color=007ec6" alt="Python versions">
  </a>
  <a href="https://pypi.org/project/langchain-kreuzberg/">
    <img src="https://img.shields.io/pypi/dm/langchain-kreuzberg" alt="Downloads">
  </a>
  <a href="https://github.com/xberg-io/langchain-kreuzberg/actions/workflows/ci.yaml">
    <img src="https://github.com/xberg-io/langchain-kreuzberg/actions/workflows/ci.yaml/badge.svg" alt="CI">
  </a>
  <a href="https://github.com/xberg-io/langchain-kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://github.com/xberg-io/kreuzberg">
    <img src="https://img.shields.io/github/stars/xberg-io/kreuzberg?style=flat&label=Kreuzberg&color=007ec6" alt="Kreuzberg">
  </a>
  <a href="https://docs.xberg.io">
    <img src="https://img.shields.io/badge/docs-xberg.io-blue" alt="Documentation">
  </a>
</div>

<img width="3384" height="573" alt="Kreuzberg Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

## Overview

**langchain-kreuzberg** is a LangChain document loader that wraps [Kreuzberg](https://github.com/xberg-io/kreuzberg)'s extraction API. It supports 90+ file formats out of the box, provides true async extraction powered by Rust's tokio runtime, and produces LangChain `Document` objects enriched with rich metadata including detected languages, quality scores, and extracted keywords.

## Installation

```bash
pip install langchain-kreuzberg
```

Requires Python 3.10+.

## Quick Start

```python
from langchain_kreuzberg import KreuzbergLoader

loader = KreuzbergLoader(file_path="report.pdf")
docs = loader.load()

print(docs[0].page_content[:200])
print(docs[0].metadata["source"])
```

## Features

- **90+ file formats** -- PDF, DOCX, PPTX, XLSX, images, HTML, Markdown, plain text, and many more
- **True async** -- native async extraction backed by Rust's tokio runtime; no thread-pool workarounds
- **Rich metadata** -- title, author, page count, detected languages, quality score, extracted keywords, and more
- **OCR with 3 backends** -- Tesseract, EasyOCR, and PaddleOCR with configurable language support
- **Per-page splitting** -- yield one `Document` per page for fine-grained RAG pipelines
- **Bytes input** -- load documents directly from raw bytes (e.g., API responses, S3 objects)
- **Output format selection** -- choose between plain text, Markdown, Djot, HTML, or structured output

## Usage Examples

### Load a PDF with defaults

```python
from langchain_kreuzberg import KreuzbergLoader

loader = KreuzbergLoader(file_path="contract.pdf")
docs = loader.load()
```

### Load multiple files

```python
loader = KreuzbergLoader(
    file_path=["report.pdf", "notes.docx", "data.xlsx"],
)
docs = loader.load()
```

### OCR a scanned document with Tesseract

```python
from kreuzberg import ExtractionConfig, OcrConfig

config = ExtractionConfig(
    force_ocr=True,
    ocr=OcrConfig(backend="tesseract", language="eng"),
)

loader = KreuzbergLoader(
    file_path="scanned.pdf",
    config=config,
)
docs = loader.load()
```

### Load all files from a directory

```python
loader = KreuzbergLoader(
    file_path="./documents/",
    glob="**/*.pdf",
)
docs = loader.load()
```

### Per-page splitting for RAG

```python
from kreuzberg import ExtractionConfig, PageConfig

config = ExtractionConfig(pages=PageConfig(extract_pages=True))

loader = KreuzbergLoader(
    file_path="handbook.pdf",
    config=config,
)
docs = loader.load()
# docs[0].metadata["page"] == 0  (zero-indexed)
```

### Load from bytes (API response)

```python
import httpx

response = httpx.get("https://example.com/report.pdf")

loader = KreuzbergLoader(
    data=response.content,
    mime_type="application/pdf",
)
docs = loader.load()
```

### Advanced config

```python
from kreuzberg import ExtractionConfig, OcrConfig, PageConfig

config = ExtractionConfig(
    output_format="markdown",
    ocr=OcrConfig(backend="easyocr", language="deu"),
    force_ocr=True,
    pages=PageConfig(extract_pages=True),
)

loader = KreuzbergLoader(
    file_path="report.pdf",
    config=config,
)
docs = loader.load()
```

### Async loading

```python
import asyncio
from langchain_kreuzberg import KreuzbergLoader

async def main():
    loader = KreuzbergLoader(file_path="report.pdf")
    docs = await loader.aload()
    print(f"Loaded {len(docs)} documents")

asyncio.run(main())
```

## API Reference

### `KreuzbergLoader`

```python
from langchain_kreuzberg import KreuzbergLoader
```

Extends `langchain_core.document_loaders.BaseLoader`.

#### Constructor Parameters

All parameters are keyword-only.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `file_path` | `str \| Path \| list[str \| Path] \| None` | `None` | File path, list of file paths, or directory path to load. |
| `data` | `bytes \| None` | `None` | Raw bytes to extract text from. Mutually exclusive with `file_path`. |
| `mime_type` | `str \| None` | `None` | MIME type hint. Required when using `data`, optional for `file_path`. |
| `glob` | `str \| None` | `None` | Glob pattern for directory loading. |
| `config` | `ExtractionConfig \| None` | `None` | Kreuzberg `ExtractionConfig` for controlling extraction behavior (output format, OCR settings, page splitting, etc.). See the [Kreuzberg repository](https://github.com/xberg-io/kreuzberg) for all options. |

#### Methods

| Method | Return Type | Description |
|---|---|---|
| `load()` | `list[Document]` | Load all documents into memory. |
| `lazy_load()` | `Iterator[Document]` | Lazily yield documents one at a time (synchronous). |
| `aload()` | `list[Document]` | Load all documents asynchronously. |
| `alazy_load()` | `AsyncIterator[Document]` | Lazily yield documents one at a time (asynchronous). |

## Metadata Fields

Each `Document` produced by `KreuzbergLoader` includes the following metadata fields (when available):

| Field | Type | Description |
|---|---|---|
| `source` | `str` | File path or `bytes://<mime_type>` for bytes input. |
| `mime_type` | `str` | Detected or provided MIME type. |
| `page_count` | `int` | Total number of pages in the document. |
| `output_format` | `str` | The output format used for extraction. |
| `quality_score` | `float` | Extraction quality score (0.0 -- 1.0). |
| `detected_languages` | `list[str]` | Languages detected in the document. |
| `extracted_keywords` | `list[dict]` | Keywords with `text`, `score`, and `algorithm` fields. |
| `table_count` | `int` | Number of tables found in the document. |
| `tables` | `list[dict]` | Table data with `cells`, `markdown`, and `page_number` fields. |
| `processing_warnings` | `list[dict]` | Warnings with `source` and `message` fields. |
| `page` | `int` | Zero-indexed page number (only present in per-page mode). |
| `is_blank` | `bool` | Whether the page is blank (only present in per-page mode). |
| `title` | `str` | Document title (from file metadata). |
| `author` | `str` | Document author (from file metadata). |
| `subject` | `str` | Document subject (from file metadata). |
| `creator` | `str` | Application that created the document. |
| `producer` | `str` | Application that produced the document. |
| `creation_date` | `str` | Document creation date. |
| `modification_date` | `str` | Document last modification date. |

Additional metadata fields from Kreuzberg's document-level metadata are flattened into the metadata dict when present.

## Supported Formats

Kreuzberg supports 90+ file formats including PDF, DOCX, images (via OCR), spreadsheets, presentations, HTML, Markdown, and many more. For the full and up-to-date list of supported formats, see the [Kreuzberg repository](https://github.com/xberg-io/kreuzberg).

## Contributing

This project uses [uv](https://docs.astral.sh/uv/) for dependency management.

```bash
# Clone the repository
git clone https://github.com/xberg-io/langchain-kreuzberg.git
cd langchain-kreuzberg

# Install dependencies (including dev group)
uv sync

# Run linting
uv run ruff check .
uv run ruff format --check .
uv run mypy .

# Run unit tests
uv run pytest --cov

# Run integration tests (real file extraction, no mocks)
uv run pytest -m integration -v

# Install pre-commit hooks
prek install
```

## License

This project is licensed under the [MIT License](LICENSE).
