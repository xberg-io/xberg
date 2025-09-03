# Extraction Examples

This page provides practical examples of using Kreuzberg for text extraction in various scenarios.

## Basic Extraction

```python
import asyncio
from kreuzberg import extract_file

async def main():
    # Extract text from a PDF file
    result = await extract_file("document.pdf")
    print(result.content)

    # Access metadata
    if result.metadata.get("title"):
        print(f"Document title: {result.metadata['title']}")

asyncio.run(main())
```

## OCR Configuration

Kreuzberg provides options to configure OCR for different languages and document layouts:

```python
from kreuzberg import extract_file, TesseractConfig, PSMMode, ExtractionConfig

async def extract_with_ocr():
    # Extract from a German document
    result = await extract_file(
        "german_document.pdf",
        config=ExtractionConfig(
            force_ocr=True,
            ocr_config=TesseractConfig(
                language="deu", psm=PSMMode.SINGLE_BLOCK  # German language  # Treat as a single text block
            ),
        ),
    )
    print(result.content)

    # Extract from a multilingual document
    result = await extract_file(
        "multilingual.pdf",
        config=ExtractionConfig(
            force_ocr=True,
            ocr_config=TesseractConfig(
                language="eng+deu", psm=PSMMode.AUTO  # English primary, German secondary  # Automatic page segmentation
            ),
        ),
    )
    print(result.content)
```

## Alternative OCR Backends

Kreuzberg supports multiple OCR backends:

```python
from kreuzberg import extract_file, ExtractionConfig, EasyOCRConfig, PaddleOCRConfig

async def extract_with_different_backends():
    # Using EasyOCR
    result = await extract_file(
        "document.jpg", config=ExtractionConfig(ocr_backend="easyocr", ocr_config=EasyOCRConfig(language_list=["en", "de"]))
    )
    print(f"EasyOCR result: {result.content[:100]}...")

    # Using PaddleOCR
    result = await extract_file(
        "chinese_document.jpg",
        config=ExtractionConfig(ocr_backend="paddleocr", ocr_config=PaddleOCRConfig(language="ch")),  # Chinese
    )
    print(f"PaddleOCR result: {result.content[:100]}...")

    # Disable OCR completely
    result = await extract_file("searchable_pdf.pdf", config=ExtractionConfig(ocr_backend=None))
    print(f"No OCR result: {result.content[:100]}...")
```

## Language Detection

```python
from kreuzberg import extract_file, ExtractionConfig, LanguageDetectionConfig

async def detect_document_language():
    # Simple automatic language detection
    result = await extract_file("document.pdf", config=ExtractionConfig(auto_detect_language=True))

    # Access detected languages
    if result.detected_languages:
        print(f"Detected languages: {', '.join(result.detected_languages)}")
        # Example output: "Detected languages: en, de, fr"

async def detect_multilingual_document():
    # Advanced multilingual detection with custom configuration
    lang_config = LanguageDetectionConfig(
        multilingual=True,  # Detect multiple languages in mixed text
        top_k=5,  # Return top 5 languages
        low_memory=False,  # Use high accuracy mode
    )

    result = await extract_file(
        "multilingual_document.pdf", config=ExtractionConfig(auto_detect_language=True, language_detection_config=lang_config)
    )

    if result.detected_languages:
        print(f"Detected languages: {result.detected_languages}")

        # Use detected languages for OCR
        from kreuzberg import TesseractConfig

        # Create language string for Tesseract (e.g., "eng+deu+fra")
        tesseract_langs = "+".join(result.detected_languages[:3])

        result_with_ocr = await extract_file(
            "multilingual_document.pdf",
            config=ExtractionConfig(force_ocr=True, ocr_config=TesseractConfig(language=tesseract_langs)),
        )
```

## Table Extraction

```python
from kreuzberg import extract_file, ExtractionConfig, GMFTConfig

async def extract_tables_from_pdf():
    # Enable table extraction with default settings
    result = await extract_file("document_with_tables.pdf", config=ExtractionConfig(extract_tables=True))

    # Process extracted tables
    print(f"Found {len(result.tables)} tables")
    for i, table in enumerate(result.tables):
        print(f"Table {i+1} on page {table['page_number']}:")
        print(table["text"])  # Markdown formatted table

        # Work with the pandas DataFrame
        df = table["df"]
        print(f"Table shape: {df.shape}")

        # The cropped table image is also available
        # table['cropped_image'].save(f"table_{i+1}.png")

    # With custom GMFT configuration
    custom_config = ExtractionConfig(
        extract_tables=True,
        gmft_config=GMFTConfig(
            detector_base_threshold=0.85,  # Min confidence for table detection
            enable_multi_header=True,  # Support multi-level headers
            semantic_spanning_cells=True,  # Handle spanning cells
            semantic_hierarchical_left_fill="deep",  # Handle hierarchical headers
        ),
    )

    result = await extract_file("complex_tables.pdf", config=custom_config)
    # Process tables...
```

## OCR Output Formats and Table Extraction

### Choosing the Right Output Format

Kreuzberg's Tesseract backend supports multiple output formats, each optimized for different use cases.

#### Fast Plain Text Extraction

Use the `text` format for the fastest extraction when you don't need formatting:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_plain_text():
    result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="text")))
    print(result.content)
```

#### Default Markdown with Structure

The default `markdown` format preserves document structure:

```python
from kreuzberg import extract_file

async def extract_with_markdown():
    # Markdown is the default format
    result = await extract_file("document.jpg")
    print(result.content)  # Structured markdown output
```

#### Extract Tables from Scanned Documents

Use TSV format with table detection to extract tables from images:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_tables():
    result = await extract_file(
        "scanned_table.png",
        config=ExtractionConfig(
            ocr_config=TesseractConfig(
                output_format="tsv",
                enable_table_detection=True,
                table_column_threshold=20,  # Adjust for column spacing
                table_row_threshold_ratio=0.5,  # Adjust for row spacing
            )
        ),
    )

    # Access extracted tables
    for table in result.tables:
        print("Extracted table in markdown format:")
        print(table["text"])
        print(f"Page number: {table['page_number']}")
```

#### Get Word Positions with hOCR

Use hOCR format to access detailed position information:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig

async def extract_with_positions():
    result = await extract_file("document.jpg", config=ExtractionConfig(ocr_config=TesseractConfig(output_format="hocr")))
    # result.content contains HTML with position data
    print(result.content[:500])  # hOCR HTML output
```

### Processing Scanned Invoices

Complete example for extracting data from scanned invoices:

```python
from kreuzberg import extract_file, ExtractionConfig, TesseractConfig, PSMMode

async def process_invoice():
    # Configure for invoice processing
    config = ExtractionConfig(
        ocr_config=TesseractConfig(
            output_format="tsv",
            enable_table_detection=True,
            table_column_threshold=25,  # Adjust for invoice layout
            psm=PSMMode.SPARSE_TEXT,  # Good for forms and invoices
            language="eng",
        )
    )

    result = await extract_file("invoice_scan.pdf", config=config)

    # Get the main text content
    print("Invoice text:")
    print(result.content)

    # Extract tables (line items)
    if result.tables:
        print("\nInvoice line items:")
        for table in result.tables:
            print(table["text"])
```

## Batch Processing

```python
from kreuzberg import batch_extract_file, ExtractionConfig

async def process_documents():
    file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
    config = ExtractionConfig()  # Optional: configure extraction options
    results = await batch_extract_file(file_paths, config=config)

    for path, result in zip(file_paths, results):
        print(f"File: {path}")
        print(f"Content: {result.content[:100]}...")
```

## Working with Bytes

```python
from kreuzberg import extract_bytes, ExtractionConfig

async def process_upload(file_content: bytes, mime_type: str):
    # Extract text from uploaded file content
    config = ExtractionConfig()  # Optional: configure extraction options
    result = await extract_bytes(file_content, mime_type=mime_type, config=config)
    print(f"Content: {result.content[:100]}...")

    # Access metadata
    if result.metadata:
        for key, value in result.metadata.items():
            print(f"{key}: {value}")
```

## Keywords

Kreuzberg supports keywords and regex extraction as follows:

```python
from kreuzberg import ExtractionConfig, extract_file

async def extract_keywords():
    config = ExtractionConfig(
        extract_keywords=True,
        keyword_count=5,  # defaults to 10 if not set
    )
    result = await extract_file(
        "document.pdf",
        config=config,
    )
    print(f"Keywords: {result.keywords}")
```

## Entity and Keyword Extraction

Kreuzberg can extract named entities using spaCy and keywords using KeyBERT. It automatically detects entities like people, organizations, locations, and more, plus supports custom regex patterns:

```python
from kreuzberg import ExtractionConfig, extract_file, SpacyEntityExtractionConfig

async def extract_entities_and_keywords():
    # Basic extraction
    config = ExtractionConfig(
        extract_entities=True,
        extract_keywords=True,
        keyword_count=5,
        custom_entity_patterns={
            "INVOICE_ID": r"INV-\d+",
            "EMAIL": r"[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+",
        },
    )
    result = await extract_file("document.pdf", config=config)

    # Print extracted entities
    if result.entities:
        for entity in result.entities:
            print(f"{entity.type}: {entity.text}")

    # Print extracted keywords
    if result.keywords:
        for keyword, score in result.keywords:
            print(f"Keyword: {keyword} (score: {score:.3f})")

async def extract_multilingual_entities():
    # Configure spaCy for multiple languages
    spacy_config = SpacyEntityExtractionConfig(
        language_models={
            "en": "en_core_web_sm",
            "de": "de_core_news_sm",
            "fr": "fr_core_news_sm",
        },
        fallback_to_multilingual=True,
    )

    config = ExtractionConfig(
        auto_detect_language=True,  # Automatically detect document languages
        extract_entities=True,
        spacy_entity_extraction_config=spacy_config,
    )

    result = await extract_file("multilingual_document.pdf", config=config)

    if result.detected_languages:
        print(f"Detected languages: {result.detected_languages}")

    if result.entities:
        print(f"Extracted {len(result.entities)} entities")
        for entity in result.entities:
            print(f"  {entity.type}: {entity.text}")
```

## Synchronous API

For cases where async isn't needed or available:

```python
from kreuzberg import extract_file_sync, batch_extract_file_sync, ExtractionConfig

# Configuration for extraction
config = ExtractionConfig()  # Optional: configure extraction options

# Single file extraction
result = extract_file_sync("document.pdf", config=config)
print(result.content)

# Batch processing
file_paths = ["document1.pdf", "document2.docx", "image.jpg"]
results = batch_extract_file_sync(file_paths, config=config)
for path, result in zip(file_paths, results):
    print(f"File: {path}")
    print(f"Content: {result.content[:100]}...")
```

## Error Handling

```python
from kreuzberg import extract_file, ExtractionConfig
from kreuzberg import KreuzbergError, MissingDependencyError, OCRError

async def safe_extract(path):
    try:
        config = ExtractionConfig()  # Optional: configure extraction options
        result = await extract_file(path, config=config)
        return result.content
    except MissingDependencyError as e:
        print(f"Missing dependency: {e}")
        print("Please install the required dependencies.")
    except OCRError as e:
        print(f"OCR processing failed: {e}")
    except KreuzbergError as e:
        print(f"Extraction error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")

    return None
```
