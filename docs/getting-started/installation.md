# Installation

Kreuzberg is a modular document intelligence framework with a core package and optional components for specialized functionality.

## System Dependencies

### Pandoc

Pandoc is the foundation of Kreuzberg's universal document conversion capabilities. This **required** system dependency enables reliable extraction across diverse document formats. Install Pandoc for your platform:

#### Ubuntu/Debian

```shell
sudo apt-get install pandoc
```

#### macOS

```shell
brew install pandoc
```

#### Windows

```shell
choco install -y pandoc
```

## Kreuzberg Core Package

The Kreuzberg core package can be installed using pip with:

```shell
pip install kreuzberg
```

## Optional Features

### OCR

OCR is an optional feature for extracting text from images and non-searchable PDFs. Kreuzberg supports multiple OCR backends. To understand the differences between these backends, please read the [OCR Backends documentation](../user-guide/ocr-backends.md).

#### Tesseract OCR

Tesseract OCR is built into Kreuzberg and doesn't require additional Python packages. However, you must install Tesseract 5.0 or higher on your system:

##### Ubuntu/Debian

```shell
sudo apt-get install tesseract-ocr
```

##### macOS

```shell
brew install tesseract
```

##### Windows

```shell
choco install -y tesseract
```

!!! note "Language Support"

    Tesseract includes English language support by default. Kreuzberg Docker images come pre-configured with 12 common business languages: English, Spanish, French, German, Italian, Portuguese, Chinese (Simplified & Traditional), Japanese, Arabic, Russian, and Hindi.

    For local installations requiring additional languages, you must install the appropriate language data files:

    - **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr-deu` (for German)
    - **macOS**: `brew install tesseract-lang`
    - **Windows**: See the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html#windows)

    For more details on language installation and configuration, refer to the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html).

### Chunking

Chunking is an optional feature - useful for RAG applications among others. Kreuzberg uses the excellent `semantic-text-splitter` package for chunking. To install Kreuzberg with chunking support, you can use:

```shell
pip install "kreuzberg[chunking]"
```

### Language Detection

Language detection is an optional feature that automatically detects the language of extracted text. It uses the [fast-langdetect](https://github.com/LlmKira/fast-langdetect) package. To install Kreuzberg with language detection support, you can use:

```shell
pip install "kreuzberg[langdetect]"
```

### Document Classification

For automatic document type detection (invoice, contract, receipt, etc.), install the document classification extra:

```shell
pip install "kreuzberg[document-classification]"
```

This feature uses Google Translate for multi-language support and requires explicit opt-in by setting `auto_detect_document_type=True` in your configuration.

### All Optional Dependencies

To install Kreuzberg with all optional dependencies, you can use the `all` extra group:

```shell
pip install "kreuzberg[all]"
```

This is equivalent to:

```shell
pip install "kreuzberg[chunking,document-classification,langdetect,crypto,additional-extensions,cli,api]"
```
