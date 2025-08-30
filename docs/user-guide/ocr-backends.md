# OCR Backends

Kreuzberg supports multiple OCR (Optical Character Recognition) backends, giving you flexibility to choose the best option for your specific needs. Each backend has different strengths, language support, and installation requirements.

## Supported Backends

### 1. Tesseract OCR

[Tesseract OCR](https://github.com/tesseract-ocr/tesseract) is the default OCR backend in Kreuzberg. It's a mature, open-source OCR engine with support for over 100 languages.

**Installation Requirements:**

- Requires system-level installation
- Minimum required version: Tesseract 5.0

**Installation Instructions:**

```bash
# Ubuntu/Debian
sudo apt-get install tesseract-ocr

# macOS
brew install tesseract

# Windows
choco install -y tesseract
```

**Language Support:**

- Kreuzberg Docker images include 12 common business languages: English, Spanish, French, German, Italian, Portuguese, Chinese (Simplified & Traditional), Japanese, Arabic, Russian, Hindi
- For additional languages, install language packs:
    - Ubuntu: `sudo apt-get install tesseract-ocr-deu` (for German)
    - macOS: `brew install tesseract-lang`

**Configuration:**

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

result = await extract_file(
    "document.pdf",
    config=ExtractionConfig(
        ocr_backend="tesseract",  # This is the default
        ocr_config=TesseractConfig(language="eng+deu", psm=PSMMode.AUTO),  # English and German  # Page segmentation mode
    ),
)
```

### 2. No OCR

You can also disable OCR completely, which is useful for documents that already contain searchable text.

**Configuration:**

```python
from kreuzberg import extract_file, ExtractionConfig

result = await extract_file("searchable_pdf.pdf", config=ExtractionConfig(ocr_backend=None))
```

## Choosing the Right Backend

Here are some guidelines for choosing the appropriate OCR backend:

### Tesseract OCR (Default)

**Advantages:**

- Lightweight and CPU-optimized
- No model downloads required (faster startup)
- Mature and widely used
- Lower memory usage
- Good for general-purpose OCR across many languages
- Good balance of accuracy and performance
- Supports 12 common business languages out of the box

**Considerations:**

- Requires system-level installation
- May have lower accuracy for some complex layouts compared to deep learning approaches
- More configuration may be needed for optimal results with specialized documents

### No OCR (Setting ocr_backend=None)

**Use when:**

- Processing searchable PDFs or documents with embedded text
- You want to extract embedded text only
- You want to avoid the overhead of OCR processing

**Behavior:**

- For searchable PDFs, embedded text will still be extracted
- For images and non-searchable PDFs, an empty string will be returned for content
- Fastest option as it skips OCR processing entirely

## Installation Summary

To install Kreuzberg:

```bash
# Basic installation (includes Tesseract OCR support)
pip install kreuzberg

# With chunking support
pip install "kreuzberg[chunking]"

# With language detection
pip install "kreuzberg[langdetect]"

# With all optional dependencies
pip install "kreuzberg[all]"
```

!!! note "System Dependencies"

    Remember that Pandoc and Tesseract are system dependencies that must be installed separately from the Python package.

    For Tesseract, you must install version 5.0 or higher, and you'll need to install additional language data files for languages other than English.
