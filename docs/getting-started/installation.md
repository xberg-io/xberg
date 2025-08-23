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

    Tesseract includes English language support by default. If you need to process documents in other languages, you must install the appropriate language data files:

    - **Ubuntu/Debian**: `sudo apt-get install tesseract-ocr-deu` (for German)
    - **macOS**: `brew install tesseract-lang`
    - **Windows**: See the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html#windows)

    For more details on language installation and configuration, refer to the [Tesseract documentation](https://tesseract-ocr.github.io/tessdoc/Installation.html).

#### EasyOCR

EasyOCR is a Python-based OCR backend with wide language support and strong performance.

```shell
pip install "kreuzberg[easyocr]"
```

#### PaddleOCR

PaddleOCR requires additional system dependencies for OpenCV support:

##### System Dependencies

###### Ubuntu/Debian

```shell
# Install OpenGL libraries required by OpenCV
sudo apt-get install libgl1 libglib2.0-0
```

###### macOS

```shell
# OpenGL support is typically included with macOS
# If you encounter issues, install:
brew install glfw
```

###### Windows

OpenGL libraries are typically included with graphics drivers on Windows.

##### Python Package

```shell
pip install "kreuzberg[paddleocr]"
```

### Chunking

Chunking is an optional feature - useful for RAG applications among others. Kreuzberg uses the excellent `semantic-text-splitter` package for chunking. To install Kreuzberg with chunking support, you can use:

```shell
pip install "kreuzberg[chunking]"
```

### Table Extraction

Table extraction is an optional feature that allows Kreuzberg to extract tables from PDFs. It uses the [GMFT](https://github.com/conjuncts/gmft) package. To install Kreuzberg with table extraction support, you can use:

```shell
pip install "kreuzberg[gmft]"
```

### Language Detection

Language detection is an optional feature that automatically detects the language of extracted text. It uses the [fast-langdetect](https://github.com/LlmKira/fast-langdetect) package. To install Kreuzberg with language detection support, you can use:

```shell
pip install "kreuzberg[langdetect]"
```

### Entity and Keyword Extraction

Entity and keyword extraction are optional features that extract named entities and keywords from documents. Entity extraction uses [spaCy](https://spacy.io/) for multilingual named entity recognition, while keyword extraction uses [KeyBERT](https://github.com/MaartenGr/KeyBERT) for semantic keyword extraction:

```shell
pip install "kreuzberg[entity-extraction]"
```

After installation, you'll need to download the spaCy language models you plan to use:

```shell
# Download English model (most common)
python -m spacy download en_core_web_sm

# Download other language models as needed
python -m spacy download de_core_news_sm  # German
python -m spacy download fr_core_news_sm  # French
python -m spacy download es_core_news_sm  # Spanish
```

!!! note "Language Model Requirements"

    spaCy language models are large (50-500MB each) and are downloaded separately. Only download the models for languages you actually need to process. See the [spaCy models documentation](https://spacy.io/models) for a complete list of available models.

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
pip install "kreuzberg[chunking,document-classification,easyocr,entity-extraction,gmft,langdetect,paddleocr]"
```
