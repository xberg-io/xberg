# TypeScript API Reference <span class="version-badge">v4.0.0</span>

Complete reference for the Kreuzberg **native TypeScript/Node.js** API (`@kreuzberg/node`).

!!! Info "WASM Alternative"

    This reference covers **native bindings** (`@kreuzberg/node`) for Node.js, Bun, and Deno.

    For browser and edge environments, see the [WASM API Reference](api-wasm.md) (`@kreuzberg/wasm`).

## Installation

```bash title="Terminal"
npm install @kreuzberg/node
```

**Or with other package managers:**

```bash title="Terminal"
# Yarn
yarn add @kreuzberg/node

# pnpm
pnpm add @kreuzberg/node
```

## Core Functions

### BatchExtractBytes()

Extract content from multiple byte arrays in parallel (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function batchExtractBytes(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

**Parameters:**

Same as [`batchExtractBytesSync()`](#batchextractbytessync).

**Returns:**

- `Promise<ExtractionResult[]>`: Promise resolving to array of extraction results

---

### BatchExtractBytesSync()

Extract content from multiple byte arrays in parallel (synchronous).

**Signature:**

```typescript title="TypeScript"
function batchExtractBytesSync(
  dataList: Uint8Array[],
  mimeTypes: string[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

**Parameters:**

- `dataList` (Uint8Array[]): Array of file contents as Uint8Array
- `mimeTypes` (string[]): Array of MIME types (one per data item, same length as dataList)
- `config` (ExtractionConfig | null): Extraction configuration applied to all items

**Returns:**

- `ExtractionResult[]`: Array of extraction results (one per data item)

---

### BatchExtractFiles()

Extract content from multiple files in parallel (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function batchExtractFiles(
  paths: string[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

**Parameters:**

Same as [`batchExtractFilesSync()`](#batchextractfilessync).

**Returns:**

- `Promise<ExtractionResult[]>`: Promise resolving to array of extraction results

**Examples:**

```typescript title="async_batch_extraction.ts"
import { batchExtractFiles } from '@kreuzberg/node';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = await batchExtractFiles(files);

for (const result of results) {
  console.log(result.content);
}
```

---

### BatchExtractFilesSync()

Extract content from multiple files in parallel (synchronous).

**Signature:**

```typescript title="TypeScript"
function batchExtractFilesSync(
  paths: string[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

**Parameters:**

- `paths` (string[]): Array of file paths to extract
- `config` (ExtractionConfig | null): Extraction configuration applied to all files

**Returns:**

- `ExtractionResult[]`: Array of extraction results (one per file)

**Examples:**

```typescript title="parallel_batch_extraction.ts"
import { batchExtractFilesSync } from '@kreuzberg/node';

const paths = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = batchExtractFilesSync(paths);

results.forEach((result, i) => {
  console.log(`${paths[i]}: ${result.content.length} characters`);
});
```

---

### BatchExtractFilesWithConfigs() <span class="version-badge">v4.5.0</span>

Extract content from multiple files in parallel, with per-file configuration overrides (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function batchExtractFilesWithConfigs(
  paths: string[],
  fileConfigs: (FileExtractionConfig | null)[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

**Parameters:**

- `paths` (string[]): Array of file paths
- `fileConfigs` ((FileExtractionConfig | null)[]): Array of per-file configs (null = use batch defaults). Must match paths length.
- `config` (ExtractionConfig | null): Batch-level extraction configuration

**Returns:**

- `Promise<ExtractionResult[]>`: Promise resolving to array of extraction results

---

### BatchExtractFilesWithConfigsSync() <span class="version-badge">v4.5.0</span>

Synchronous variant of [`batchExtractFilesWithConfigs()`](#batchextractfileswithconfigs).

**Signature:**

```typescript title="TypeScript"
function batchExtractFilesWithConfigsSync(
  paths: string[],
  fileConfigs: (FileExtractionConfig | null)[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

**Example:**

```typescript title="per_file_config.ts"
import { batchExtractFilesWithConfigsSync } from '@kreuzberg/node';

const results = batchExtractFilesWithConfigsSync(
  ['report.pdf', 'scanned.pdf', 'page.html'],
  [
    null,  // use batch defaults
    { forceOcr: true, ocr: { backend: 'tesseract', language: 'deu' } },
    { outputFormat: 'markdown' },
  ],
);
```

---

### BatchExtractBytesWithConfigs() <span class="version-badge">v4.5.0</span>

Extract content from multiple byte arrays in parallel, with per-file configuration overrides (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function batchExtractBytesWithConfigs(
  dataList: Uint8Array[],
  mimeTypes: string[],
  fileConfigs: (FileExtractionConfig | null)[],
  config: ExtractionConfig | null = null
): Promise<ExtractionResult[]>
```

---

### BatchExtractBytesWithConfigsSync() <span class="version-badge">v4.5.0</span>

Synchronous variant of [`batchExtractBytesWithConfigs()`](#batchextractbyteswithconfigs).

**Signature:**

```typescript title="TypeScript"
function batchExtractBytesWithConfigsSync(
  dataList: Uint8Array[],
  mimeTypes: string[],
  fileConfigs: (FileExtractionConfig | null)[],
  config: ExtractionConfig | null = null
): ExtractionResult[]
```

---

### ExtractBytes()

Extract content from bytes (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function extractBytes(
  data: Uint8Array,
  mimeType: string,
  config: ExtractionConfig | null = null
): Promise<ExtractionResult>
```

**Parameters:**

Same as [`extractBytesSync()`](#extractbytessync).

**Returns:**

- `Promise<ExtractionResult>`: Promise resolving to extraction result

---

### ExtractBytesSync()

Extract content from bytes (synchronous).

**Signature:**

```typescript title="TypeScript"
function extractBytesSync(
  data: Uint8Array,
  mimeType: string,
  config: ExtractionConfig | null = null
): ExtractionResult
```

**Parameters:**

- `data` (Uint8Array): File content as Uint8Array
- `mimeType` (string): MIME type of the data (required for format detection)
- `config` (ExtractionConfig | null): Extraction configuration. Uses defaults if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Examples:**

```typescript title="extract_from_buffer.ts"
import { extractBytesSync } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');
const result = extractBytesSync(data, 'application/pdf');
console.log(result.content);
```

---

### ExtractFile()

Extract content from a file (asynchronous).

**Signature:**

```typescript title="TypeScript"
async function extractFile(
  filePath: string,
  mimeType: string | null = null,
  config: ExtractionConfig | null = null
): Promise<ExtractionResult>
```

**Parameters:**

Same as [`extractFileSync()`](#extractfilesync).

**Returns:**

- `Promise<ExtractionResult>`: Promise resolving to extraction result

**Examples:**

```typescript title="async_extraction.ts"
import { extractFile } from '@kreuzberg/node';

async function main() {
  const result = await extractFile('document.pdf');
  console.log(result.content);
}

main();
```

---

### ExtractFileSync()

Extract content from a file (synchronous).

**Signature:**

```typescript title="TypeScript"
function extractFileSync(
  filePath: string,
  mimeType: string | null = null,
  config: ExtractionConfig | null = null
): ExtractionResult
```

**Parameters:**

- `filePath` (string): Path to the file to extract
- `mimeType` (string | null): Optional MIME type hint. If null, MIME type is auto-detected from file extension and content
- `config` (ExtractionConfig | null): Extraction configuration. Uses defaults if null

**Returns:**

- `ExtractionResult`: Extraction result containing content, metadata, and tables

**Throws:**

- `Error`: Base error for all extraction failures (validation, parsing, OCR, etc.)

**Example - Basic usage:**

```typescript title="basic_pdf_extraction.ts"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');
console.log(result.content);
console.log(`Pages: ${result.metadata.pageCount}`);
```

**Example - With OCR:**

```typescript title="ocr_extraction.ts"
import { extractFileSync } from '@kreuzberg/node';

const config = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  }
};
const result = extractFileSync('scanned.pdf', null, config);
```

**Example - With explicit MIME type:**

```typescript title="mime_type_override.ts"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', 'application/pdf');
```

---

## Configuration

### ExtractionConfig

Main configuration interface for extraction operations.

**Type Definition:**

```typescript title="TypeScript"
interface ExtractionConfig {
  chunking?: ChunkingConfig | null;
  concurrency?: ConcurrencyConfig | null; // <span class="version-badge">v4.5.0</span>
  enableQualityProcessing?: boolean;
  forceOcr?: boolean;
  htmlOptions?: HtmlConversionOptions | null;
  imageExtraction?: ImageExtractionConfig | null;
  includeDocumentStructure?: boolean;
  keywords?: KeywordConfig | null;
  languageDetection?: LanguageDetectionConfig | null;
  maxConcurrentExtractions?: number;
  ocr?: OcrConfig | null;
  outputFormat?: "plain" | "markdown" | "djot" | "html";
  pages?: PageConfig | null;
  pdfOptions?: PdfConfig | null;
  postProcessor?: PostProcessorConfig | null;
  resultFormat?: "unified" | "element_based";
  securityLimits?: Record<string, number> | null;
  tokenReduction?: TokenReductionConfig | null;
  useCache?: boolean;
}
```

**Fields:**

- `useCache` (boolean): Enable caching for identical inputs. Default: true
- `enableQualityProcessing` (boolean): Enable built-in filters to improve extraction reliability. Default: false
- `ocr` (OcrConfig | null): OCR configuration. Default: null (no OCR)
- `forceOcr` (boolean): Force OCR even for text-based PDFs. Default: false
- `pdfOptions` (PdfConfig | null): PDF-specific configuration. Default: null
- `chunking` (ChunkingConfig | null): Text chunking configuration. Default: null
- `concurrency` (ConcurrencyConfig | null): Concurrency configuration. Default: null
- `imageExtraction` (ImageExtractionConfig | null): Image extraction from documents. Default: null
- `languageDetection` (LanguageDetectionConfig | null): Language detection configuration. Default: null
- `tokenReduction` (TokenReductionConfig | null): Token reduction configuration. Default: null
- `postProcessor` (PostProcessorConfig | null): Post-processing configuration. Default: null
- `htmlOptions` (HtmlConversionOptions | null): HTML to Markdown conversion options. Default: null
- `keywords` (KeywordConfig | null): Keyword extraction configuration. Default: null
- `pages` (PageConfig | null): Page tracking and continuous extraction options. Default: null
- `securityLimits` (Record<string, number> | null): Safety limits (archive recursion, xml depth, etc.)
- `maxConcurrentExtractions` (number): Maximum parallel tasks for batching. Default: 4
- `outputFormat` ("plain" | "markdown" | "djot" | "html"): Generated text format. Default: "plain"
- `resultFormat` ("unified" | "element_based"): Shape of extraction results. Default: "unified"
- `includeDocumentStructure` (boolean): Construct hierarchical document tree. Default: false

**Example:**

```typescript title="extraction_config.ts"
import { extractFileSync, ExtractionConfig } from '@kreuzberg/node';

const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng'
  },
  forceOcr: false,
  pdfOptions: {
    passwords: ['password1', 'password2'],
    extractImages: true
  }
};

const result = extractFileSync('document.pdf', null, config);
```

---

### ExtractionConfig Static Methods

The `ExtractionConfig` object provides static methods for loading configuration from files and discovering configuration files in the filesystem.

#### ExtractionConfig.fromFile()

Load extraction configuration from a file.

**Signature:**

```typescript title="TypeScript"
static fromFile(filePath: string): ExtractionConfig
```

**Parameters:**

- `filePath` (string): Path to the configuration file (absolute or relative). Supports `.toml`, `.yaml`, `.json` formats

**Returns:**

- `ExtractionConfig`: Configuration object loaded from the file

**Throws:**

- `Error`: If file does not exist, is not readable, or contains invalid configuration

**Example:**

```typescript title="load_config_from_file.ts"
import { ExtractionConfig, extractFileSync } from '@kreuzberg/node';

const config = ExtractionConfig.fromFile('kreuzberg.toml');
const result = extractFileSync('document.pdf', null, config);
console.log(result.content);
```

---

#### ExtractionConfig.discover()

Discover and load configuration from current or parent directories.

**Signature:**

```typescript title="TypeScript"
static discover(): ExtractionConfig | null
```

**Returns:**

- `ExtractionConfig | null`: Configuration object if found, or `null` if no configuration file exists

**Description:**

Searches for a `kreuzberg.toml`, `kreuzberg.yaml`, or `kreuzberg.json` file starting from the current working directory and traversing up the directory tree. Returns the first configuration file found.

**Example:**

```typescript title="discover_config.ts"
import { ExtractionConfig, extractFile } from '@kreuzberg/node';

const config = ExtractionConfig.discover();
if (config) {
  const result = await extractFile('document.pdf', null, config);
  console.log('Extracted using discovered config');
  console.log(result.content);
} else {
  const result = await extractFile('document.pdf', null, null);
  console.log('No config file found, using defaults');
  console.log(result.content);
}
```

---

### FileExtractionConfig <span class="version-badge">v4.5.0</span>

Per-file extraction configuration overrides for batch operations. All fields are optional — `undefined` or omitted means "use the batch-level default."

**Type Definition:**

```typescript title="TypeScript"
interface FileExtractionConfig {
  enableQualityProcessing?: boolean;
  ocr?: OcrConfig | null;
  forceOcr?: boolean;
  chunking?: ChunkingConfig | null;
  imageExtraction?: ImageExtractionConfig | null;
  pdfOptions?: PdfConfig | null;
  tokenReduction?: TokenReductionConfig | null;
  languageDetection?: LanguageDetectionConfig | null;
  pages?: PageConfig | null;
  keywords?: KeywordConfig | null;
  postProcessor?: PostProcessorConfig | null;
  htmlOptions?: HtmlConversionOptions | null;
  resultFormat?: "unified" | "element_based";
  outputFormat?: "plain" | "markdown" | "djot" | "html";
  includeDocumentStructure?: boolean;
}
```

**Example:**

```typescript title="file_extraction_config.ts"
import type { FileExtractionConfig } from '@kreuzberg/node';

const perFileConfig: FileExtractionConfig = {
  forceOcr: true,
  ocr: { backend: 'tesseract', language: 'deu' },
};
```

See [Configuration Reference](configuration.md#fileextractionconfig) for full details on merge semantics and excluded batch-level fields.

---

### OcrConfig

OCR processing configuration.

**Type Definition:**

```typescript title="TypeScript"
interface OcrConfig {
  backend: string;
  language: string;
  tesseractConfig?: TesseractConfig | null;
}
```

**Fields:**

- `backend` (string): OCR backend to use. Options: "tesseract", "paddle-ocr". Default: "tesseract"
- `language` (string): Language code for OCR (ISO 639-3). Default: "eng"
- `tesseractConfig` (TesseractConfig | null): Tesseract-specific configuration. Default: null
- `modelTier` (string | null): <span class="version-badge">v4.5.0</span> PaddleOCR model tier: "mobile" (lightweight, ~21MB total, fast) or "server" (high accuracy, ~172MB, best with GPU). Default: "mobile"
- `padding` (number | null): <span class="version-badge">v4.5.0</span> Padding in pixels (0-100) added around the image before PaddleOCR detection. Default: 10

**Example:**

```typescript title="ocr_config.ts"
const ocrConfig: OcrConfig = {
  backend: 'tesseract',
  language: 'eng'
};
```

---

### TesseractConfig

Tesseract OCR backend configuration.

**Type Definition:**

```typescript title="TypeScript"
interface TesseractConfig {
  psm?: number;
  oem?: number;
  enableTableDetection?: boolean;
  tesseditCharWhitelist?: string | null;
  tesseditCharBlacklist?: string | null;
}
```

**Fields:**

- `psm` (number): Page segmentation mode (0-13). Default: 3 (auto)
- `oem` (number): OCR engine mode (0-3). Default: 3 (LSTM only)
- `enableTableDetection` (boolean): Enable table detection and extraction. Default: false
- `tesseditCharWhitelist` (string | null): Character whitelist (for example, "0123456789" for digits only). Default: null
- `tesseditCharBlacklist` (string | null): Character blacklist. Default: null

**Example:**

```typescript title="tesseract_config.ts"
const config: ExtractionConfig = {
  ocr: {
    backend: 'tesseract',
    language: 'eng',
    tesseractConfig: {
      psm: 6,
      enableTableDetection: true,
      tesseditCharWhitelist: '0123456789'
    }
  }
};
```

---

### PdfConfig

PDF-specific configuration.

**Type Definition:**

```typescript title="TypeScript"
interface PdfConfig {
  allowSingleColumnTables?: boolean;
  passwords?: string[] | null;
  extractImages?: boolean;
  imageDpi?: number;
}
```

**Fields:**

- `allowSingleColumnTables` (boolean) <span class="version-badge">v4.5.0</span>: Allow extraction of single-column tables. Default: false
- `passwords` (string[] | null): List of passwords to try for encrypted PDFs. Default: null
- `extractImages` (boolean): Extract images from PDF. Default: false
- `imageDpi` (number): DPI for image extraction. Default: 300

**Example:**

```typescript title="pdf_config.ts"
const pdfConfig: PdfConfig = {
  allowSingleColumnTables: false,
  passwords: ['password1', 'password2'],
  extractImages: true,
  imageDpi: 300
};
```

---

### ConcurrencyConfig <span class="version-badge">v4.5.0</span>

Concurrency configuration for controlling parallel extraction.

**Type Definition:**

```typescript title="TypeScript"
interface ConcurrencyConfig {
  maxThreads?: number | null;
}
```

**Fields:**

- `maxThreads` (number | null): Maximum number of concurrent threads. Default: null (use system default)

**Example:**

```typescript title="concurrency_config.ts"
const config: ExtractionConfig = {
  concurrency: {
    maxThreads: 4
  }
};
```

### LayoutDetectionConfig <span class="version-badge">v4.5.0</span>

Configuration for ONNX-based document layout analysis.

**Type Definition:**

<!-- skip -->

```typescript title="TypeScript"
interface LayoutDetectionConfig {
  preset?: string;              // "fast" or "accurate" (default)
  confidenceThreshold?: number; // minimum detection confidence
  applyHeuristics?: boolean;    // post-processing refinement (default: true)
}
```

**Fields:**

- `preset` (string): Model preset — `"accurate"` (RT-DETR v2, 17 classes) or `"fast"` (YOLO DocLayNet, 11 classes). Default: `"accurate"`
- `confidenceThreshold` (number | null): Minimum confidence for layout detections. Default: null (model default)
- `applyHeuristics` (boolean): Apply post-processing heuristics to refine results. Default: true

**Example:**

<!-- skip -->

```typescript title="layout_config.ts"
const config: ExtractionConfig = {
  layout: {
    preset: "accurate"
  },
  acceleration: {
    provider: "cuda"
  }
};
```

---

### ChunkingConfig

Text chunking configuration for splitting long documents.

**Type Definition:**

```typescript title="TypeScript"
interface ChunkingConfig {
  maxChars?: number;
  maxOverlap?: number;
  embedding?: EmbeddingConfig | null;
  preset?: string | null;
  chunkerType?: string | null;
  sizingType?: "characters" | "tokenizer" | null;
  sizingModel?: string | null;
  sizingCacheDir?: string | null;
  prependHeadingContext?: boolean | null;
}
```

**Fields:**

- `maxChars` (number): Maximum characters per chunk. Default: 1000
- `maxOverlap` (number): Overlap between chunks in characters. Default: 200
- `embedding` (EmbeddingConfig | null): Embedding configuration for generating embeddings. Default: null
- `preset` (string | null): Chunking preset to use. Default: null
- `sizingType` ("characters" | "tokenizer" | null): How chunk size is measured. Use `"tokenizer"` to measure by token count using a HuggingFace tokenizer. Default: null (characters)
- `sizingModel` (string | null): HuggingFace model ID for tokenizer-based sizing (for example `"bert-base-uncased"`). Required when `sizingType` is `"tokenizer"`. Default: null
- `sizingCacheDir` (string | null): Optional directory to cache downloaded tokenizer files. Default: null
- `chunkerType` (string | null): Type of chunker to use. Options: `"text"` (default), `"markdown"`, `"yaml"`. Default: null (text)
- `prependHeadingContext` (boolean | null): When true, prepends heading hierarchy path to each chunk's content. Most useful with `chunkerType: "markdown"`. Default: null (false)

---

### LanguageDetectionConfig

Language detection configuration.

**Type Definition:**

```typescript title="TypeScript"
interface LanguageDetectionConfig {
  enabled?: boolean;
  confidenceThreshold?: number;
}
```

**Fields:**

- `enabled` (boolean): Enable language detection. Default: true
- `confidenceThreshold` (number): Minimum confidence threshold (0.0-1.0). Default: 0.5

---

### ImageExtractionConfig

Image extraction configuration.

**Type Definition:**

```typescript title="TypeScript"
interface ImageExtractionConfig {
  enabled?: boolean;
  minWidth?: number;
  minHeight?: number;
}
```

**Fields:**

- `enabled` (boolean): Enable image extraction from documents. Default: false
- `minWidth` (number): Minimum image width in pixels. Default: 100
- `minHeight` (number): Minimum image height in pixels. Default: 100

---

### TokenReductionConfig

Token reduction configuration for compressing extracted text.

**Type Definition:**

```typescript title="TypeScript"
interface TokenReductionConfig {
  enabled?: boolean;
  strategy?: string;
}
```

**Fields:**

- `enabled` (boolean): Enable token reduction. Default: false
- `strategy` (string): Reduction strategy. Options: "whitespace", "stemming". Default: "whitespace"

---

### PostProcessorConfig

Post-processing configuration.

**Type Definition:**

```typescript title="TypeScript"
interface PostProcessorConfig {
  enabled?: boolean;
  processors?: string[] | null;
}
```

**Fields:**

- `enabled` (boolean): Enable post-processing. Default: true
- `processors` (string[] | null): List of processor names to enable. Default: null (all registered processors)

---

## Results & Types

### ExtractionResult

Result object returned by all extraction functions.

**Type Definition:**

```typescript title="TypeScript"
interface ExtractionResult {
  annotations?: PdfAnnotation[];
  chunks?: Chunk[];
  content: string;
  detectedLanguages: string[] | null;
  djotContent?: DjotContent | null;
  document?: DocumentStructure | null;
  elements?: Element[];
  extractedKeywords?: ExtractedKeyword[];
  images?: ExtractedImage[];
  metadata: Metadata;
  metadataJson: string;
  mimeType: string;
  ocrElements?: OcrElement[];
  outputFormat?: string | null;
  pages?: PageContent[];
  processingWarnings: ProcessingWarning[];
  qualityScore?: number;
  resultFormat?: string | null;
  tables: Table[];
}
```

**Fields:**

- `annotations` (PdfAnnotation[] | undefined): Extracted PDF annotations and highlights
- `chunks` (Chunk[] | undefined): Text chunks if chunking is enabled
- `content` (string): Extracted text content
- `detectedLanguages` (string[] | null): Array of detected language codes (ISO 639-1) if language detection is enabled
- `djotContent` (DjotContent | null): Rich structural markup
- `document` (DocumentStructure | null): Hierarchical document structure
- `elements` (Element[] | undefined): Semantic elements (headings, paragraphs, etc.)
- `extractedKeywords` (ExtractedKeyword[] | undefined): Extracted keywords (RAKE/YAKE)
- `images` (ExtractedImage[] | undefined): Extracted images if enabled
- `metadata` (Metadata): Document metadata (format-specific fields)
- `mimeType` (string): MIME type of the processed document
- `ocrElements` (OcrElement[] | undefined): Granular OCR text blocks with bounding boxes
- `pages` (PageContent[] | undefined): Per-page extracted content when page extraction is enabled via `PageConfig.extractPages = true`
- `processingWarnings` (ProcessingWarning[]): Non-fatal warnings encountered during extraction
- `qualityScore` (number | undefined): Document quality estimation score
- `tables` (Table[]): Array of extracted tables

**Example:**

```typescript title="inspect_extraction_result.ts"
const result = extractFileSync('document.pdf');

console.log(`Content: ${result.content}`);
console.log(`MIME type: ${result.mimeType}`);
console.log(`Page count: ${result.metadata.page_count}`);
console.log(`Tables: ${result.tables.length}`);

if (result.detectedLanguages) {
  console.log(`Languages: ${result.detectedLanguages.join(', ')}`);
}
```

#### Pages

**Type**: `PageContent[] | undefined`

Per-page extracted content when page extraction is enabled via `PageConfig.extractPages = true`.

Each page contains:

- Page number (1-indexed)
- Text content for that page
- Tables on that page
- Images on that page
- Layout regions when layout detection is enabled, each with `class` (string), `confidence` (number, 0–1), `boundingBox`, and `areaFraction` (number, 0–1)

**Example:**

```typescript title="page_extraction.ts"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', null, {
  pages: {
    extractPages: true
  }
});

if (result.pages) {
  for (const page of result.pages) {
    console.log(`Page ${page.pageNumber}:`);
    console.log(`  Content: ${page.content.length} chars`);
    console.log(`  Tables: ${page.tables.length}`);
    console.log(`  Images: ${page.images.length}`);
  }
}
```

---

### Accessing Per-Page Content

When page extraction is enabled, access individual pages and iterate over them:

```typescript title="iterate_pages.ts"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', null, {
  pages: {
    extractPages: true,
    insertPageMarkers: true,
    markerFormat: '\n\n--- Page {page_num} ---\n\n'
  }
});

// Access combined content with page markers
console.log('Combined content with markers:');
console.log(result.content.substring(0, 500));
console.log();

// Access per-page content
if (result.pages) {
  for (const page of result.pages) {
    console.log(`Page ${page.pageNumber}:`);
    console.log(`  ${page.content.substring(0, 100)}...`);
    if (page.tables.length > 0) {
      console.log(`  Found ${page.tables.length} table(s)`);
    }
    if (page.images.length > 0) {
      console.log(`  Found ${page.images.length} image(s)`);
    }
  }
}
```

---

### Metadata

Document metadata with format-specific fields.

**Type Definition:**

```typescript title="TypeScript"
interface Metadata {
  // Standard 13 Fields
  authors?: string[];
  createdAt?: string;
  createdBy?: string;
  custom?: Record<string, any>;
  date?: string;
  formatType?: string;
  keywords?: string[];
  language?: string;
  modifiedAt?: string;
  modifiedBy?: string;
  pageCount?: number;
  producer?: string;
  subject?: string;
  title?: string;

  // Format-specific fields
  sheetCount?: number;
  sheetNames?: string[];
  fromEmail?: string;
  fromName?: string;
  toEmails?: string[];
  ccEmails?: string[];
  bccEmails?: string[];
  messageId?: string;
  attachments?: string[];

  // Allow for any other fields
  [key: string]: any;
}
```

**Common Fields:**

- `authors` (string[]): Document authors
- `createdAt` (string): Creation date (ISO 8601)
- `createdBy` (string): Creator application/user
- `custom` (Record<string, any>): Custom metadata fields
- `date` (string): Document date
- `formatType` (string): Format discriminator ("pdf", "docx", etc.)
- `keywords` (string[]): Document keywords
- `language` (string): Document language (ISO 639-1)
- `modifiedAt` (string): Modification date (ISO 8601)
- `modifiedBy` (string): Last modifier
- `pageCount` (number): Total number of pages
- `producer` (string): Producer application
- `subject` (string): Document subject
- `title` (string): Document title

**Excel-Specific Fields** (when `formatType === "excel"`):

- `sheetCount` (number): Number of sheets
- `sheetNames` (string[]): Names of the sheets

**Example:**

```typescript title="inspect_pdf_metadata.ts"
const result = extractFileSync('document.pdf');
const metadata = result.metadata;

if (metadata.format_type === 'pdf') {
  console.log(`Title: ${metadata.title}`);
  console.log(`Author: ${metadata.author}`);
  console.log(`Pages: ${metadata.page_count}`);
}
```

See the Types Reference for complete metadata field documentation.

---

### Table

Extracted table structure.

**Type Definition:**

```typescript title="TypeScript"
interface Table {
  cells: string[][];
  markdown: string;
  pageNumber: number;
}
```

**Fields:**

- `cells` (`string[][]`): 2D array of table cells (rows x columns)
- `markdown` (string): Table rendered as markdown
- `pageNumber` (number): Page number where table was found

**Example:**

```typescript title="extract_tables.ts"
const result = extractFileSync('invoice.pdf');

for (const table of result.tables) {
  console.log(`Table on page ${table.pageNumber}:`);
  console.log(table.markdown);
  console.log();
}
```

---

### ChunkMetadata

Metadata for a single text chunk.

**Type Definition:**

```typescript title="TypeScript"
interface ChunkMetadata {
  byteStart: number;
  byteEnd: number;
  charCount: number;
  tokenCount?: number;
  firstPage?: number;
  lastPage?: number;
  headingContext?: HeadingContext;
}
```

**Fields:**

- `byteStart` (number): UTF-8 byte offset in content (inclusive)
- `byteEnd` (number): UTF-8 byte offset in content (exclusive)
- `charCount` (number): Number of characters in chunk
- `tokenCount` (number | undefined): Estimated token count (if configured)
- `firstPage` (number | undefined): First page this chunk appears on (1-indexed, only when page boundaries available)
- `lastPage` (number | undefined): Last page this chunk appears on (1-indexed, only when page boundaries available)
- `headingContext` (HeadingContext | undefined): Heading hierarchy when using Markdown chunker. Only populated when chunker_type is set to markdown.

**Page tracking:** When `PageStructure.boundaries` is available and chunking is enabled, `firstPage` and `lastPage` are automatically calculated based on byte offsets.

**Example:**

```typescript title="chunk_metadata.ts"
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf', null, {
  chunking: { chunkSize: 500, chunkOverlap: 50 },
  pages: { extractPages: true }
});

if (result.chunks) {
  for (const chunk of result.chunks) {
    const meta = chunk.metadata;
    let pageInfo = '';
    if (meta.firstPage !== undefined) {
      if (meta.firstPage === meta.lastPage) {
        pageInfo = ` (page ${meta.firstPage})`;
      } else {
        pageInfo = ` (pages ${meta.firstPage}-${meta.lastPage})`;
      }
    }

    console.log(
      `Chunk [${meta.byteStart}:${meta.byteEnd}]: ${meta.charCount} chars${pageInfo}`
    );
  }
}
```

---

## Extensibility

### Custom Post-Processors

Create custom post-processors to add processing logic to the extraction pipeline.

**Protocol:**

```typescript title="TypeScript"
interface PostProcessorProtocol {
  name(): string;
  process(result: ExtractionResult): ExtractionResult;
  processingStage(): string;
}
```

**Example:**

```typescript title="custom_post_processor.ts"
import { registerPostProcessor, extractFileSync } from '@kreuzberg/node';

class CustomProcessor implements PostProcessorProtocol {
  name(): string {
    return 'custom_processor';
  }

  process(result: ExtractionResult): ExtractionResult {
    result.metadata.customField = 'custom_value';
    return result;
  }

  processingStage(): string {
    return 'middle';
  }
}

registerPostProcessor(new CustomProcessor());

const result = extractFileSync('document.pdf');
console.log(result.metadata.customField);
```

**Managing Processors:**

```typescript title="manage_post_processors.ts"
import {
  registerPostProcessor,
  unregisterPostProcessor,
  clearPostProcessors
} from '@kreuzberg/node';

registerPostProcessor(new CustomProcessor());

unregisterPostProcessor('custom_processor');

clearPostProcessors();
```

---

### Custom Validators

Create custom validators to validate extraction results.

**Protocol:**

```typescript title="TypeScript"
interface ValidatorProtocol {
  name(): string;
  validate(result: ExtractionResult): void;
}
```

**Functions:**

```typescript title="manage_validators.ts"
import {
  registerValidator,
  unregisterValidator,
  clearValidators
} from '@kreuzberg/node';

registerValidator(validator);

unregisterValidator('validator_name');

clearValidators();
```

---

### Custom OCR Backends

Register custom OCR backends for image and PDF processing.

**Example with PaddleOCR (native backend):**

PaddleOCR is now built into the native Rust core. Simply set the backend to `"paddle-ocr"`:

```typescript title="register_paddle_ocr.ts"
import { extractFileSync } from '@kreuzberg/node';

const config = {
  ocr: {
    backend: 'paddle-ocr',
    language: 'en'
  }
};

const result = extractFileSync('scanned.pdf', null, config);
console.log(result.content);
```

---

## Embeddings

### EmbedSync()

Generate embeddings for a list of texts synchronously.

**Signature:**

```typescript
function embedSync(texts: string[], config?: EmbeddingConfig): number[][]
```

**Parameters:**

- `texts` (`string[]`): List of strings to embed.
- `config` (`EmbeddingConfig`, optional): Embedding configuration.

**Returns:** `number[][]` — one embedding vector per input text.

**Example:**

--8<-- "snippets/typescript/utils/standalone_embed.md"

---

### Embed()

Async variant of `embedSync()`.

**Signature:**

```typescript
function embed(texts: string[], config?: EmbeddingConfig): Promise<number[][]>
```

Same parameters and return type as `embedSync()`.

---

## PDF Rendering

!!! Info "Added in v4.6.2"

### RenderPdfPageSync()

Render a single page of a PDF as a PNG image (synchronous).

**Signature:**

```typescript title="TypeScript"
function renderPdfPageSync(filePath: string, pageIndex: number, dpi?: number): Buffer
```

**Parameters:**

- `filePath` (string): Path to the PDF file
- `pageIndex` (number): Zero-based page index to render
- `dpi` (number | undefined): Resolution for rendering (default 150)

**Returns:**

- `Buffer`: PNG-encoded Buffer for the requested page

**Example:**

```typescript title="renderSinglePage.ts"
import { renderPdfPageSync } from "@kreuzberg/node";

const png = renderPdfPageSync("document.pdf", 0);
writeFileSync("first_page.png", png);
```

---

### PdfPageIterator (class)

A more memory-efficient alternative to `iteratePdfPagesSync`/`iteratePdfPages` when memory is a concern or when pages should be processed as they are rendered (for example, sending each page to a vision model for OCR). Renders one page at a time via the `.next()` method.

**Signature:**

```typescript title="TypeScript"
class PdfPageIterator {
    constructor(filePath: string, dpi?: number);
    next(): PdfPageResult | null;
    pageCount(): number;
    close(): void;
}

interface PdfPageResult {
    pageIndex: number;
    data: Buffer;
}
```

**Example:**

```typescript title="iteratePages.ts"
import { PdfPageIterator } from "@kreuzberg/node";

const iter = new PdfPageIterator("document.pdf", 150);
let result;
while ((result = iter.next()) !== null) {
    const { pageIndex, data } = result;
    writeFileSync(`page_${pageIndex}.png`, data);
}
iter.close();
```

---

## Error Handling

All errors are thrown as standard JavaScript `Error` objects with descriptive messages.

**Example:**

```typescript title="error_handling.ts"
import { extractFileSync } from '@kreuzberg/node';

try {
  const result = extractFileSync('document.pdf');
  console.log(result.content);
} catch (error) {
  console.error(`Extraction failed: ${error.message}`);

  if (error.message.includes('file not found')) {
    console.error('File does not exist');
  } else if (error.message.includes('parsing')) {
    console.error('Failed to parse document');
  } else if (error.message.includes('OCR')) {
    console.error('OCR processing failed');
  }
}
```

See [Error Handling Reference](errors.md) for detailed error documentation.

---

## Type Exports

All types are exported for use in your TypeScript code:

```typescript title="type_imports.ts"
import type {
  ExtractionConfig,
  ExtractionResult,
  FileExtractionConfig,
  OcrConfig,
  TesseractConfig,
  PdfConfig,
  ChunkingConfig,
  LanguageDetectionConfig,
  ImageExtractionConfig,
  TokenReductionConfig,
  PostProcessorConfig,
  Table,
  Metadata,
  PostProcessorProtocol,
  ValidatorProtocol,
  OcrBackendProtocol
} from '@kreuzberg/node';
```

---

## Performance Recommendations

### Batch Processing

For processing multiple documents, **always use batch APIs**:

```typescript title="batch_processing_comparison.ts"
// Good - Uses batch API
const batchResults = await batchExtractFiles(['doc1.pdf', 'doc2.pdf', 'doc3.pdf']);

// Bad - Multiple individual calls
const individualResults = [];
for (const file of files) {
  individualResults.push(await extractFile(file));
}
```

**Benefits of batch APIs:**

- Parallel processing in Rust
- Better memory management
- Optimal resource usage

### Sync vs Async

- Use **async functions** (`extractFile`, `batchExtractFiles`) for I/O-bound operations
- Use **sync functions** (`extractFileSync`, `batchExtractFilesSync`) for simple scripts or CLI tools

---

## LLM Integration

Kreuzberg integrates with LLMs via the `liter-llm` crate for structured extraction and VLM-based OCR. See the [LLM Integration Guide](../guides/llm-integration.md) for full details.

### Structured Extraction

Use `structuredExtraction` in your config to extract structured data from documents using an LLM:

--8<-- "snippets/typescript/llm/structured_extraction.md"

The `structuredOutput` field on `ExtractionResult` contains the JSON string conforming to the provided schema:

```typescript title="access_structured_output.ts"
const result = extractFileSync('paper.pdf', null, config);

if (result.structuredOutput) {
  const data = JSON.parse(result.structuredOutput);
  console.log(data.title);
}
```

### VLM OCR

Use a vision-language model as an OCR backend by setting `backend: 'vlm'` with a `vlmConfig`:

--8<-- "snippets/typescript/llm/vlm_ocr.md"

### LLM Embeddings

Generate embeddings using an LLM provider instead of local ONNX models:

```typescript title="llm_embeddings.ts"
import { embedSync } from '@kreuzberg/node';

const vectors = embedSync(['hello world'], {
  modelType: 'llm',
  llm: { model: 'openai/text-embedding-3-small' },
});
```

For configuration details including API keys, model selection, and provider setup, see the [LLM Integration Guide](../guides/llm-integration.md).

---

## Code Intelligence

Kreuzberg uses [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) to parse and analyze source code files across 248 programming languages. When extracting code files, the result metadata includes structural analysis, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available in `result.metadata.format` when `formatType` is `"code"`.

```typescript title="code_intelligence.ts"
import { extractFileSync, ExtractionConfig } from "@kreuzberg/node";

const config: ExtractionConfig = {
  treeSitter: {
    process: {
      structure: true,
      imports: true,
      exports: true,
      comments: true,
      docstrings: true,
    },
  },
};

const result = extractFileSync("app.ts", config);

// Access code intelligence from format metadata
const fmt = result.metadata?.format;
if (fmt && fmt.formatType === "code") {
  console.log(`Language: ${fmt.language}`);
  console.log(`Functions/classes: ${fmt.structure.length}`);
  console.log(`Imports: ${fmt.imports.length}`);

  for (const item of fmt.structure) {
    console.log(`  ${item.kind}: ${item.name} at line ${item.span.startLine}`);
  }

  for (const chunk of fmt.chunks ?? []) {
    console.log(`Chunk: ${chunk.content.slice(0, 50)}...`);
  }
}
```

For configuration details, see the [Code Intelligence Guide](../guides/code-intelligence.md).

---

## System Requirements

**Node.js:** 16.x or higher

**Native Dependencies:**

- Tesseract OCR (for OCR support): `brew install tesseract` (macOS) or `apt-get install tesseract-ocr` (Ubuntu)

**Platforms:**

- Linux (x64, arm64)
- MacOS (x64, arm64)
- Windows (x64)
