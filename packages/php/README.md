# PHP

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
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
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.2.1" alt="Go">
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
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Documentation">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>


Extract text, tables, images, and metadata from 56 file formats including PDF, Office documents, and images. PHP bindings with modern PHP 8.2+ support and type-safe API.


## Installation

### Package Installation




Install via Composer:

```bash
composer require kreuzberg/kreuzberg
```




### System Requirements

- **PHP 8.0+** required
- Optional: [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) version 1.22.x for embeddings support
- Optional: [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) for OCR functionality



## Quick Start

### Basic Extraction

Extract text, metadata, and structure from any supported document format:

```php title="basic_extraction_oop.php"
<?php

declare(strict_types=1);

/**
 * Basic Document Extraction (OOP API)
 *
 * This example demonstrates the simplest way to extract text from a document
 * using the object-oriented API.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

$result = $kreuzberg->extractFile('document.pdf');

echo "Extracted Content:\n";
echo "==================\n";
echo $result->content . "\n\n";

echo "Metadata:\n";
echo "=========\n";
echo "Title: " . ($result->metadata->title ?? 'N/A') . "\n";
echo "Authors: " . (isset($result->metadata->authors) ? implode(', ', $result->metadata->authors) : 'N/A') . "\n";
echo "Pages: " . ($result->metadata->pageCount ?? 'N/A') . "\n";
echo "Format: " . $result->mimeType . "\n\n";

if (count($result->tables) > 0) {
    echo "Tables Found: " . count($result->tables) . "\n";
    foreach ($result->tables as $index => $table) {
        echo "\nTable " . ($index + 1) . " (Page {$table->pageNumber}):\n";
        echo $table->markdown . "\n";
    }
}
```


### Common Use Cases

#### Extract with Custom Configuration

Most use cases benefit from configuration to control extraction behavior:


**With OCR (for scanned documents):**

```php title="basic_ocr.php"
<?php

declare(strict_types=1);

/**
 * Basic OCR with Tesseract
 *
 * Extract text from scanned PDFs and images using Tesseract OCR.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "OCR Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$multilingualConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+fra+deu'
    )
);

$kreuzberg = new Kreuzberg($multilingualConfig);
$result = $kreuzberg->extractFile('multilingual_scan.pdf');

echo "Multilingual OCR:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->content, 0, 500) . "...\n\n";

$imageConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($imageConfig);

$imageFormats = ['png', 'jpg', 'tiff'];
foreach ($imageFormats as $format) {
    $file = "scan.$format";
    if (file_exists($file)) {
        echo "Processing $file...\n";
        $result = $kreuzberg->extractFile($file);
        echo "Extracted " . strlen($result->content) . " characters\n";
        echo "Preview: " . substr($result->content, 0, 100) . "...\n\n";
    }
}

$languages = [
    'spa' => 'Spanish document',
    'fra' => 'French document',
    'deu' => 'German document',
    'ita' => 'Italian document',
    'por' => 'Portuguese document',
    'rus' => 'Russian document',
    'jpn' => 'Japanese document',
    'chi_sim' => 'Chinese (Simplified) document',
];

foreach ($languages as $lang => $description) {
    $file = strtolower(str_replace(' ', '_', $description)) . '.pdf';

    if (file_exists($file)) {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: $lang
            )
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        echo "$description ($lang):\n";
        echo "  Characters extracted: " . mb_strlen($result->content) . "\n\n";
    }
}

use function Kreuzberg\extract_file;

$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

$result = extract_file('invoice_scan.pdf', config: $config);

echo "Invoice OCR:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n";

$result = $kreuzberg->extractFile('scanned.pdf');

$contentLength = strlen($result->content);
$pageCount = $result->metadata->pageCount ?? 1;
$avgCharsPerPage = $contentLength / $pageCount;

echo "\nOCR Quality Assessment:\n";
echo "Total characters: $contentLength\n";
echo "Pages: $pageCount\n";
echo "Average chars/page: " . number_format($avgCharsPerPage) . "\n";

if ($avgCharsPerPage < 100) {
    echo "Warning: Low character count may indicate poor scan quality\n";
    echo "Consider using image preprocessing or higher DPI settings.\n";
} elseif ($avgCharsPerPage > 2000) {
    echo "Pass: Good - Adequate text extracted\n";
} else {
    echo "Pass: Moderate - Text extracted successfully\n";
}
```




#### Table Extraction


See [Table Extraction Guide](https://kreuzberg.dev/features/table-extraction/) for detailed examples.



#### Processing Multiple Files


```php title="batch_processing.php"
<?php

declare(strict_types=1);

/**
 * Batch Document Processing
 *
 * Process multiple documents in parallel for maximum performance.
 * Kreuzberg's batch API uses multiple threads to extract documents concurrently.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use function Kreuzberg\batch_extract_files;
use function Kreuzberg\batch_extract_bytes;

$files = [
    'document1.pdf',
    'document2.docx',
    'document3.xlsx',
    'presentation.pptx',
];

$files = array_filter($files, 'file_exists');

if (!empty($files)) {
    echo "Processing " . count($files) . " files in batch...\n\n";

    $start = microtime(true);
    $results = batch_extract_files($files);
    $elapsed = microtime(true) - $start;

    echo "Batch extraction completed in " . number_format($elapsed, 3) . " seconds\n";
    echo "Average: " . number_format($elapsed / count($files), 3) . " seconds per file\n\n";

    foreach ($results as $index => $result) {
        $filename = basename($files[$index]);
        echo "$filename:\n";
        echo "  Content: " . strlen($result->content) . " chars\n";
        echo "  Tables: " . count($result->tables) . "\n";
        echo "  MIME: " . $result->mimeType . "\n\n";
    }
}

$config = new ExtractionConfig(
    extractTables: true,
    extractImages: false
);

$kreuzberg = new Kreuzberg($config);

$pdfFiles = glob('*.pdf');
if (!empty($pdfFiles)) {
    echo "Processing " . count($pdfFiles) . " PDF files...\n";

    $start = microtime(true);
    $results = $kreuzberg->batchExtractFiles($pdfFiles, $config);
    $elapsed = microtime(true) - $start;

    echo "Completed in " . number_format($elapsed, 2) . " seconds\n";
    echo "Throughput: " . number_format(count($pdfFiles) / $elapsed, 2) . " files/second\n\n";

    $totalChars = 0;
    $totalTables = 0;

    foreach ($results as $result) {
        $totalChars += strlen($result->content);
        $totalTables += count($result->tables);
    }

    echo "Total content: " . number_format($totalChars) . " characters\n";
    echo "Total tables: $totalTables\n";
}

$uploadedFiles = [
    ['data' => file_get_contents('file1.pdf'), 'mime' => 'application/pdf'],
    ['data' => file_get_contents('file2.docx'), 'mime' => 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
];

$dataList = array_column($uploadedFiles, 'data');
$mimeTypes = array_column($uploadedFiles, 'mime');

$results = batch_extract_bytes($dataList, $mimeTypes);

echo "\nProcessed " . count($results) . " files from memory\n";

function processDirectory(string $dir, Kreuzberg $kreuzberg): array
{
    $results = [];
    $iterator = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($dir)
    );

    $files = [];
    foreach ($iterator as $file) {
        if ($file->isFile()) {
            $ext = strtolower($file->getExtension());
            if (in_array($ext, ['pdf', 'docx', 'xlsx', 'pptx', 'txt'], true)) {
                $files[] = $file->getPathname();
            }
        }
    }

    if (empty($files)) {
        return $results;
    }

    $batches = array_chunk($files, 10);

    foreach ($batches as $batchIndex => $batch) {
        echo "Processing batch " . ($batchIndex + 1) . "/" . count($batches) . "...\n";
        $batchResults = $kreuzberg->batchExtractFiles($batch);
        $results = array_merge($results, $batchResults);
    }

    return $results;
}

$directory = './documents';
if (is_dir($directory)) {
    echo "\nProcessing directory: $directory\n";
    $results = processDirectory($directory, $kreuzberg);
    echo "Processed " . count($results) . " files\n";
}

$mixedFiles = ['valid.pdf', 'nonexistent.pdf', 'another.docx'];

try {
    $results = batch_extract_files($mixedFiles);
} catch (\Kreuzberg\Exceptions\KreuzbergException $e) {
    echo "Batch processing error: " . $e->getMessage() . "\n";
}

$allFiles = glob('documents/*.{pdf,docx,xlsx}', GLOB_BRACE);
$batchSize = 5;
$batches = array_chunk($allFiles, $batchSize);
$totalProcessed = 0;

echo "\nProcessing " . count($allFiles) . " files in " . count($batches) . " batches...\n";

foreach ($batches as $index => $batch) {
    $progress = (($index + 1) / count($batches)) * 100;
    echo sprintf("\rProgress: %.1f%% [%d/%d batches]",
        $progress, $index + 1, count($batches));

    $results = $kreuzberg->batchExtractFiles($batch);
    $totalProcessed += count($results);
}

echo "\n\nCompleted! Processed $totalProcessed files.\n";
```








### Next Steps

- **[Installation Guide](https://kreuzberg.dev/getting-started/installation/)** - Platform-specific setup
- **[API Documentation](https://kreuzberg.dev/api/)** - Complete API reference
- **[Examples & Guides](https://kreuzberg.dev/guides/)** - Full code examples and usage guides
- **[Configuration Guide](https://kreuzberg.dev/guides/configuration/)** - Advanced configuration options



## Features

### Supported File Formats (56+)

56 file formats across 8 major categories with intelligent format detection and comprehensive metadata extraction.

#### Office Documents

| Category | Formats | Capabilities |
|----------|---------|--------------|
| **Word Processing** | `.docx`, `.odt` | Full text, tables, images, metadata, styles |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.ods` | Sheet data, formulas, cell metadata, charts |
| **Presentations** | `.pptx`, `.ppt`, `.ppsx` | Slides, speaker notes, images, metadata |
| **PDF** | `.pdf` | Text, tables, images, metadata, OCR support |
| **eBooks** | `.epub`, `.fb2` | Chapters, metadata, embedded resources |

#### Images (OCR-Enabled)

| Category | Formats | Features |
|----------|---------|----------|
| **Raster** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif` | OCR, table detection, EXIF metadata, dimensions, color space |
| **Advanced** | `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.pnm`, `.pbm`, `.pgm`, `.ppm` | OCR, table detection, format-specific metadata |
| **Vector** | `.svg` | DOM parsing, embedded text, graphics metadata |

#### Web & Data

| Category | Formats | Features |
|----------|---------|----------|
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml`, `.svg` | DOM parsing, metadata (Open Graph, Twitter Card), link extraction |
| **Structured Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` | Schema detection, nested structures, validation |
| **Text & Markdown** | `.txt`, `.md`, `.markdown`, `.rst`, `.org`, `.rtf` | CommonMark, GFM, reStructuredText, Org Mode |

#### Email & Archives

| Category | Formats | Features |
|----------|---------|----------|
| **Email** | `.eml`, `.msg` | Headers, body (HTML/plain), attachments, threading |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` | File listing, nested archives, metadata |

#### Academic & Scientific

| Category | Formats | Features |
|----------|---------|----------|
| **Citations** | `.bib`, `.biblatex`, `.ris`, `.enw`, `.csl` | Bibliography parsing, citation extraction |
| **Scientific** | `.tex`, `.latex`, `.typst`, `.jats`, `.ipynb`, `.docbook` | LaTeX, Jupyter notebooks, PubMed JATS |
| **Documentation** | `.opml`, `.pod`, `.mdoc`, `.troff` | Technical documentation formats |

**[Complete Format Reference](https://kreuzberg.dev/reference/formats/)**

### Key Capabilities

- **Text Extraction** - Extract all text content with position and formatting information
- **Metadata Extraction** - Retrieve document properties, creation date, author, etc.
- **Table Extraction** - Parse tables with structure and cell content preservation
- **Image Extraction** - Extract embedded images and render page previews
- **OCR Support** - Integrate multiple OCR backends for scanned documents


- **Plugin System** - Extensible post-processing for custom text transformation


- **Embeddings** - Generate vector embeddings using ONNX Runtime models

- **Batch Processing** - Efficiently process multiple documents in parallel
- **Memory Efficient** - Stream large files without loading entirely into memory
- **Language Detection** - Detect and support multiple languages in documents
- **Configuration** - Fine-grained control over extraction behavior

### Performance Characteristics

| Format | Speed | Memory | Notes |
|--------|-------|--------|-------|
| **PDF (text)** | 10-100 MB/s | ~50MB per doc | Fastest extraction |
| **Office docs** | 20-200 MB/s | ~100MB per doc | DOCX, XLSX, PPTX |
| **Images (OCR)** | 1-5 MB/s | Variable | Depends on OCR backend |
| **Archives** | 5-50 MB/s | ~200MB per doc | ZIP, TAR, etc. |
| **Web formats** | 50-200 MB/s | Streaming | HTML, XML, JSON |



## OCR Support

Kreuzberg supports multiple OCR backends for extracting text from scanned documents and images:


- **Tesseract**


### OCR Configuration Example

```php title="basic_ocr.php"
<?php

declare(strict_types=1);

/**
 * Basic OCR with Tesseract
 *
 * Extract text from scanned PDFs and images using Tesseract OCR.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_document.pdf');

echo "OCR Extraction Results:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n\n";

$multilingualConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng+fra+deu'
    )
);

$kreuzberg = new Kreuzberg($multilingualConfig);
$result = $kreuzberg->extractFile('multilingual_scan.pdf');

echo "Multilingual OCR:\n";
echo str_repeat('=', 60) . "\n";
echo substr($result->content, 0, 500) . "...\n\n";

$imageConfig = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($imageConfig);

$imageFormats = ['png', 'jpg', 'tiff'];
foreach ($imageFormats as $format) {
    $file = "scan.$format";
    if (file_exists($file)) {
        echo "Processing $file...\n";
        $result = $kreuzberg->extractFile($file);
        echo "Extracted " . strlen($result->content) . " characters\n";
        echo "Preview: " . substr($result->content, 0, 100) . "...\n\n";
    }
}

$languages = [
    'spa' => 'Spanish document',
    'fra' => 'French document',
    'deu' => 'German document',
    'ita' => 'Italian document',
    'por' => 'Portuguese document',
    'rus' => 'Russian document',
    'jpn' => 'Japanese document',
    'chi_sim' => 'Chinese (Simplified) document',
];

foreach ($languages as $lang => $description) {
    $file = strtolower(str_replace(' ', '_', $description)) . '.pdf';

    if (file_exists($file)) {
        $config = new ExtractionConfig(
            ocr: new OcrConfig(
                backend: 'tesseract',
                language: $lang
            )
        );

        $kreuzberg = new Kreuzberg($config);
        $result = $kreuzberg->extractFile($file);

        echo "$description ($lang):\n";
        echo "  Characters extracted: " . mb_strlen($result->content) . "\n\n";
    }
}

use function Kreuzberg\extract_file;

$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);

$result = extract_file('invoice_scan.pdf', config: $config);

echo "Invoice OCR:\n";
echo str_repeat('=', 60) . "\n";
echo $result->content . "\n";

$result = $kreuzberg->extractFile('scanned.pdf');

$contentLength = strlen($result->content);
$pageCount = $result->metadata->pageCount ?? 1;
$avgCharsPerPage = $contentLength / $pageCount;

echo "\nOCR Quality Assessment:\n";
echo "Total characters: $contentLength\n";
echo "Pages: $pageCount\n";
echo "Average chars/page: " . number_format($avgCharsPerPage) . "\n";

if ($avgCharsPerPage < 100) {
    echo "Warning: Low character count may indicate poor scan quality\n";
    echo "Consider using image preprocessing or higher DPI settings.\n";
} elseif ($avgCharsPerPage > 2000) {
    echo "Pass: Good - Adequate text extracted\n";
} else {
    echo "Pass: Moderate - Text extracted successfully\n";
}
```





## Plugin System

Kreuzberg supports extensible post-processing plugins for custom text transformation and filtering.

For detailed plugin documentation, visit [Plugin System Guide](https://kreuzberg.dev/guides/plugins/).




## Embeddings Support

Generate vector embeddings for extracted text using the built-in ONNX Runtime support. Requires ONNX Runtime installation.

**[Embeddings Guide](https://kreuzberg.dev/features/#embeddings)**



## Batch Processing

Process multiple documents efficiently:

```php title="batch_processing.php"
<?php

declare(strict_types=1);

/**
 * Batch Document Processing
 *
 * Process multiple documents in parallel for maximum performance.
 * Kreuzberg's batch API uses multiple threads to extract documents concurrently.
 */

require_once __DIR__ . '/vendor/autoload.php';

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use function Kreuzberg\batch_extract_files;
use function Kreuzberg\batch_extract_bytes;

$files = [
    'document1.pdf',
    'document2.docx',
    'document3.xlsx',
    'presentation.pptx',
];

$files = array_filter($files, 'file_exists');

if (!empty($files)) {
    echo "Processing " . count($files) . " files in batch...\n\n";

    $start = microtime(true);
    $results = batch_extract_files($files);
    $elapsed = microtime(true) - $start;

    echo "Batch extraction completed in " . number_format($elapsed, 3) . " seconds\n";
    echo "Average: " . number_format($elapsed / count($files), 3) . " seconds per file\n\n";

    foreach ($results as $index => $result) {
        $filename = basename($files[$index]);
        echo "$filename:\n";
        echo "  Content: " . strlen($result->content) . " chars\n";
        echo "  Tables: " . count($result->tables) . "\n";
        echo "  MIME: " . $result->mimeType . "\n\n";
    }
}

$config = new ExtractionConfig(
    extractTables: true,
    extractImages: false
);

$kreuzberg = new Kreuzberg($config);

$pdfFiles = glob('*.pdf');
if (!empty($pdfFiles)) {
    echo "Processing " . count($pdfFiles) . " PDF files...\n";

    $start = microtime(true);
    $results = $kreuzberg->batchExtractFiles($pdfFiles, $config);
    $elapsed = microtime(true) - $start;

    echo "Completed in " . number_format($elapsed, 2) . " seconds\n";
    echo "Throughput: " . number_format(count($pdfFiles) / $elapsed, 2) . " files/second\n\n";

    $totalChars = 0;
    $totalTables = 0;

    foreach ($results as $result) {
        $totalChars += strlen($result->content);
        $totalTables += count($result->tables);
    }

    echo "Total content: " . number_format($totalChars) . " characters\n";
    echo "Total tables: $totalTables\n";
}

$uploadedFiles = [
    ['data' => file_get_contents('file1.pdf'), 'mime' => 'application/pdf'],
    ['data' => file_get_contents('file2.docx'), 'mime' => 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
];

$dataList = array_column($uploadedFiles, 'data');
$mimeTypes = array_column($uploadedFiles, 'mime');

$results = batch_extract_bytes($dataList, $mimeTypes);

echo "\nProcessed " . count($results) . " files from memory\n";

function processDirectory(string $dir, Kreuzberg $kreuzberg): array
{
    $results = [];
    $iterator = new RecursiveIteratorIterator(
        new RecursiveDirectoryIterator($dir)
    );

    $files = [];
    foreach ($iterator as $file) {
        if ($file->isFile()) {
            $ext = strtolower($file->getExtension());
            if (in_array($ext, ['pdf', 'docx', 'xlsx', 'pptx', 'txt'], true)) {
                $files[] = $file->getPathname();
            }
        }
    }

    if (empty($files)) {
        return $results;
    }

    $batches = array_chunk($files, 10);

    foreach ($batches as $batchIndex => $batch) {
        echo "Processing batch " . ($batchIndex + 1) . "/" . count($batches) . "...\n";
        $batchResults = $kreuzberg->batchExtractFiles($batch);
        $results = array_merge($results, $batchResults);
    }

    return $results;
}

$directory = './documents';
if (is_dir($directory)) {
    echo "\nProcessing directory: $directory\n";
    $results = processDirectory($directory, $kreuzberg);
    echo "Processed " . count($results) . " files\n";
}

$mixedFiles = ['valid.pdf', 'nonexistent.pdf', 'another.docx'];

try {
    $results = batch_extract_files($mixedFiles);
} catch (\Kreuzberg\Exceptions\KreuzbergException $e) {
    echo "Batch processing error: " . $e->getMessage() . "\n";
}

$allFiles = glob('documents/*.{pdf,docx,xlsx}', GLOB_BRACE);
$batchSize = 5;
$batches = array_chunk($allFiles, $batchSize);
$totalProcessed = 0;

echo "\nProcessing " . count($allFiles) . " files in " . count($batches) . " batches...\n";

foreach ($batches as $index => $batch) {
    $progress = (($index + 1) / count($batches)) * 100;
    echo sprintf("\rProgress: %.1f%% [%d/%d batches]",
        $progress, $index + 1, count($batches));

    $results = $kreuzberg->batchExtractFiles($batch);
    $totalProcessed += count($results);
}

echo "\n\nCompleted! Processed $totalProcessed files.\n";
```




## Configuration

For advanced configuration options including language detection, table extraction, OCR settings, and more:

**[Configuration Guide](https://kreuzberg.dev/guides/configuration/)**

## Documentation

- **[Official Documentation](https://kreuzberg.dev/)**
- **[API Reference](https://kreuzberg.dev/reference/api-php/)**
- **[Examples & Guides](https://kreuzberg.dev/guides/)**

## Contributing

Contributions are welcome! See [Contributing Guide](https://github.com/kreuzberg-dev/kreuzberg/blob/main/CONTRIBUTING.md).

## License

MIT License - see LICENSE file for details.

## Support

- **Discord Community**: [Join our Discord](https://discord.gg/xt9WY3GnKR)
- **GitHub Issues**: [Report bugs](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Discussions**: [Ask questions](https://github.com/kreuzberg-dev/kreuzberg/discussions)
