# Kreuzberg PHP Snippets

Comprehensive code examples for the Kreuzberg PHP bindings. These snippets demonstrate all major features and use cases.

## Directory Structure

```
php/
├── installation/          # Getting started, setup, requirements
├── quickstart/           # Basic usage examples
├── configuration/        # Configuration classes and options
├── extraction/           # Document extraction examples
├── async/               # Async extraction with DeferredResult
├── ocr/                 # OCR and image preprocessing
├── chunking/            # Text chunking for RAG
├── embeddings/          # Vector embeddings and semantic search
├── advanced/            # Error handling, performance tuning
├── cache/               # Caching strategies
├── cli/                 # Command-line tools
└── benchmarking/        # Performance testing
```

## Installation (3 snippets)

### Composer_install.php

Installing Kreuzberg via Composer and verifying the extension is loaded.

### Extension_setup.php

Setting up the native PHP extension (kreuzberg.so/.dll) and checking for optional dependencies (Tesseract, ONNX Runtime).

### Requirements_check.php

Comprehensive system requirements verification script.

## Quickstart (4 snippets)

### Basic_extraction_oop.php

Simple document extraction using the object-oriented API.

### Basic_extraction_procedural.php

Simple extraction using the procedural API for more concise code.

### Extract_from_bytes.php

Extract content from file data in memory (useful for uploaded files).

### Mime_type_detection.php

Automatic MIME type detection from file paths or content.

## Configuration (5 snippets)

### Extraction_config.php

Main ExtractionConfig class - controlling all aspects of extraction.

### Pdf_config.php

PDF-specific settings including image quality and extraction methods.

### Page_config.php

Per-page extraction and page markers for maintaining document structure.

### Language_detection_config.php

Automatic language detection for multilingual documents.

### Keyword_config.php

Automatic keyword extraction for document categorization.

## Extraction (7 snippets)

### Pdf_extraction.php

Extract text, tables, and images from PDF files with various configurations.

### Docx_extraction.php

Extract content from Microsoft Word documents including metadata and tables.

### Image_extraction.php

Extract embedded images from documents with optional OCR.

### Batch_processing.php

Process multiple documents in parallel for maximum performance.

### Table_extraction.php

Extract and process tables, export to CSV, JSON, and HTML formats.

### Metadata_extraction.php

Extract document metadata (title, author, dates, keywords).

### Multi_format.php

Handle various document formats with format-specific processing.

## OCR (3 snippets)

### Basic_ocr.php

Basic OCR with Tesseract for scanned documents and images.

### Advanced_ocr.php

Advanced OCR configuration with Tesseract PSM modes and table detection.

### Image_preprocessing.php

Image preprocessing for better OCR accuracy (denoising, deskewing, sharpening).

## Chunking (1 snippet)

### Basic_chunking.php

Split documents into chunks for RAG applications with various strategies.

## Embeddings (2 snippets)

### Basic_embeddings.php

Generate vector embeddings for semantic search and similarity matching.

### Semantic_search.php

Build a semantic search system using document embeddings.

## Advanced (2 snippets)

### Error_handling.php

Robust error handling, retry strategies, and validation.

### Performance_tuning.php

Performance optimization tips and techniques.

## Cache (1 snippet)

### Disk_cache.php

File-based caching to avoid re-processing documents.

## CLI (2 snippets)

### Basic_cli.php

Simple command-line interface for document extraction.

### Cli_with_config.php

Advanced CLI with support for various extraction options.

## Benchmarking (1 snippet)

### Simple_benchmark.php

Benchmark extraction performance across different configurations.

## Usage Patterns

### Basic Extraction

```php title="Basic Extraction"
use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');
echo $result->content;
```

### With Configuration

```php title="With Configuration"
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
    extractTables: true
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned.pdf');
```

### Procedural API

```php title="Procedural API"
use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');
echo $result->content;
```

### Batch Processing

```php title="Batch Processing"
use function Kreuzberg\batch_extract_files;

$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$results = batch_extract_files($files);
```

## Async Extraction (4 snippets)

### Async_extract_file.php

Async file extraction with DeferredResult polling and blocking patterns.

### Async_batch.php

Async batch extraction with timeout-based waiting.

### Async_amp_bridge.php

Integration with Amp v3+ framework using AmpBridge::toFuture().

### Async_react_bridge.php

Integration with ReactPHP framework using ReactBridge::toPromise().

## Key Features Demonstrated

- **91+ File Formats**: PDF, DOCX, XLSX, PPTX, images, HTML, and more
- **Async Extraction**: Non-blocking extraction with DeferredResult pattern
- **OCR Support**: Tesseract integration with preprocessing
- **Table Extraction**: Extract structured tables with multiple export formats
- **Metadata**: Rich metadata extraction for all formats
- **Batch Processing**: Parallel processing for high throughput
- **Text Chunking**: Intelligent segmentation for RAG applications
- **Embeddings**: Vector embeddings for semantic search
- **Type Safety**: Full PHP 8.1+ type hints and readonly classes
- **Error Handling**: Comprehensive error handling patterns
- **Performance**: Optimization techniques and benchmarking

## Requirements

- PHP 8.1.0 or higher
- Kreuzberg PHP extension (kreuzberg.so/.dll)
- Composer package: kreuzberg/Kreuzberg
- Optional: Tesseract OCR (for OCR functionality)
- Optional: ONNX Runtime (for embeddings)

## Testing Snippets

Each snippet is designed to be self-contained and runnable. To test:

1. Install dependencies:

   ```bash
   composer require kreuzberg/kreuzberg
   ```

2. Ensure the extension is loaded:

   ```bash
   php -m | grep kreuzberg
   ```

3. Run any snippet:
   ```bash
   php docs/snippets/php/quickstart/basic_extraction_oop.php
   ```

## Best Practices

1. **Use batch processing** for multiple files
2. **Disable unnecessary features** (OCR, embeddings) if not needed
3. **Implement caching** for often accessed documents
4. **Handle errors gracefully** with try-catch blocks
5. **Monitor memory usage** for large documents
6. **Use type hints** for better IDE support and safety

## Contributing

These snippets follow these conventions:

- All files use `declare(strict_types=1)`
- Code is wrapped in ````php` markdown code blocks
- Clear comments explain what each snippet demonstrates
- Both OOP and procedural APIs are shown where applicable
- Examples are realistic and production-ready

## Links

- **Documentation**: https://kreuzberg.dev
- **GitHub**: https://github.com/kreuzberg-dev/Kreuzberg
- **Issues**: https://github.com/kreuzberg-dev/kreuzberg/issues
- **Package**: https://packagist.org/packages/kreuzberg/Kreuzberg
