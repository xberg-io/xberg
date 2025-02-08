# Kreuzberg

Kreuzberg is a modern Python library for text extraction from documents, designed for simplicity and efficiency. It provides a unified async interface for extracting text from a wide range of file formats including PDFs, images, office documents, and more.

## Why Kreuzberg?

- **Simple and Hassle-Free**: Clean API that just works, without complex configuration
- **Local Processing**: No external API calls or cloud dependencies required
- **Resource Efficient**: Lightweight processing without GPU requirements
- **Format Support**: Comprehensive support for documents, images, and text formats
- **Modern Python**: Built with async/await, type hints, and current best practices

Kreuzberg was created to solve text extraction needs in RAG (Retrieval Augmented Generation) applications, but it's suitable for any text extraction use case. Unlike many commercial solutions that require API calls or complex setups, Kreuzberg focuses on local processing with minimal dependencies.

## Features

- **Universal Text Extraction**: Extract text from PDFs (both searchable and scanned), images, office documents, and more
- **Smart Processing**: Automatic OCR for scanned documents, encoding detection for text files
- **Modern Python Design**:
  - Async-first API using `anyio`
  - Comprehensive type hints for better IDE support
  - Detailed error handling with context information
- **Production Ready**:
  - Robust error handling
  - Detailed debugging information
  - Memory efficient processing

## Installation

### 1. Install the Python Package

```shell
pip install kreuzberg
```

### 2. Install System Dependencies

Kreuzberg requires two open-source tools:

- [Pandoc](https://pandoc.org/installing.html) - For document format conversion

  - GPL v2.0 licensed (used via CLI only)
  - Handles office documents and markup formats

- [Tesseract OCR](https://tesseract-ocr.github.io/) - For image and PDF OCR
  - Apache License
  - Required for scanned documents and images

## Architecture

Kreuzberg is designed as a high-level async abstraction over established open-source tools. It integrates:

- **PDF Processing**:
  - `pdfium2` for searchable PDFs
  - Tesseract OCR for scanned content
- **Document Conversion**:
  - Pandoc for office documents and markup
  - `python-pptx` for PowerPoint files
  - `html-to-markdown` for HTML content
- **Text Processing**:
  - Smart encoding detection
  - Markdown and plain text handling

### Supported Formats

#### Document Formats

- PDF (`.pdf`, both searchable and scanned documents)
- Microsoft Word (`.docx`, `.doc`)
- PowerPoint presentations (`.pptx`)
- OpenDocument Text (`.odt`)
- Rich Text Format (`.rtf`)
- EPUB (`.epub`)
- DocBook XML (`.dbk`, `.xml`)
- FictionBook (`.fb2`)
- LaTeX (`.tex`, `.latex`)
- Typst (`.typ`)

#### Markup and Text Formats

- HTML (`.html`, `.htm`)
- Plain text (`.txt`) and Markdown (`.md`, `.markdown`)
- reStructuredText (`.rst`)
- Org-mode (`.org`)
- DokuWiki (`.txt`)
- Pod (`.pod`)
- Man pages (`.1`, `.2`, etc.)

#### Data and Research Formats

- CSV (`.csv`) and TSV (`.tsv`) files
- Jupyter Notebooks (`.ipynb`)
- BibTeX (`.bib`) and BibLaTeX (`.bib`)
- CSL-JSON (`.json`)
- EndNote XML (`.xml`)
- RIS (`.ris`)
- JATS XML (`.xml`)

#### Image Formats

- JPEG (`.jpg`, `.jpeg`, `.pjpeg`)
- PNG (`.png`)
- TIFF (`.tiff`, `.tif`)
- BMP (`.bmp`)
- GIF (`.gif`)
- WebP (`.webp`)
- JPEG 2000 (`.jp2`, `.jpx`, `.jpm`, `.mj2`)
- Portable Anymap (`.pnm`)
- Portable Bitmap (`.pbm`)
- Portable Graymap (`.pgm`)
- Portable Pixmap (`.ppm`)

## Usage

Kreuzberg provides a simple, async-first API for text extraction. The library exports two main functions:

- `extract_file()`: Extract text from a file (accepts string path or `pathlib.Path`)
- `extract_bytes()`: Extract text from bytes (accepts a byte string)

### Quick Start

```python
from pathlib import Path
from kreuzberg import extract_file, extract_bytes

# Basic file extraction
async def extract_document():
    # Extract from a PDF file
    pdf_result = await extract_file("document.pdf")
    print(f"PDF text: {pdf_result.content}")

    # Extract from an image
    img_result = await extract_file("scan.png")
    print(f"Image text: {img_result.content}")

    # Extract from Word document
    docx_result = await extract_file(Path("document.docx"))
    print(f"Word text: {docx_result.content}")
```

### Processing Uploaded Files

```python
from kreuzberg import extract_bytes

async def process_upload(file_content: bytes, mime_type: str):
    """Process uploaded file content with known MIME type."""
    result = await extract_bytes(file_content, mime_type=mime_type)
    return result.content

# Example usage with different file types
async def handle_uploads():
    # Process PDF upload
    pdf_result = await extract_bytes(pdf_bytes, mime_type="application/pdf")

    # Process image upload
    img_result = await extract_bytes(image_bytes, mime_type="image/jpeg")

    # Process Word document upload
    docx_result = await extract_bytes(docx_bytes,
        mime_type="application/vnd.openxmlformats-officedocument.wordprocessingml.document")
```

### Advanced Features

#### PDF Processing Options

```python
from kreuzberg import extract_file

async def process_pdf():
    # Force OCR for PDFs with embedded images or scanned content
    result = await extract_file("document.pdf", force_ocr=True)

    # Process a scanned PDF (automatically uses OCR)
    scanned = await extract_file("scanned.pdf")
```

#### ExtractionResult Object

All extraction functions return an `ExtractionResult` containing:

- `content`: The extracted text (str)
- `mime_type`: Output format ("text/plain" or "text/markdown" for Pandoc conversions)

```python
from kreuzberg import ExtractionResult

async def process_document(path: str) -> tuple[str, str]:
    # Access as a named tuple
    result: ExtractionResult = await extract_file(path)
    print(f"Content: {result.content}")
    print(f"Format: {result.mime_type}")

    # Or unpack as a tuple
    content, mime_type = await extract_file(path)
    return content, mime_type
```

### Error Handling

Kreuzberg provides detailed error handling with two main exception types:

```python
from kreuzberg import extract_file
from kreuzberg.exceptions import ValidationError, ParsingError

async def safe_extract(path: str) -> str:
    try:
        result = await extract_file(path)
        return result.content

    except ValidationError as e:
        # Handles input validation issues:
        # - Unsupported file types
        # - Missing files
        # - Invalid MIME types
        print(f"Invalid input: {e.message}")
        print(f"Details: {e.context}")

    except ParsingError as e:
        # Handles processing errors:
        # - PDF parsing failures
        # - OCR errors
        # - Format conversion issues
        print(f"Processing failed: {e.message}")
        print(f"Details: {e.context}")

    return ""

# Example error contexts
try:
    result = await extract_file("document.xyz")
except ValidationError as e:
    # e.context might contain:
    # {
    #    "file_path": "document.xyz",
    #    "error": "Unsupported file type",
    #    "supported_types": ["pdf", "docx", ...]
    # }

try:
    result = await extract_file("scan.pdf")
except ParsingError as e:
    # e.context might contain:
    # {
    #    "file_path": "scan.pdf",
    #    "error": "OCR processing failed",
    #    "details": "Tesseract error: Unable to process image"
    # }
```

## Roadmap

V1:

- [x] - html file text extraction
- [ ] - better PDF table extraction
- [ ] - batch APIs
- [ ] - sync APIs

V2:

- [ ] - metadata extraction (breaking change)
- [ ] - TBD

## Contribution

This library is open to contribution. Feel free to open issues or submit PRs. Its better to discuss issues before
submitting PRs to avoid disappointment.

### Local Development

1. Clone the repo
2. Install the system dependencies
3. Install the full dependencies with `uv sync`
4. Install the pre-commit hooks with:
   ```shell
   pre-commit install && pre-commit install --hook-type commit-msg
   ```
5. Make your changes and submit a PR

## License

This library uses the MIT license.
