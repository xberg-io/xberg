# PHP API Reference

Complete reference for the Kreuzberg PHP API.

## Installation

### Composer Installation

```bash title="Terminal"
composer require kreuzberg/kreuzberg
```

### PHP Extension Setup

The Kreuzberg PHP package requires the native extension to be installed and enabled.

> **Windows Notice**: The Windows PHP extension is temporarily unavailable due to a transitive dependency conflict in `ort-sys`. Linux and macOS packages are fully supported. See [Platform Support](#platform-support) for more details.

**1. Install the extension:**

Download the appropriate extension for your platform from the releases page, or build from source.

**2. Enable the extension:**

Add to your `php.ini`:

```ini title="php.ini"
extension=kreuzberg.so  # Linux/macOS
extension=kreuzberg.dll  # Windows
```

**3. Verify installation:**

```php title="verify.php"
<?php

if (!extension_loaded('kreuzberg')) {
    die("Kreuzberg extension not loaded\n");
}

echo "Kreuzberg extension loaded successfully!\n";
echo "Version: " . \Kreuzberg\Kreuzberg::version() . "\n";
```

## Platform Support

- **Linux (x86_64, aarch64)**: Fully supported
- **macOS (Intel, Apple Silicon)**: Fully supported
- **Windows**: Temporarily unavailable due to a transitive dependency conflict (`ort-sys` → `lzma-rust2` → `crc` version collision on the `x86_64-pc-windows-gnu` target). Will be resolved when upstream `ort` updates its `lzma-rust2` dependency. Please use Linux or macOS for now, or deploy using Docker.

## Quick Start

### Basic Usage (OOP API)

```php title="basic_oop.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');

echo $result->content;
echo "Pages: {$result->metadata->pageCount}\n";
```

### Basic Usage (Procedural API)

```php title="basic_procedural.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');

echo $result->content;
echo "Pages: {$result->metadata->pageCount}\n";
```

### Extract from Bytes

```php title="extract_bytes.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();
$data = file_get_contents('document.pdf');
$result = $kreuzberg->extractBytes($data, 'application/pdf');

echo $result->content;
```

---

## Core Classes

### Kreuzberg

Main API class for document extraction.

**Signature:**

```php title="PHP"
final readonly class Kreuzberg
{
    public function __construct(?ExtractionConfig $defaultConfig = null);

    public function extractFile(
        string $filePath,
        ?string $mimeType = null,
        ?ExtractionConfig $config = null
    ): ExtractionResult;

    public function extractBytes(
        string $data,
        string $mimeType,
        ?ExtractionConfig $config = null
    ): ExtractionResult;

    public function batchExtractFiles(
        array $paths,
        ?ExtractionConfig $config = null
    ): array;

    public function batchExtractBytes(
        array $dataList,
        array $mimeTypes,
        ?ExtractionConfig $config = null
    ): array;

    public static function version(): string;
}
```

**Example:**

```php title="kreuzberg_class.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;

// Create with default config
$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('document.pdf');

// Create with custom config
$config = new ExtractionConfig(extractTables: true);
$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

// Override config per call
$overrideConfig = new ExtractionConfig(extractImages: true);
$result = $kreuzberg->extractFile('document.pdf', config: $overrideConfig);
```

---

### Kreuzberg::extractFile()

Extract content from a file.

**Signature:**

```php title="PHP"
public function extractFile(
    string $filePath,
    ?string $mimeType = null,
    ?ExtractionConfig $config = null
): ExtractionResult
```

**Parameters:**

- `$filePath` (string): Path to the file to extract
- `$mimeType` (string|null): Optional MIME type hint. If null, MIME type is auto-detected
- `$config` (ExtractionConfig|null): Extraction configuration. Uses constructor config if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `KreuzbergException`: If extraction fails

**Examples:**

```php title="extract_file_examples.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$kreuzberg = new Kreuzberg();

// Basic extraction
$result = $kreuzberg->extractFile('document.pdf');

// With MIME type hint
$result = $kreuzberg->extractFile('document.pdf', 'application/pdf');

// With configuration
$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
);
$result = $kreuzberg->extractFile('scanned.pdf', config: $config);
```

---

### Kreuzberg::extractBytes()

Extract content from bytes.

**Signature:**

```php title="PHP"
public function extractBytes(
    string $data,
    string $mimeType,
    ?ExtractionConfig $config = null
): ExtractionResult
```

**Parameters:**

- `$data` (string): File content as bytes
- `$mimeType` (string): MIME type of the data (required for format detection)
- `$config` (ExtractionConfig|null): Extraction configuration. Uses constructor config if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```php title="extract_bytes_examples.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

// Extract from file data
$data = file_get_contents('document.pdf');
$result = $kreuzberg->extractBytes($data, 'application/pdf');

// Extract from uploaded file
$uploadedData = $_FILES['document']['tmp_name'];
$data = file_get_contents($uploadedData);
$result = $kreuzberg->extractBytes($data, 'application/pdf');
```

---

### Kreuzberg::batchExtractFiles()

Extract content from multiple files in parallel.

**Signature:**

```php title="PHP"
public function batchExtractFiles(
    array $paths,
    ?ExtractionConfig $config = null
): array
```

**Parameters:**

- `$paths` (array<string>): Array of file paths to extract
- `$config` (ExtractionConfig|null): Extraction configuration applied to all files

**Returns:**

- `array<ExtractionResult>`: Array of extraction results (one per file)

**Examples:**

```php title="batch_extract_files.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$results = $kreuzberg->batchExtractFiles($files);

foreach ($results as $i => $result) {
    echo "{$files[$i]}: {$result->content}\n";
}
```

---

### Kreuzberg::batchExtractBytes()

Extract content from multiple byte arrays in parallel.

**Signature:**

```php title="PHP"
public function batchExtractBytes(
    array $dataList,
    array $mimeTypes,
    ?ExtractionConfig $config = null
): array
```

**Parameters:**

- `$dataList` (array<string>): Array of file contents as bytes
- `$mimeTypes` (array<string>): Array of MIME types (one per data item, same length as dataList)
- `$config` (ExtractionConfig|null): Extraction configuration applied to all items

**Returns:**

- `array<ExtractionResult>`: Array of extraction results (one per data item)

**Examples:**

```php title="batch_extract_bytes.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

$dataList = [
    file_get_contents('doc1.pdf'),
    file_get_contents('doc2.docx'),
];

$mimeTypes = [
    'application/pdf',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
];

$results = $kreuzberg->batchExtractBytes($dataList, $mimeTypes);
```

---

## Procedural Functions

### Extract_file()

Extract content from a file (procedural API).

**Signature:**

```php title="PHP"
function extract_file(
    string $filePath,
    ?string $mimeType = null,
    ?ExtractionConfig $config = null
): ExtractionResult
```

**Parameters:**

Same as [`Kreuzberg::extractFile()`](#kreuzbergextractfile).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```php title="procedural_extract_file.php"
<?php

use function Kreuzberg\extract_file;
use Kreuzberg\Config\ExtractionConfig;

// Basic extraction
$result = extract_file('document.pdf');

// With configuration
$config = new ExtractionConfig(extractTables: true);
$result = extract_file('document.pdf', config: $config);
```

---

### Extract_bytes()

Extract content from bytes (procedural API).

**Signature:**

```php title="PHP"
function extract_bytes(
    string $data,
    string $mimeType,
    ?ExtractionConfig $config = null
): ExtractionResult
```

**Parameters:**

Same as [`Kreuzberg::extractBytes()`](#kreuzbergextractbytes).

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

---

### Batch_extract_files()

Extract content from multiple files in parallel (procedural API).

**Signature:**

```php title="PHP"
function batch_extract_files(
    array $paths,
    ?ExtractionConfig $config = null
): array
```

**Parameters:**

Same as [`Kreuzberg::batchExtractFiles()`](#kreuzbergbatchextractfiles).

**Returns:**

- `array<ExtractionResult>`: Array of extraction results (one per file)

**Examples:**

```php title="procedural_batch.php"
<?php

use function Kreuzberg\batch_extract_files;

$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$results = batch_extract_files($files);

foreach ($results as $result) {
    echo $result->content;
}
```

---

### Batch_extract_bytes()

Extract content from multiple byte arrays in parallel (procedural API).

**Signature:**

```php title="PHP"
function batch_extract_bytes(
    array $dataList,
    array $mimeTypes,
    ?ExtractionConfig $config = null
): array
```

**Parameters:**

Same as [`Kreuzberg::batchExtractBytes()`](#kreuzbergbatchextractbytes).

**Returns:**

- `array<ExtractionResult>`: Array of extraction results (one per data item)

---

### FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All properties are nullable — `null` means "use the batch-level default."

```php title="PHP"
class FileExtractionConfig
{
    public ?bool $enableQualityProcessing = null;
    public ?OcrConfig $ocr = null;
    public ?bool $forceOcr = null;
    public ?ChunkingConfig $chunking = null;
    public ?ImageExtractionConfig $images = null;
    public ?PdfConfig $pdfOptions = null;
    public ?TokenReductionConfig $tokenReduction = null;
    public ?LanguageDetectionConfig $languageDetection = null;
    public ?PageConfig $pages = null;
    public ?PostProcessorConfig $postprocessor = null;
    public ?string $outputFormat = null;
    public ?string $resultFormat = null;
    public ?bool $includeDocumentStructure = null;
}
```

Batch-level fields (`$maxConcurrentExtractions`, `$useCache`, `$securityLimits`) cannot be overridden per file. See [Configuration Reference](configuration.md#fileextractionconfig) for details.

---

### Detect_mime_type()

Detect MIME type from file bytes.

**Signature:**

```php title="PHP"
function detect_mime_type(string $data): string
```

**Parameters:**

- `$data` (string): File content as bytes

**Returns:**

- `string`: Detected MIME type (for example, "application/pdf", "image/png")

**Examples:**

```php title="detect_mime.php"
<?php

use function Kreuzberg\detect_mime_type;

$data = file_get_contents('unknown.file');
$mimeType = detect_mime_type($data);
echo $mimeType; // "application/pdf"
```

---

### Detect_mime_type_from_path()

Detect MIME type from file path.

**Signature:**

```php title="PHP"
function detect_mime_type_from_path(string $path): string
```

**Parameters:**

- `$path` (string): Path to the file

**Returns:**

- `string`: Detected MIME type (for example, "application/pdf", "text/plain")

**Examples:**

```php title="detect_mime_path.php"
<?php

use function Kreuzberg\detect_mime_type_from_path;

$mimeType = detect_mime_type_from_path('document.pdf');
echo $mimeType; // "application/pdf"
```

---

## Configuration

!!! Warning "Deprecated API"
    The `$outputFormat` property has been deprecated in favor of the new configuration object approach.

    **Old pattern (no longer supported):**
    ```php
    $config = new ExtractionConfig();
    $config->outputFormat = 'markdown';
    ```

    **New pattern:**
    ```php
    $config = new ExtractionConfig();
    $config->outputFormat = OutputFormat::MARKDOWN;
    ```

    For more control, use the full configuration builder with `OutputConfig` object.

### ExtractionConfig

Main configuration class for extraction operations.

**Signature:**

```php title="PHP"
readonly class ExtractionConfig
{
    public function __construct(
        public ?ChunkingConfig $chunking = null,
        public ?ConcurrencyConfig $concurrency = null,
        public bool $enableQualityProcessing = true,
        public bool $extractImages = false,
        public bool $extractTables = true,
        public bool $forceOcr = false,
        public ?array $htmlOptions = null,
        public ?ImageExtractionConfig $imageExtraction = null,
        public bool $includeDocumentStructure = false,
        public ?KeywordConfig $keyword = null,
        public ?LanguageDetectionConfig $languageDetection = null,
        public ?int $maxConcurrentExtractions = null,
        public ?OcrConfig $ocr = null,
        public string $outputFormat = 'plain',
        public ?PageConfig $page = null,
        public ?PdfConfig $pdf = null,
        public ?PostProcessorConfig $postprocessor = null,
        public string $resultFormat = 'unified',
        public ?array $securityLimits = null,
        public ?TokenReductionConfig $tokenReduction = null,
        public bool $useCache = true,
    );
}
```

**Fields:**

- `$chunking` (ChunkingConfig|null): Text chunking configuration. Default: null
- `$concurrency` (ConcurrencyConfig|null): Concurrency configuration for extraction parallelization. Default: null
- `$enableQualityProcessing` (bool): Enable quality post-processing enhancements. Default: true
- `$extractImages` (bool): Extract images from documents. Default: false
- `$extractTables` (bool): Extract tables from documents. Default: true
- `$forceOcr` (bool): Force OCR on all documents regardless of document type. Default: false
- `$htmlOptions` (array<string, mixed>|null): HTML to Markdown conversion options. Default: null
- `$imageExtraction` (ImageExtractionConfig|null): Image extraction configuration. Default: null
- `$includeDocumentStructure` (bool): Include hierarchical document structure in results. Default: false
- `$keyword` (KeywordConfig|null): Keyword extraction configuration. Default: null
- `$languageDetection` (LanguageDetectionConfig|null): Language detection configuration. Default: null
- `$maxConcurrentExtractions` (int|null): Maximum concurrent extractions for batch operations. Default: null
- `$ocr` (OcrConfig|null): OCR configuration. Default: null (no OCR)
- `$outputFormat` (string): Output format for extracted content ('plain', 'markdown', 'djot', 'html'). Default: 'plain'
- `$page` (PageConfig|null): Page extraction configuration. Default: null
- `$pdf` (PdfConfig|null): PDF-specific configuration. Default: null
- `$postprocessor` (PostProcessorConfig|null): Post-processor configuration. Default: null
- `$resultFormat` (string): Result format ('unified', 'element_based'). Default: 'unified'
- `$securityLimits` (array<string, int>|null): Security limits for archive extraction. Default: null
- `$tokenReduction` (TokenReductionConfig|null): Token reduction configuration. Default: null
- `$useCache` (bool): Enable caching of extraction results. Default: true

**Examples:**

```php title="extraction_config_examples.php"
<?php

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\PdfConfig;

// Basic configuration
$config = new ExtractionConfig(
    extractImages: true,
    extractTables: true
);

// Advanced configuration
$config = new ExtractionConfig(
    chunking: new ChunkingConfig(maxChunkSize: 1000),
    extractImages: true,
    extractTables: true,
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    ),
    pdf: new PdfConfig(
        extractImages: true,
        extractMetadata: true
    )
);
```

---

### ExtractionConfigBuilder

Builder class for constructing `ExtractionConfig` instances with a fluent interface. This is the recommended approach for complex configurations.

**Signature:**

```php title="PHP"
class ExtractionConfigBuilder
{
    public function build(): ExtractionConfig;
    public function withChunking(?ChunkingConfig $chunking): self;
    public function withConcurrency(?ConcurrencyConfig $concurrency): self;
    public function withEnableQualityProcessing(bool $enableQualityProcessing): self;
    public function withForceOcr(bool $forceOcr): self;
    public function withHtmlOptions(?array $htmlOptions = null): self;
    public function withImages(?ImageExtractionConfig $images): self;
    public function withKeywords(?KeywordConfig $keywords): self;
    public function withLanguageDetection(?LanguageDetectionConfig $languageDetection): self;
    public function withMaxConcurrentExtractions(?int $maxConcurrentExtractions): self;
    public function withOcr(?OcrConfig $ocr): self;
    public function withOutputFormat(string $outputFormat): self;
    public function withPages(?PageConfig $pages): self;
    public function withPdfOptions(?PdfConfig $pdfOptions): self;
    public function withPostprocessor(?PostProcessorConfig $postprocessor): self;
    public function withResultFormat(string $resultFormat): self;
    public function withTokenReduction(?TokenReductionConfig $tokenReduction): self;
    public function withUseCache(bool $useCache): self;
}
```

**Examples:**

```php title="extraction_config_builder.php"
<?php

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\ChunkingConfig;

$config = ExtractionConfig::builder()
    ->withOcr(new OcrConfig(backend: 'tesseract', language: 'eng'))
    ->withChunking(new ChunkingConfig(maxChunkSize: 1000))
    ->withUseCache(true)
    ->withMaxConcurrentExtractions(8)
    ->build();
```

---

### OcrConfig

OCR processing configuration.

**Signature:**

```php title="PHP"
readonly class OcrConfig
{
    public function __construct(
        public string $backend = 'tesseract',
        public string $language = 'eng',
        public ?TesseractConfig $tesseractConfig = null,
    );
}
```

**Fields:**

- `$backend` (string): OCR backend to use. Options: "tesseract", "paddle-ocr". Default: "tesseract"
- `$language` (string): Language code for OCR (ISO 639-3). Default: "eng"
- `$tesseractConfig` (TesseractConfig|null): Tesseract-specific configuration. Default: null
- `$paddleOcrConfig` (PaddleOcrConfig|null): <span class="version-badge">v4.5.0</span> PaddleOCR-specific configuration. Fields: `$modelTier` (string, "mobile" or "server", default "mobile"), `$padding` (int, 0-100, default 10). Default: null

**Examples:**

```php title="ocr_config_examples.php"
<?php

use Kreuzberg\Config\OcrConfig;

// Basic OCR
$config = new OcrConfig(
    backend: 'tesseract',
    language: 'eng'
);

// Multilingual OCR
$config = new OcrConfig(
    backend: 'tesseract',
    language: 'eng+fra+deu'
);
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Signature:**

```php title="PHP"
readonly class TesseractConfig
{
    public function __construct(
        public int $psm = 3,
        public int $oem = 3,
        public bool $enableTableDetection = false,
        public ?string $tesseditCharWhitelist = null,
        public ?string $tesseditCharBlacklist = null,
    );
}
```

**Fields:**

- `$psm` (int): Page segmentation mode (0-13). Default: 3 (auto)
- `$oem` (int): OCR engine mode (0-3). Default: 3 (LSTM only)
- `$enableTableDetection` (bool): Enable table detection and extraction. Default: false
- `$tesseditCharWhitelist` (string|null): Character whitelist (for example, "0123456789" for digits only). Default: null
- `$tesseditCharBlacklist` (string|null): Character blacklist. Default: null

**Examples:**

```php title="tesseract_config_examples.php"
<?php

use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;

$config = new OcrConfig(
    backend: 'tesseract',
    language: 'eng',
    tesseractConfig: new TesseractConfig(
        psm: 6,
        enableTableDetection: true,
        tesseditCharWhitelist: '0123456789'
    )
);
```

---

### PdfConfig

PDF-specific configuration.

**Signature:**

```php title="PHP"
readonly class PdfConfig
{
    public function __construct(
        public bool $extractImages = false,
        public bool $extractMetadata = true,
        public bool $ocrFallback = false,
        public ?int $startPage = null,
        public ?int $endPage = null,
        public int $imageQuality = 95,
        public bool $allowSingleColumnTables = false,
    );
}
```

**Fields:**

- `$extractImages` (bool): Extract images from PDF. Default: false
- `$extractMetadata` (bool): Extract PDF metadata. Default: true
- `$ocrFallback` (bool): Use OCR if text extraction fails. Default: false
- `$startPage` (int|null): Start page for extraction (0-indexed). Default: null (all pages)
- `$endPage` (int|null): End page for extraction (0-indexed). Default: null (all pages)
- `$imageQuality` (int): JPEG quality for extracted images (1-100). Default: 95
- `$allowSingleColumnTables` (bool): <span class="version-badge">v4.5.0</span> Allow extraction of single-column tables. Default: false

**Examples:**

```php title="pdf_config_examples.php"
<?php

use Kreuzberg\Config\PdfConfig;

// Extract specific page range
$config = new PdfConfig(
    extractImages: true,
    startPage: 0,
    endPage: 4
);

// OCR fallback for scanned PDFs
$config = new PdfConfig(
    ocrFallback: true,
    extractImages: true
);
```

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Signature:**

```php title="PHP"
readonly class ChunkingConfig
{
    public function __construct(
        public int $maxChunkSize = 512,
        public int $chunkOverlap = 50,
        public bool $respectSentences = true,
        public bool $respectParagraphs = false,
        public ?string $sizingType = null,
        public ?string $sizingModel = null,
        public ?string $sizingCacheDir = null,
        public string $chunkerType = 'text',
        public bool $prependHeadingContext = false,
    );
}
```

**Fields:**

- `$maxChunkSize` (int): Maximum chunk size in characters. Default: 512
- `$chunkOverlap` (int): Overlap between chunks in characters. Default: 50
- `$respectSentences` (bool): Try to split at sentence boundaries. Default: true
- `$respectParagraphs` (bool): Try to split at paragraph boundaries. Default: false
- `$sizingType` (string|null): How chunk size is measured. Options: `"characters"` (default) or `"tokenizer"` (use a HuggingFace tokenizer). Default: null (characters)
- `$sizingModel` (string|null): HuggingFace model ID for tokenizer-based sizing (for example `"bert-base-uncased"`). Required when `$sizingType` is `"tokenizer"`. Default: null
- `$sizingCacheDir` (string|null): Optional directory to cache downloaded tokenizer files. Default: null
- `$chunkerType` (string): Type of chunker to use. Options: `"text"` (default), `"markdown"`, `"yaml"`. Default: `"text"`
- `$prependHeadingContext` (bool): When true, prepends heading hierarchy path to each chunk's content. Most useful with `chunkerType: "markdown"`. Default: false

**Examples:**

```php title="chunking_config_examples.php"
<?php

use Kreuzberg\Config\ChunkingConfig;

$config = new ChunkingConfig(
    maxChunkSize: 1000,
    chunkOverlap: 100,
    respectSentences: true,
    respectParagraphs: true
);
```

---

### ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Concurrency configuration for extraction parallelization.

**Signature:**

```php title="PHP"
readonly class ConcurrencyConfig
{
    public function __construct(
        public ?int $maxThreads = null,
    );
}
```

**Fields:**

- `$maxThreads` (int|null): Maximum number of threads for parallel extraction. Default: null (uses system default)

---

### EmbeddingConfig

Embedding generation configuration.

**Signature:**

```php title="PHP"
readonly class EmbeddingConfig
{
    public function __construct(
        public string $model = 'all-MiniLM-L6-v2',
        public bool $normalize = true,
        public int $batchSize = 32,
    );
}
```

**Fields:**

- `$model` (string): Embedding model name. Default: "all-MiniLM-L6-v2"
- `$normalize` (bool): Normalize embeddings. Default: true
- `$batchSize` (int): Batch size for embedding generation. Default: 32

---

### ImageExtractionConfig

Image extraction configuration.

**Signature:**

```php title="PHP"
readonly class ImageExtractionConfig
{
    public function __construct(
        public bool $extractImages = false,
        public bool $performOcr = false,
        public int $minWidth = 100,
        public int $minHeight = 100,
    );
}
```

**Fields:**

- `$extractImages` (bool): Enable image extraction from documents. Default: false
- `$performOcr` (bool): Perform OCR on extracted images. Default: false
- `$minWidth` (int): Minimum image width in pixels. Default: 100
- `$minHeight` (int): Minimum image height in pixels. Default: 100

**Examples:**

```php title="image_extraction_config.php"
<?php

use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ImageExtractionConfig(
    extractImages: true,
    performOcr: true,
    minWidth: 200,
    minHeight: 200
);
```

---

### PageConfig

Page extraction configuration.

**Signature:**

```php title="PHP"
readonly class PageConfig
{
    public function __construct(
        public bool $extractPages = false,
        public bool $insertPageMarkers = false,
        public string $markerFormat = '--- Page {page_number} ---',
    );
}
```

**Fields:**

- `$extractPages` (bool): Extract per-page content. Default: false
- `$insertPageMarkers` (bool): Insert page markers in content. Default: false
- `$markerFormat` (string): Page marker format string. Default: "--- Page {page_number} ---"

**Examples:**

```php title="page_config_examples.php"
<?php

use Kreuzberg\Config\PageConfig;

$config = new PageConfig(
    extractPages: true,
    insertPageMarkers: true,
    markerFormat: '=== Page {page_number} ==='
);
```

---

### LanguageDetectionConfig

Language detection configuration.

**Signature:**

```php title="PHP"
readonly class LanguageDetectionConfig
{
    public function __construct(
        public bool $enabled = true,
        public int $maxLanguages = 3,
        public float $confidenceThreshold = 0.8,
    );
}
```

**Fields:**

- `$enabled` (bool): Enable language detection. Default: true
- `$maxLanguages` (int): Maximum number of languages to detect. Default: 3
- `$confidenceThreshold` (float): Minimum confidence threshold (0.0-1.0). Default: 0.8

**Examples:**

```php title="language_detection_config.php"
<?php

use Kreuzberg\Config\LanguageDetectionConfig;

$config = new LanguageDetectionConfig(
    enabled: true,
    maxLanguages: 5,
    confidenceThreshold: 0.9
);
```

---

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Layout detection configuration (requires `layout-detection` feature).

**Signature:**

```php title="PHP"
readonly class LayoutDetectionConfig
{
    public function __construct(
        public ?float $confidenceThreshold = null,
        public bool $applyHeuristics = true,
        public ?string $tableModel = null,
    );
}
```

**Fields:**

- `$confidenceThreshold` (float|null): Confidence threshold for layout detection (0.0-1.0). Default: `null`
- `$applyHeuristics` (bool): Apply post-processing heuristics to refine layout results. Default: `true`
- `$tableModel` (string|null): Table structure recognition model. Options: `"tatr"` (default), `"slanet_wired"`, `"slanet_wireless"`, `"slanet_plus"`, `"slanet_auto"`. Default: `null` (uses `"tatr"`)

**Example:**

```php title="layout_detection_config_example.php"
<?php

use Kreuzberg\Config\LayoutDetectionConfig;
use Kreuzberg\Config\ExtractionConfig;

$config = new ExtractionConfig(
    layout: new LayoutDetectionConfig(
        confidenceThreshold: 0.5,
        applyHeuristics: true
    )
);
```

---

### KeywordConfig

Keyword extraction configuration.

**Signature:**

```php title="PHP"
readonly class KeywordConfig
{
    public function __construct(
        public bool $enabled = false,
        public string $algorithm = 'rake',
        public int $maxKeywords = 10,
    );
}
```

**Fields:**

- `$enabled` (bool): Enable keyword extraction. Default: false
- `$algorithm` (string): Keyword extraction algorithm. Options: "rake". Default: "rake"
- `$maxKeywords` (int): Maximum number of keywords to extract. Default: 10

**Examples:**

```php title="keyword_config_examples.php"
<?php

use Kreuzberg\Config\KeywordConfig;

$config = new KeywordConfig(
    enabled: true,
    algorithm: 'rake',
    maxKeywords: 15
);
```

---

### ImagePreprocessingConfig

Image preprocessing configuration for OCR.

**Signature:**

```php title="PHP"
readonly class ImagePreprocessingConfig
{
    public function __construct(
        public int $targetDpi = 300,
        public bool $autoRotate = true,
        public bool $denoise = false,
    );
}
```

**Fields:**

- `$targetDpi` (int): Target DPI for image preprocessing. Default: 300
- `$autoRotate` (bool): Auto-rotate images based on orientation. Default: true
- `$denoise` (bool): Apply denoising filter. Default: false

---

## Results & Types

### ExtractionResult

Result object returned by all extraction functions.

**Signature:**

```php title="PHP"
readonly class ExtractionResult
{
    public function __construct(
        public ?array $annotations = null,
        public ?array $chunks = null,
        public string $content,
        public ?array $detectedLanguages = null,
        public ?DjotContent $djotContent = null,
        public ?DocumentStructure $document = null,
        public ?array $elements = null,
        public ?array $extractedKeywords = null,
        public ?array $images = null,
        public Metadata $metadata,
        public string $mimeType,
        public ?array $ocrElements = null,
        public ?array $pages = null,
        public ?array $processingWarnings = null,
        public ?float $qualityScore = null,
        public array $tables = [],
    );
}
```

**Fields:**

- `$annotations` (array<PdfAnnotation>|null): PDF annotations (links, highlights, notes)
- `$chunks` (array<Chunk>|null): Text chunks with embeddings and metadata
- `$content` (string): Extracted text content
- `$detectedLanguages` (array<string>|null): Array of detected language codes (ISO 639-1) if language detection is enabled
- `$djotContent` (DjotContent|null): Structured Djot content
- `$document` (DocumentStructure|null): Hierarchical document structure
- `$elements` (array<Element>|null): Semantic elements when `resultFormat='element_based'`
- `$extractedKeywords` (array<ExtractedKeyword>|null): Extracted keywords with scores
- `$images` (array<ExtractedImage>|null): Extracted images (with nested OCR results)
- `$metadata` (Metadata): Document metadata (format-specific fields)
- `$mimeType` (string): MIME type of the processed document
- `$ocrElements` (array<OcrElement>|null): OCR elements with positioning and confidence
- `$pages` (array<PageContent>|null): Per-page extracted content when page extraction is enabled
- `$processingWarnings` (array<ProcessingWarning>|null): Non-fatal processing warnings
- `$qualityScore` (float|null): Document extraction quality score (0.0 to 1.0)
- `$tables` (array<Table>): Array of extracted tables

**Examples:**

```php title="extraction_result_examples.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');

echo "Content: {$result->content}\n";
echo "MIME type: {$result->mimeType}\n";
echo "Page count: {$result->metadata->pageCount}\n";
echo "Tables: " . count($result->tables) . "\n";

if ($result->detectedLanguages !== null) {
    echo "Languages: " . implode(', ', $result->detectedLanguages) . "\n";
}
```

---

### Metadata

Document metadata with format-specific fields.

**Signature:**

```php title="PHP"
readonly class Metadata
{
    public function __construct(
        public ?string $language = null,
        public ?string $date = null,
        public ?string $subject = null,
        public ?string $formatType = null,
        public ?string $title = null,
        public ?array $authors = null,
        public ?array $keywords = null,
        public ?string $createdAt = null,
        public ?string $modifiedAt = null,
        public ?string $createdBy = null,
        public ?string $producer = null,
        public ?int $pageCount = null,
        public array $custom = [],
    );
}
```

**Common Fields:**

- `$authors` (array<string>|null): Document authors
- `$createdAt` (string|null): Creation date (ISO 8601)
- `$createdBy` (string|null): Creator/application name
- `$custom` (array<string, mixed>): Additional custom metadata from postprocessors
- `$date` (string|null): Document date (ISO 8601 format)
- `$formatType` (string|null): Format discriminator ("pdf", "excel", "email", etc.)
- `$keywords` (array<string>|null): Document keywords
- `$language` (string|null): Document language (ISO 639-1 code)
- `$modifiedAt` (string|null): Modification date (ISO 8601)
- `$pageCount` (int|null): Number of pages
- `$producer` (string|null): Producer/generator
- `$subject` (string|null): Document subject
- `$title` (string|null): Document title

**Examples:**

```php title="metadata_examples.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');
$metadata = $result->metadata;

echo "Title: " . ($metadata->title ?? 'N/A') . "\n";
echo "Authors: " . ($metadata->authors ? implode(', ', $metadata->authors) : 'N/A') . "\n";
echo "Pages: " . ($metadata->pageCount ?? 'N/A') . "\n";
echo "Created: " . ($metadata->createdAt ?? 'N/A') . "\n";
echo "Format: " . ($metadata->formatType ?? 'N/A') . "\n";
```

---

### Table

Extracted table structure.

**Signature:**

```php title="PHP"
readonly class Table
{
    public function __construct(
        public ?BoundingBox $boundingBox = null,
        public array $cells,
        public string $markdown,
        public int $pageNumber,
    );
}
```

**Fields:**

- `$boundingBox` (BoundingBox|null): Bounding box coordinates if available
- `$cells` (array<array<string>>): 2D array of table cells (rows x columns)
- `$markdown` (string): Table rendered as markdown
- `$pageNumber` (int): Page number where table was found

**Examples:**

```php title="table_examples.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('invoice.pdf');

foreach ($result->tables as $i => $table) {
    echo "Table " . ($i + 1) . " (Page {$table->pageNumber}):\n";
    echo $table->markdown . "\n\n";

    // Access cells directly
    foreach ($table->cells as $row) {
        foreach ($row as $cell) {
            echo "$cell | ";
        }
        echo "\n";
    }
}
```

---

### Chunk

Text chunk with optional embeddings and metadata.

**Signature:**

```php title="PHP"
readonly class Chunk
{
    public function __construct(
        public string $text,
        public ?array $embedding,
        public ChunkMetadata $metadata,
    );
}
```

**Fields:**

- `$text` (string): Chunk text content
- `$embedding` (array<float>|null): Embedding vector (if embedding generation is enabled)
- `$metadata` (ChunkMetadata): Chunk metadata (byte offsets, page numbers, etc.)

---

### ChunkMetadata

Metadata for a single text chunk.

**Signature:**

```php title="PHP"
readonly class ChunkMetadata
{
    public function __construct(
        public int $byteEnd,
        public int $byteStart,
        public int $charCount,
        public int $chunkIndex,
        public ?int $firstPage = null,
        public ?HeadingContext $headingContext = null,
        public ?int $lastPage = null,
        public ?int $tokenCount = null,
        public int $totalChunks,
    );
}
```

**Fields:**

- `$byteEnd` (int): UTF-8 byte offset in content (exclusive)
- `$byteStart` (int): UTF-8 byte offset in content (inclusive)
- `$charCount` (int): Number of characters in chunk
- `$chunkIndex` (int): Chunk index (0-based)
- `$firstPage` (int|null): First page this chunk appears on (1-indexed)
- `$headingContext` (?HeadingContext): Heading hierarchy when using Markdown chunker. Only populated when chunker_type is set to markdown.
- `$lastPage` (int|null): Last page this chunk appears on (1-indexed)
- `$tokenCount` (int|null): Estimated token count (if configured)
- `$totalChunks` (int): Total number of chunks

---

### ExtractedImage

Extracted image with optional OCR results.

**Signature:**

```php title="PHP"
readonly class ExtractedImage
{
    public function __construct(
        public ?int $bitsPerComponent = null,
        public ?BoundingBox $boundingBox = null,
        public ?string $colorspace = null,
        public string $data,
        public ?string $description = null,
        public string $format,
        public ?int $height = null,
        public int $imageIndex,
        public bool $isMask = false,
        public ?ExtractionResult $ocrResult = null,
        public ?int $pageNumber = null,
        public ?int $width = null,
    );
}
```

**Fields:**

- `$bitsPerComponent` (int|null): Bits per color component
- `$boundingBox` (BoundingBox|null): Bounding box coordinates if available
- `$colorspace` (string|null): Image colorspace
- `$data` (string): Image data (base64 encoded or raw bytes)
- `$description` (string|null): Image description/alt text
- `$format` (string): Image format (for example, "png", "jpeg")
- `$height` (int|null): Image height in pixels
- `$imageIndex` (int): Image index within document
- `$isMask` (bool): Whether image is a mask
- `$ocrResult` (ExtractionResult|null): OCR result if OCR was performed on the image
- `$pageNumber` (int|null): Page number where image was found
- `$width` (int|null): Image width in pixels

---

### PageContent

Per-page extracted content.

**Signature:**

```php title="PHP"
readonly class PageContent
{
    public function __construct(
        public int $pageNumber,
        public string $content,
        public array $tables,
        public array $images,
    );
}
```

**Fields:**

- `$pageNumber` (int): Page number (1-indexed)
- `$content` (string): Text content for this page
- `$tables` (array<Table>): Tables on this page
- `$images` (array<ExtractedImage>): Images on this page
- `$layoutRegions` (array<LayoutRegion>|null): Detected layout regions when layout detection is enabled. Each region has `class` (string), `confidence` (float, 0–1), `boundingBox`, and `areaFraction` (float, 0–1). `null` when layout detection is not configured.

**Examples:**

```php title="page_content_examples.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PageConfig;

$config = new ExtractionConfig(
    page: new PageConfig(extractPages: true)
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('document.pdf');

if ($result->pages !== null) {
    foreach ($result->pages as $page) {
        echo "Page {$page->pageNumber}:\n";
        echo "  Content: " . strlen($page->content) . " chars\n";
        echo "  Tables: " . count($page->tables) . "\n";
        echo "  Images: " . count($page->images) . "\n";
    }
}
```

---

## Embeddings

### Embed()

Generate embeddings for a list of texts.

**Signature:**

```php title="PHP"
public function embed(array $texts, ?EmbeddingConfig $config = null): array
```

**Parameters:**

- `$texts` (array\<string\>): List of strings to embed.
- `$config` (EmbeddingConfig|null): Embedding configuration. Defaults to the "balanced" preset.

**Returns:** `array<array<float>>` — one embedding vector per input text.

**Throws:** `KreuzbergException` if embedding generation fails or the `embeddings` feature is not enabled.

**Example:**

--8<-- "snippets/php/utils/standalone_embed.md"

---

## PDF Rendering

!!! Info "Added in v4.6.2"

### Render_pdf_page()

Render a single page of a PDF as a PNG image.

**Signature:**

```php title="PHP"
function render_pdf_page(string $filePath, int $pageIndex, int $dpi = 150): string
```

**Parameters:**

- `$filePath` (string): Path to the PDF file
- `$pageIndex` (int): Zero-based page index to render
- `$dpi` (int): Resolution for rendering (default 150)

**Returns:**

- `string`: PNG-encoded string for the requested page

**Throws:**

- `KreuzbergException`: If file cannot be read, rendered, or page index is out of bounds

**Example:**

```php title="render_single_page.php"
$png = \Kreuzberg\render_pdf_page('document.pdf', 0);
file_put_contents('first_page.png', $png);
```

---

## Exceptions

### KreuzbergException

Base exception class for all Kreuzberg errors.

**Signature:**

```php title="PHP"
class KreuzbergException extends Exception
{
    public function __construct(
        string $message = '',
        int $code = 0,
        ?Exception $previous = null
    );

    public static function validation(string $message): self;
    public static function parsing(string $message): self;
    public static function ocr(string $message): self;
    public static function missingDependency(string $message): self;
    public static function io(string $message): self;
    public static function plugin(string $message): self;
    public static function unsupportedFormat(string $message): self;
}
```

**Exception Types:**

- **Validation Error** (code 1): Invalid configuration or input
- **Parsing Error** (code 2): Document parsing failure
- **OCR Error** (code 3): OCR processing failure
- **Missing Dependency** (code 4): Required system dependency not found
- **I/O Error** (code 5): File system or network error
- **Plugin Error** (code 6): Plugin execution error
- **Unsupported Format** (code 7): Unsupported file format

**Examples:**

```php title="exception_handling.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Exceptions\KreuzbergException;

try {
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile('document.pdf');
    echo $result->content;
} catch (KreuzbergException $e) {
    echo "Error: {$e->getMessage()}\n";
    echo "Code: {$e->getCode()}\n";

    // Handle specific error types
    switch ($e->getCode()) {
        case 1:
            echo "Validation error\n";
            break;
        case 2:
            echo "Parsing error\n";
            break;
        case 3:
            echo "OCR error\n";
            break;
        case 4:
            echo "Missing dependency\n";
            break;
        default:
            echo "Unknown error\n";
    }
}
```

---

## Async Extraction

Kreuzberg PHP provides async extraction via a `DeferredResult` pattern. Async operations spawn work on a background Tokio thread pool and return immediately with a pollable `DeferredResult` object.

### DeferredResult

Result object returned by all async extraction functions. Supports non-blocking polling, blocking wait, and timeout-based waiting.

**Signature:**

```php title="PHP"
final class DeferredResult
{
    public function isReady(): bool;
    public function tryGetResult(): ?ExtractionResult;
    public function getResult(): ExtractionResult;
    public function wait(int $timeoutMs): ?ExtractionResult;
    public function getResults(): array;
    public function waitBatch(int $timeoutMs): ?array;
}
```

**Methods:**

- `isReady()`: Non-blocking check if the result is available
- `tryGetResult()`: Returns the result if ready, null if still pending (single extraction)
- `getResult()`: Blocks until the result is ready (single extraction)
- `wait($timeoutMs)`: Blocks with a timeout in milliseconds, returns null on timeout (single extraction)
- `getResults()`: Blocks until batch results are ready (batch extraction)
- `waitBatch($timeoutMs)`: Blocks with a timeout for batch results, returns null on timeout

---

### Kreuzberg::extractFileAsync()

Extract content from a file asynchronously.

**Signature:**

```php title="PHP"
public function extractFileAsync(
    string $filePath,
    ?string $mimeType = null,
    ?ExtractionConfig $config = null
): DeferredResult
```

**Parameters:**

- `$filePath` (string): Path to the file to extract
- `$mimeType` (string|null): Optional MIME type hint
- `$config` (ExtractionConfig|null): Extraction configuration

**Returns:**

- `DeferredResult`: A deferred result that can be polled or awaited

**Examples:**

```php title="async_extraction.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

// Start async extraction
$deferred = $kreuzberg->extractFileAsync('large_document.pdf');

// Do other work while extraction runs in background...
processOtherTasks();

// Check if ready (non-blocking)
if ($deferred->isReady()) {
    $result = $deferred->getResult();
    echo $result->content;
}

// Or block until ready
$result = $deferred->getResult();
echo $result->content;
```

---

### Kreuzberg::extractBytesAsync()

Extract content from bytes asynchronously.

**Signature:**

```php title="PHP"
public function extractBytesAsync(
    string $data,
    string $mimeType,
    ?ExtractionConfig $config = null
): DeferredResult
```

**Parameters:**

Same as [`Kreuzberg::extractBytes()`](#kreuzbergextractbytes).

**Returns:**

- `DeferredResult`: A deferred result that can be polled or awaited

---

### Kreuzberg::batchExtractFilesAsync()

Extract content from multiple files asynchronously.

**Signature:**

```php title="PHP"
public function batchExtractFilesAsync(
    array $paths,
    ?ExtractionConfig $config = null
): DeferredResult
```

**Parameters:**

Same as [`Kreuzberg::batchExtractFiles()`](#kreuzbergbatchextractfiles).

**Returns:**

- `DeferredResult`: A deferred result. Use `getResults()` or `waitBatch()` to retrieve.

**Examples:**

```php title="async_batch.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

$files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
$deferred = $kreuzberg->batchExtractFilesAsync($files);

// Wait with timeout (5 seconds)
$results = $deferred->waitBatch(5000);

if ($results !== null) {
    foreach ($results as $i => $result) {
        echo "{$files[$i]}: {$result->content}\n";
    }
} else {
    echo "Extraction timed out\n";
}
```

---

### Kreuzberg::batchExtractBytesAsync()

Extract content from multiple byte arrays asynchronously.

**Signature:**

```php title="PHP"
public function batchExtractBytesAsync(
    array $dataList,
    array $mimeTypes,
    ?ExtractionConfig $config = null
): DeferredResult
```

**Parameters:**

Same as [`Kreuzberg::batchExtractBytes()`](#kreuzbergbatchextractbytes).

**Returns:**

- `DeferredResult`: A deferred result. Use `getResults()` or `waitBatch()` to retrieve.

---

### Async Procedural Functions

All async methods are also available as procedural functions:

```php title="PHP"
function extract_file_async(
    string $filePath,
    ?string $mimeType = null,
    ?ExtractionConfig $config = null
): DeferredResult

function extract_bytes_async(
    string $data,
    string $mimeType,
    ?ExtractionConfig $config = null
): DeferredResult

function batch_extract_files_async(
    array $paths,
    ?ExtractionConfig $config = null
): DeferredResult

function batch_extract_bytes_async(
    array $dataList,
    array $mimeTypes,
    ?ExtractionConfig $config = null
): DeferredResult
```

**Examples:**

```php title="async_procedural.php"
<?php

use function Kreuzberg\extract_file_async;

$deferred = extract_file_async('document.pdf');

// Poll until ready
while (!$deferred->isReady()) {
    usleep(1000); // 1ms
}

$result = $deferred->getResult();
echo $result->content;
```

---

### Async Static Methods

Static convenience methods mirror the instance methods:

```php title="PHP"
Kreuzberg::extractFileAsyncStatic($filePath, $mimeType, $config): DeferredResult
Kreuzberg::extractBytesAsyncStatic($data, $mimeType, $config): DeferredResult
Kreuzberg::batchExtractFilesAsyncStatic($paths, $config): DeferredResult
Kreuzberg::batchExtractBytesAsyncStatic($dataList, $mimeTypes, $config): DeferredResult
```

---

### Framework Integration

#### Amp Integration

For projects using [Amp](https://amphp.org/) v3+, use `AmpBridge` to convert `DeferredResult` to Amp Futures:

```php title="amp_integration.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Async\AmpBridge;

$kreuzberg = new Kreuzberg();
$deferred = $kreuzberg->extractFileAsync('document.pdf');

// Convert to Amp Future
$future = AmpBridge::toFuture($deferred);
$result = $future->await();
echo $result->content;

// For batch operations
$batchDeferred = $kreuzberg->batchExtractFilesAsync(['doc1.pdf', 'doc2.pdf']);
$batchFuture = AmpBridge::toBatchFuture($batchDeferred);
$results = $batchFuture->await();
```

**Requires:** `amphp/amp ^3.0`

#### ReactPHP Integration

For projects using [ReactPHP](https://reactphp.org/), use `ReactBridge` to convert `DeferredResult` to ReactPHP Promises:

```php title="reactphp_integration.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Async\ReactBridge;

$kreuzberg = new Kreuzberg();
$deferred = $kreuzberg->extractFileAsync('document.pdf');

// Convert to ReactPHP Promise
$promise = ReactBridge::toPromise($deferred);
$promise->then(function ($result) {
    echo $result->content;
});
```

**Requires:** `react/promise ^3.0`, `react/event-loop ^1.0`

---

## Advanced Topics

### Error Handling

Robust error handling strategies for production applications.

**Basic Error Handling:**

```php title="basic_error_handling.php"
<?php

use function Kreuzberg\extract_file;
use Kreuzberg\Exceptions\KreuzbergException;

try {
    $result = extract_file('document.pdf');
    echo $result->content;
} catch (KreuzbergException $e) {
    error_log("Extraction failed: {$e->getMessage()}");
    // Handle error gracefully
}
```

**Retry with Exponential Backoff:**

```php title="retry_logic.php"
<?php

use function Kreuzberg\extract_file;
use Kreuzberg\Exceptions\KreuzbergException;

function extractWithRetry(
    string $filePath,
    int $maxRetries = 3,
    int $initialDelay = 1000
): ?string {
    $attempt = 0;
    $delay = $initialDelay;

    while ($attempt < $maxRetries) {
        try {
            $result = extract_file($filePath);
            return $result->content;
        } catch (KreuzbergException $e) {
            $attempt++;
            if ($attempt >= $maxRetries) {
                error_log("Max retries exceeded: {$e->getMessage()}");
                return null;
            }

            usleep($delay * 1000);
            $delay *= 2; // Exponential backoff
        }
    }

    return null;
}

$content = extractWithRetry('document.pdf');
```

**Fallback Strategies:**

```php title="fallback_strategies.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Exceptions\KreuzbergException;

function extractWithFallback(string $filePath): ?string
{
    // Try normal extraction
    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($filePath);
        if (!empty($result->content)) {
            return $result->content;
        }
    } catch (KreuzbergException $e) {
        // Fallback to OCR
        try {
            $config = new ExtractionConfig(
                ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
            );
            $kreuzberg = new Kreuzberg($config);
            $result = $kreuzberg->extractFile($filePath);
            return $result->content;
        } catch (KreuzbergException $e) {
            error_log("All extraction methods failed: {$e->getMessage()}");
        }
    }

    return null;
}
```

---

### Performance Tuning

Optimize extraction performance for high-throughput applications.

**Batch Processing:**

Always use batch APIs for processing multiple documents:

```php title="batch_performance.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();

// Good: Batch processing
$files = glob('documents/*.pdf');
$results = $kreuzberg->batchExtractFiles($files);

// Bad: Individual processing
foreach ($files as $file) {
    $result = $kreuzberg->extractFile($file);
}
```

**Memory Management:**

For large files, process in chunks or use streaming:

```php title="memory_management.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;

// Enable chunking for large documents
$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 1000,
        chunkOverlap: 100
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('large_document.pdf');

// Process chunks individually
if ($result->chunks !== null) {
    foreach ($result->chunks as $chunk) {
        // Process chunk
        processChunk($chunk->text);

        // Free memory
        unset($chunk);
    }
}
```

**Caching:**

Implement caching to avoid re-extracting unchanged documents:

```php title="caching_strategy.php"
<?php

use Kreuzberg\Kreuzberg;

function extractWithCache(string $filePath): string
{
    $cacheKey = 'extraction_' . md5($filePath . filemtime($filePath));

    // Check cache
    $cached = apcu_fetch($cacheKey);
    if ($cached !== false) {
        return $cached;
    }

    // Extract and cache
    $kreuzberg = new Kreuzberg();
    $result = $kreuzberg->extractFile($filePath);

    apcu_store($cacheKey, $result->content, 3600);

    return $result->content;
}
```

---

### Working with Images

Extract and process images from documents.

**Basic Image Extraction:**

```php title="image_extraction.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;

$config = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        minWidth: 200,
        minHeight: 200
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('presentation.pptx');

if ($result->images !== null) {
    foreach ($result->images as $i => $image) {
        // Save image to disk
        $filename = "image_{$i}.{$image->format}";
        file_put_contents($filename, base64_decode($image->data));

        echo "Saved {$filename}: {$image->width}x{$image->height}\n";
    }
}
```

**Image OCR:**

```php title="image_ocr.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ImageExtractionConfig;
use Kreuzberg\Config\OcrConfig;

$config = new ExtractionConfig(
    imageExtraction: new ImageExtractionConfig(
        extractImages: true,
        performOcr: true
    ),
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng'
    )
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('scanned_images.pdf');

if ($result->images !== null) {
    foreach ($result->images as $image) {
        if ($image->ocrResult !== null) {
            echo "Image on page {$image->pageNumber}:\n";
            echo $image->ocrResult->content . "\n\n";
        }
    }
}
```

---

### Working with Tables

Extract and process tables from documents.

**Basic Table Extraction:**

```php title="table_extraction.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('invoice.pdf');

foreach ($result->tables as $table) {
    echo "Table on page {$table->pageNumber}:\n";
    echo $table->markdown . "\n\n";
}
```

**Convert Tables to CSV:**

```php title="tables_to_csv.php"
<?php

use function Kreuzberg\extract_file;

$result = extract_file('spreadsheet.xlsx');

foreach ($result->tables as $i => $table) {
    $filename = "table_{$i}.csv";
    $fp = fopen($filename, 'w');

    foreach ($table->cells as $row) {
        fputcsv($fp, $row);
    }

    fclose($fp);
    echo "Saved {$filename}\n";
}
```

---

### Multi-Format Processing

Handle different file formats dynamically.

**Dynamic Configuration:**

```php title="multi_format.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\PdfConfig;
use Kreuzberg\Config\OcrConfig;
use function Kreuzberg\detect_mime_type_from_path;

function createConfigForMimeType(string $mimeType): ExtractionConfig
{
    return match (true) {
        str_contains($mimeType, 'pdf') => new ExtractionConfig(
            pdf: new PdfConfig(extractImages: true),
            extractTables: true
        ),
        str_contains($mimeType, 'image') => new ExtractionConfig(
            ocr: new OcrConfig(backend: 'tesseract', language: 'eng')
        ),
        str_contains($mimeType, 'spreadsheet') => new ExtractionConfig(
            extractTables: true
        ),
        default => new ExtractionConfig(),
    };
}

$filePath = 'document.pdf';
$mimeType = detect_mime_type_from_path($filePath);
$config = createConfigForMimeType($mimeType);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile($filePath);
```

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. The PHP binding exposes typed `LlmConfig` and `StructuredExtractionConfig` classes. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `StructuredExtractionConfig` to extract structured data from documents using an LLM:

```php title="structured_extraction.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\StructuredExtractionConfig;
use Kreuzberg\Config\LlmConfig;

$config = new ExtractionConfig(
    structuredExtraction: new StructuredExtractionConfig(
        schema: [
            'type' => 'object',
            'properties' => [
                'title' => ['type' => 'string'],
                'authors' => ['type' => 'array', 'items' => ['type' => 'string']],
                'date' => ['type' => 'string'],
            ],
            'required' => ['title', 'authors', 'date'],
            'additionalProperties' => false,
        ],
        llm: new LlmConfig(model: 'openai/gpt-4o-mini'),
        strict: true,
    ),
);

$kreuzberg = new Kreuzberg($config);
$result = $kreuzberg->extractFile('paper.pdf');

if ($result->structuredOutput !== null) {
    $data = json_decode($result->structuredOutput, true);
    echo $data['title'] . "\n";
}
```

### VLM OCR

Use a vision-language model as an OCR backend:

```php title="vlm_ocr.php"
<?php

use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\LlmConfig;

$config = new ExtractionConfig(
    forceOcr: true,
    ocr: new OcrConfig(
        backend: 'vlm',
        vlmConfig: new LlmConfig(model: 'openai/gpt-4o-mini'),
    ),
);
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## System Requirements

**PHP:** 8.1.0 or higher

**Native Extension:**

The Kreuzberg native extension must be compiled and installed for your PHP version and platform.

**Native Dependencies:**

- **Tesseract OCR** (for OCR support):
  - MacOS: `brew install tesseract`
  - Ubuntu: `apt-get install tesseract-ocr`
  - Windows: Download from [GitHub releases](https://github.com/tesseract-ocr/tesseract)

**Platforms:**

- Linux (x64, arm64)
- MacOS (x64, arm64)
- Windows (x64)

---

## Thread Safety

All Kreuzberg functions are thread-safe and can be called from multiple threads concurrently. However, for better performance with multiple files, use the batch APIs instead of threading.

**Example:**

```php title="thread_safety.php"
<?php

use Kreuzberg\Kreuzberg;

// Good: Use batch API
$kreuzberg = new Kreuzberg();
$files = ['doc1.pdf', 'doc2.pdf', 'doc3.pdf'];
$results = $kreuzberg->batchExtractFiles($files);

// Avoid: Manual threading
// PHP's threading support is limited, and batch APIs are more efficient
```

---

## Version Information

```php title="version.php"
<?php

use Kreuzberg\Kreuzberg;

echo "Kreuzberg version: " . Kreuzberg::version() . "\n";
```

---

## Troubleshooting

### Extension Not Loading

If you get an error that the Kreuzberg extension is not loaded:

**Check if extension is installed:**

```php title="check_extension.php"
<?php

if (!extension_loaded('kreuzberg')) {
    echo "Extension not loaded!\n";
    echo "Check your php.ini and ensure 'extension=kreuzberg.so' is present.\n";
    exit(1);
}

echo "Extension loaded successfully!\n";
```

**Verify php.ini configuration:**

```bash title="Check PHP Configuration"
php --ini
php -m | grep kreuzberg
```

**Check extension directory:**

```bash title="Check PHP Extension Directory"
php -i | grep extension_dir
```

### OCR Issues

**Tesseract not found:**

```php title="check_tesseract.php"
<?php

// Check if Tesseract is available
$output = shell_exec('tesseract --version 2>&1');

if ($output === null) {
    echo "Tesseract not found in PATH!\n";
    echo "Install: brew install tesseract (macOS) or apt install tesseract-ocr (Linux)\n";
} else {
    echo "Tesseract version:\n{$output}\n";
}

// Check for language data
$langDataPaths = [
    '/usr/local/share/tessdata/',  // macOS Homebrew
    '/usr/share/tesseract-ocr/4.00/tessdata/',  // Ubuntu/Debian
    '/usr/share/tessdata/',  // Alternative Linux path
];

foreach ($langDataPaths as $path) {
    if (is_dir($path)) {
        echo "\nLanguage data found in: {$path}\n";
        $files = glob($path . '*.traineddata');
        echo "Available languages: " . count($files) . "\n";
        foreach ($files as $file) {
            $lang = basename($file, '.traineddata');
            echo "- {$lang}\n";
        }
        break;
    }
}
```

**Poor OCR accuracy:**

Try these configurations:

```php title="improve_ocr.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;
use Kreuzberg\Config\TesseractConfig;
use Kreuzberg\Config\ImagePreprocessingConfig;

// Configuration 1: Increase DPI
$config1 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            targetDpi: 600  // Higher DPI for small text
        )
    )
);

// Configuration 2: Enable denoising
$config2 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        imagePreprocessing: new ImagePreprocessingConfig(
            denoise: true
        )
    )
);

// Configuration 3: Different PSM mode
$config3 = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'tesseract',
        language: 'eng',
        tesseractConfig: new TesseractConfig(
            psm: 11  // Sparse text mode
        )
    )
);

// Try each configuration
$kreuzberg = new Kreuzberg();
$result = $kreuzberg->extractFile('poor_quality_scan.pdf', config: $config1);
```

### Memory Issues

**Out of memory errors:**

```php title="memory_optimization.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\ChunkingConfig;

// Option 1: Increase memory limit
ini_set('memory_limit', '512M');

// Option 2: Use chunking
$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxChunkSize: 1000,
        chunkOverlap: 100
    )
);

// Option 3: Process in batches
$files = glob('documents/*.pdf');
$batchSize = 10;

foreach (array_chunk($files, $batchSize) as $batch) {
    $results = batch_extract_files($batch);

    // Process results...

    // Free memory
    unset($results);
    gc_collect_cycles();
}
```

### File Permission Errors

```php title="check_permissions.php"
<?php

$file = 'document.pdf';

// Check if file exists
if (!file_exists($file)) {
    echo "File not found: {$file}\n";
    exit(1);
}

// Check if file is readable
if (!is_readable($file)) {
    echo "File not readable: {$file}\n";
    echo "Current permissions: " . substr(sprintf('%o', fileperms($file)), -4) . "\n";
    echo "Run: chmod 644 {$file}\n";
    exit(1);
}

echo "File is accessible!\n";
```

---

## Migration Guide

### Upgrading from 3.x to 4.x

**Breaking Changes:**

1. **PHP Version Requirement**

   - Old: PHP 8.1+
   - New: PHP 8.2+

2. **Configuration Classes Now Readonly**

   ```php
   // Old (PHP 8.1)
   $config = new ExtractionConfig();
   $config->extractImages = true;  // Not possible anymore

   // New (PHP 8.2+)
   $config = new ExtractionConfig(extractImages: true);
   ```

3. **Namespace Changes**

   ```php
   // Old
   use Kreuzberg\Config\Config;

   // New
   use Kreuzberg\Config\ExtractionConfig;
   ```

4. **Method Signature Changes**

   ```php
   // Old
   $result = $kreuzberg->extract('file.pdf');

   // New
   $result = $kreuzberg->extractFile('file.pdf');
   ```

**Migration Steps:**

1. Update PHP to 8.2+:

   ```bash
   php -v  # Check current version
   ```

2. Update Composer dependency:

   ```bash
   composer require kreuzberg/kreuzberg:^4.0
   ```

3. Update extension:

   ```bash
   pie install kreuzberg/kreuzberg-ext:^4.0
   ```

4. Update code:

   ```php
   // Before
   use Kreuzberg\Config\Config;

   $config = new Config();
   $config->extractImages = true;
   $result = $kreuzberg->extract('file.pdf', $config);

   // After
   use Kreuzberg\Config\ExtractionConfig;

   $config = new ExtractionConfig(extractImages: true);
   $result = $kreuzberg->extractFile('file.pdf', config: $config);
   ```

---

## Best Practices

### 1. Error Handling

Always wrap extraction calls in try-catch blocks:

```php title="best_error_handling.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Exceptions\KreuzbergException;

function extractDocument(string $path): ?string
{
    try {
        $kreuzberg = new Kreuzberg();
        $result = $kreuzberg->extractFile($path);
        return $result->content;
    } catch (KreuzbergException $e) {
        // Log the error
        error_log("Extraction failed for {$path}: " . $e->getMessage());

        // Return null or throw
        return null;
    }
}
```

### 2. Configuration Reuse

Reuse configuration objects for better performance:

```php title="reuse_config.php"
<?php

use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;
use Kreuzberg\Config\OcrConfig;

// Good: Create config once
$config = new ExtractionConfig(
    ocr: new OcrConfig(backend: 'tesseract', language: 'eng'),
    extractTables: true
);

$kreuzberg = new Kreuzberg($config);

// Reuse for multiple files
$result1 = $kreuzberg->extractFile('doc1.pdf');
$result2 = $kreuzberg->extractFile('doc2.pdf');
$result3 = $kreuzberg->extractFile('doc3.pdf');

// Bad: Creating new config for each file
foreach ($files as $file) {
    $config = new ExtractionConfig();  // Wasteful — recreating each iteration
    $result = (new Kreuzberg($config))->extractFile($file);
}
```

### 3. Batch Processing

Use batch APIs for multiple files:

```php title="best_batch.php"
<?php

use Kreuzberg\Kreuzberg;

$kreuzberg = new Kreuzberg();
$files = glob('documents/*.pdf');

// Good: Batch processing
$results = $kreuzberg->batchExtractFiles($files);

// Bad: Individual processing in a loop
foreach ($files as $file) {
    $result = $kreuzberg->extractFile($file);  // Inefficient
}
```

### 4. Resource Management

Free resources when processing large datasets:

```php title="resource_management.php"
<?php

use function Kreuzberg\batch_extract_files;

$files = glob('documents/*.pdf');

// Process in chunks to manage memory
foreach (array_chunk($files, 50) as $batch) {
    $results = batch_extract_files($batch);

    // Process results...
    foreach ($results as $result) {
        // Do something with result
        processResult($result);
    }

    // Free memory
    unset($results);
    gc_collect_cycles();
}
```

### 5. Type Validation

Validate inputs before processing:

```php title="validation.php"
<?php

use Kreuzberg\Kreuzberg;

function extractSafely(string $path): ?ExtractionResult
{
    // Validate file exists
    if (!file_exists($path)) {
        throw new InvalidArgumentException("File not found: {$path}");
    }

    // Validate file is readable
    if (!is_readable($path)) {
        throw new RuntimeException("File not readable: {$path}");
    }

    // Validate file size (e.g., max 100MB)
    $maxSize = 100 * 1024 * 1024;
    if (filesize($path) > $maxSize) {
        throw new RuntimeException("File too large: {$path}");
    }

    // Extract
    $kreuzberg = new Kreuzberg();
    return $kreuzberg->extractFile($path);
}
```

---

## Framework Integration

### Laravel

**Service Provider:**

```php title="app/Providers/KreuzbergServiceProvider.php"
<?php

namespace App\Providers;

use Illuminate\Support\ServiceProvider;
use Kreuzberg\Kreuzberg;
use Kreuzberg\Config\ExtractionConfig;

class KreuzbergServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->singleton(Kreuzberg::class, function ($app) {
            $config = new ExtractionConfig(
                extractTables: true,
                extractImages: config('kreuzberg.extract_images', false),
            );

            return new Kreuzberg($config);
        });
    }
}
```

**Configuration:**

```php title="config/kreuzberg.php"
<?php

return [
    'extract_images' => env('KREUZBERG_EXTRACT_IMAGES', false),
    'extract_tables' => env('KREUZBERG_EXTRACT_TABLES', true),
    'ocr_language' => env('KREUZBERG_OCR_LANGUAGE', 'eng'),
];
```

**Usage:**

```php title="app/Http/Controllers/DocumentController.php"
<?php

namespace App\Http\Controllers;

use Illuminate\Http\Request;
use Kreuzberg\Kreuzberg;

class DocumentController extends Controller
{
    public function extract(Request $request, Kreuzberg $kreuzberg)
    {
        $request->validate([
            'document' => 'required|file|mimes:pdf,docx,xlsx|max:10240',
        ]);

        $file = $request->file('document');
        $result = $kreuzberg->extractFile($file->getPathname());

        return response()->json([
            'content' => $result->content,
            'metadata' => $result->metadata,
            'tables' => $result->tables,
        ]);
    }
}
```

### Symfony

**Service Configuration:**

```yaml title="config/services.yaml"
services:
    Kreuzberg\Kreuzberg:
        arguments:
            $defaultConfig: '@Kreuzberg\Config\ExtractionConfig'

    Kreuzberg\Config\ExtractionConfig:
        factory: ['App\Factory\KreuzbergConfigFactory', 'create']
```

**Factory:**

```php title="src/Factory/KreuzbergConfigFactory.php"
<?php

namespace App\Factory;

use Kreuzberg\Config\ExtractionConfig;

class KreuzbergConfigFactory
{
    public static function create(): ExtractionConfig
    {
        return new ExtractionConfig(
            extractTables: true,
            extractImages: false,
        );
    }
}
```

**Usage:**

```php title="src/Controller/DocumentController.php"
<?php

namespace App\Controller;

use Kreuzberg\Kreuzberg;
use Symfony\Bundle\FrameworkBundle\Controller\AbstractController;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\JsonResponse;

class DocumentController extends AbstractController
{
    public function extract(Request $request, Kreuzberg $kreuzberg): JsonResponse
    {
        $file = $request->files->get('document');
        $result = $kreuzberg->extractFile($file->getPathname());

        return $this->json([
            'content' => $result->content,
            'metadata' => $result->metadata,
        ]);
    }
}
```
