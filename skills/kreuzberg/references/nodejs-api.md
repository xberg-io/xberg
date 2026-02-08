# Node.js/TypeScript API Reference

## Overview

**Package**: `@kreuzberg/node` — A high-performance TypeScript SDK built on a Rust core for document intelligence and content extraction.

Supports both **ESM** (`import`) and **CommonJS** (`require`):

```typescript
// ESM
import { extractFile, batchExtractFiles } from '@kreuzberg/node';

// CommonJS
const { extractFile, batchExtractFiles } = require('@kreuzberg/node');
```

**Current Version**: 4.2.14

---

## Core Extraction Functions

All extraction functions return `ExtractionResult` containing extracted content, metadata, tables, and optional chunks/images.

### Single File Extraction

#### `extractFile(filePath, mimeType?, config?): Promise<ExtractionResult>`

Extract content from a single file asynchronously.

```typescript
import { extractFile } from '@kreuzberg/node';

// Auto-detect MIME type from file extension
const result = await extractFile('document.pdf');
console.log(result.content);

// Explicit MIME type
const result2 = await extractFile('document.pdf', 'application/pdf');

// With configuration
const result3 = await extractFile('document.pdf', null, {
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
});
```

**Parameters**:
- `filePath: string` — Path to the file to extract
- `mimeType?: string | null` — Optional MIME type hint (auto-detect if null)
- `config?: ExtractionConfig` — Optional extraction configuration

**Returns**: `Promise<ExtractionResult>`

**Throws**: `ParsingError`, `OcrError`, `ValidationError`, `KreuzbergError`

#### `extractFileSync(filePath, mimeType?, config?): ExtractionResult`

Extract content from a single file synchronously.

```typescript
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');
console.log(result.content);
```

**Parameters**: Same as `extractFile()`

**Returns**: `ExtractionResult`

---

### Raw Bytes Extraction

#### `extractBytes(data, mimeType, config?): Promise<ExtractionResult>`

Extract content from raw bytes (Buffer or Uint8Array) asynchronously.

```typescript
import { extractBytes } from '@kreuzberg/node';
import { readFile } from 'fs/promises';

const data = await readFile('document.pdf');
const result = await extractBytes(data, 'application/pdf');
console.log(result.content);
```

**Parameters**:
- `data: Buffer | Uint8Array` — Raw file content
- `mimeType: string` — MIME type (required)
- `config?: ExtractionConfig` — Optional configuration

**Returns**: `Promise<ExtractionResult>`

#### `extractBytesSync(data, mimeType, config?): ExtractionResult`

Extract content from raw bytes synchronously.

```typescript
import { extractBytesSync } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const data = readFileSync('document.pdf');
const result = extractBytesSync(data, 'application/pdf');
```

**Parameters**: Same as `extractBytes()`

**Returns**: `ExtractionResult`

---

### Batch Extraction (Recommended)

For processing multiple documents, batch APIs provide superior performance and memory management.

#### `batchExtractFiles(paths, config?): Promise<ExtractionResult[]>`

Extract content from multiple files in parallel (asynchronous).

```typescript
import { batchExtractFiles } from '@kreuzberg/node';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = await batchExtractFiles(files);

results.forEach((result, i) => {
  console.log(`${files[i]}: ${result.content.substring(0, 100)}...`);
});
```

**Parameters**:
- `paths: string[]` — Array of file paths
- `config?: ExtractionConfig` — Configuration (applied to all files)

**Returns**: `Promise<ExtractionResult[]>` — Results in same order as input

#### `batchExtractFilesSync(paths, config?): ExtractionResult[]`

Extract content from multiple files synchronously.

```typescript
import { batchExtractFilesSync } from '@kreuzberg/node';

const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
const results = batchExtractFilesSync(files);
```

**Parameters**: Same as `batchExtractFiles()`

**Returns**: `ExtractionResult[]`

#### `batchExtractBytes(dataList, mimeTypes, config?): Promise<ExtractionResult[]>`

Extract content from multiple byte arrays in parallel (asynchronous).

```typescript
import { batchExtractBytes } from '@kreuzberg/node';
import { readFile } from 'fs/promises';

const files = ['doc1.pdf', 'doc2.docx'];
const dataList = await Promise.all(files.map(f => readFile(f)));
const mimeTypes = ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'];

const results = await batchExtractBytes(dataList, mimeTypes);
```

**Parameters**:
- `dataList: Uint8Array[]` — Array of file contents
- `mimeTypes: string[]` — MIME types (one per item, must match length)
- `config?: ExtractionConfig` — Configuration (applied to all items)

**Returns**: `Promise<ExtractionResult[]>`

#### `batchExtractBytesSync(dataList, mimeTypes, config?): ExtractionResult[]`

Extract content from multiple byte arrays synchronously.

```typescript
import { batchExtractBytesSync } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const dataList = ['doc1.pdf', 'doc2.docx'].map(f => readFileSync(f));
const mimeTypes = ['application/pdf', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'];

const results = batchExtractBytesSync(dataList, mimeTypes);
```

**Parameters**: Same as `batchExtractBytes()`

**Returns**: `ExtractionResult[]`

---

## Worker Pool APIs

Worker pools enable concurrent extraction using Node.js worker threads for CPU-bound processing.

### `createWorkerPool(size?): WorkerPool`

Create a worker pool for concurrent extraction.

```typescript
import { createWorkerPool } from '@kreuzberg/node';

// Create pool with default size (number of CPU cores)
const pool = createWorkerPool();

// Create pool with specific size
const pool4 = createWorkerPool(4);
```

**Parameters**:
- `size?: number` — Number of workers (defaults to CPU core count)

**Returns**: `WorkerPool` — Opaque handle for use with worker extraction functions

### `extractFileInWorker(pool, filePath, mimeType?, config?): Promise<ExtractionResult>`

Extract a single file using a worker from the pool.

```typescript
import { createWorkerPool, extractFileInWorker, closeWorkerPool } from '@kreuzberg/node';

const pool = createWorkerPool(4);

try {
  const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
  const results = await Promise.all(
    files.map(f => extractFileInWorker(pool, f))
  );

  results.forEach((r, i) => {
    console.log(`${files[i]}: ${r.content.substring(0, 100)}...`);
  });
} finally {
  await closeWorkerPool(pool);
}
```

**Parameters**:
- `pool: WorkerPool` — Worker pool instance
- `filePath: string` — File path
- `mimeType?: string | null` — Optional MIME type
- `config?: ExtractionConfig` — Optional configuration

**Returns**: `Promise<ExtractionResult>`

### `batchExtractFilesInWorker(pool, paths, config?): Promise<ExtractionResult[]>`

Extract multiple files using the worker pool for concurrent processing.

```typescript
import { createWorkerPool, batchExtractFilesInWorker, closeWorkerPool } from '@kreuzberg/node';

const pool = createWorkerPool(4);

try {
  const files = ['invoice1.pdf', 'invoice2.pdf', 'invoice3.pdf'];
  const results = await batchExtractFilesInWorker(pool, files, {
    ocr: { backend: 'tesseract', language: 'eng' }
  });

  const total = results.reduce((sum, r) => sum + extractAmount(r.content), 0);
  console.log(`Total: $${total}`);
} finally {
  await closeWorkerPool(pool);
}
```

**Parameters**:
- `pool: WorkerPool` — Worker pool instance
- `paths: string[]` — File paths
- `config?: ExtractionConfig` — Configuration (applied to all files)

**Returns**: `Promise<ExtractionResult[]>`

### `getWorkerPoolStats(pool): WorkerPoolStats`

Get statistics about a worker pool.

```typescript
import { createWorkerPool, getWorkerPoolStats } from '@kreuzberg/node';

const pool = createWorkerPool(4);
const stats = getWorkerPoolStats(pool);

console.log(`Pool size: ${stats.size}`);
console.log(`Active workers: ${stats.activeWorkers}`);
console.log(`Queued tasks: ${stats.queuedTasks}`);
```

**Parameters**:
- `pool: WorkerPool` — Worker pool instance

**Returns**: `WorkerPoolStats`

### `closeWorkerPool(pool): Promise<void>`

Close a worker pool and shut down all worker threads.

```typescript
import { createWorkerPool, closeWorkerPool } from '@kreuzberg/node';

const pool = createWorkerPool(4);

try {
  // Use pool
} finally {
  await closeWorkerPool(pool);
}
```

**Parameters**:
- `pool: WorkerPool` — Worker pool instance to close

**Returns**: `Promise<void>`

---

## Configuration Interface

### `ExtractionConfig`

Main configuration object controlling extraction behavior.

```typescript
interface ExtractionConfig {
  // Caching and processing
  useCache?: boolean;                            // Default: true
  enableQualityProcessing?: boolean;            // Default: false

  // OCR configuration
  ocr?: OcrConfig;                              // OCR settings
  forceOcr?: boolean;                           // Default: false

  // Document processing
  chunking?: ChunkingConfig;                    // Break into chunks
  images?: ImageExtractionConfig;               // Image extraction
  pdfOptions?: PdfConfig;                       // PDF-specific options
  tokenReduction?: TokenReductionConfig;        // Token optimization
  languageDetection?: LanguageDetectionConfig;  // Language detection
  postprocessor?: PostProcessorConfig;          // Post-processing
  htmlOptions?: HtmlConversionOptions;          // HTML conversion
  keywords?: KeywordConfig;                     // Keyword extraction
  pages?: PageExtractionConfig;                 // Page extraction

  // Output control
  maxConcurrentExtractions?: number;            // Default: 4
  outputFormat?: 'plain' | 'markdown' | 'djot' | 'html';  // Default: 'plain'
  resultFormat?: 'unified' | 'element_based';  // Default: 'unified'
}
```

### `ChunkingConfig`

Configuration for breaking documents into chunks (useful for RAG and vector databases).

```typescript
interface ChunkingConfig {
  maxChars?: number;          // Max characters per chunk (default: 4096)
  maxOverlap?: number;        // Overlap between chunks (default: 512)
  chunkSize?: number;         // Alternative unit (mutually exclusive with maxChars)
  chunkOverlap?: number;      // Alternative unit (mutually exclusive with maxOverlap)
  preset?: string;            // Named preset ('default', 'aggressive', 'minimal')
  embedding?: Record<string, unknown>;  // Embedding config
  enabled?: boolean;          // Enable chunking (default: true when config provided)
}
```

**Key Point**: Use `maxChars` and `maxOverlap`, NOT `maxCharacters` or `overlap`.

### `OcrConfig`

Configuration for optical character recognition.

```typescript
interface OcrConfig {
  backend: string;              // OCR backend name (e.g., 'tesseract')
  language?: string;            // Language code (e.g., 'eng', 'deu')
  tesseractConfig?: TesseractConfig;
}

interface TesseractConfig {
  psm?: number;                 // Page Segmentation Mode (0-13)
  enableTableDetection?: boolean;
  tesseditCharWhitelist?: string;  // Character whitelist
}
```

### `ImageExtractionConfig`

Configuration for extracting and optimizing images.

```typescript
interface ImageExtractionConfig {
  extractImages?: boolean;      // Default: true
  targetDpi?: number;          // Target DPI (default: 150)
  maxImageDimension?: number;  // Max width/height in pixels (default: 2000)
  autoAdjustDpi?: boolean;     // Auto-adjust DPI (default: true)
  minDpi?: number;             // Minimum DPI (default: 72)
  maxDpi?: number;             // Maximum DPI (default: 300)
}
```

### `PdfConfig`

PDF-specific extraction options.

```typescript
interface PdfConfig {
  extractImages?: boolean;      // Default: true
  passwords?: string[];         // Passwords for encrypted PDFs
  extractMetadata?: boolean;    // Default: true
  hierarchy?: HierarchyConfig;  // Hierarchy extraction
}
```

### `LanguageDetectionConfig`

Configuration for automatic language detection.

```typescript
interface LanguageDetectionConfig {
  enabled?: boolean;            // Default: true
  minConfidence?: number;       // Threshold 0.0-1.0 (default: 0.5)
  detectMultiple?: boolean;     // Detect multiple languages (default: false)
}
```

### `TokenReductionConfig`

Configuration for optimizing token usage.

```typescript
interface TokenReductionConfig {
  mode?: string;                // 'aggressive' or 'conservative' (default: 'conservative')
  preserveImportantWords?: boolean;  // Default: true
}
```

### `KeywordConfig`

Configuration for keyword extraction.

```typescript
interface KeywordConfig {
  algorithm?: 'yake' | 'rake';  // Default: 'yake'
  maxKeywords?: number;         // Maximum keywords (default: 10)
  minScore?: number;            // Minimum relevance score (default: 0.1)
  ngramRange?: [number, number];  // N-gram range (default: [1, 3])
  language?: string;            // Language code (default: 'en')
  yakeParams?: YakeParams;
  rakeParams?: RakeParams;
}
```

### `PageExtractionConfig`

Configuration for page-level content tracking.

```typescript
interface PageExtractionConfig {
  extractPages?: boolean;       // Extract as separate pages array
  insertPageMarkers?: boolean;  // Insert page markers in content
  markerFormat?: string;        // Marker format with {page_num} placeholder
}
```

### `HtmlConversionOptions`

Configuration for HTML to Markdown conversion.

```typescript
interface HtmlConversionOptions {
  headingStyle?: 'atx' | 'underlined' | 'atx_closed';
  listIndentType?: 'spaces' | 'tabs';
  listIndentWidth?: number;
  bullets?: string;
  strongEmSymbol?: string;
  escapeAsterisks?: boolean;
  escapeUnderscores?: boolean;
  escapeMisc?: boolean;
  escapeAscii?: boolean;
  codeLanguage?: string;
  autolinks?: boolean;
  defaultTitle?: boolean;
  brInTables?: boolean;
  hocrSpatialTables?: boolean;
  highlightStyle?: 'double_equal' | 'html' | 'bold' | 'none';
  extractMetadata?: boolean;
  whitespaceMode?: 'normalized' | 'strict';
  stripNewlines?: boolean;
  wrap?: boolean;
  wrapWidth?: number;
  convertAsInline?: boolean;
  subSymbol?: string;
  supSymbol?: string;
  newlineStyle?: 'spaces' | 'backslash';
  codeBlockStyle?: 'indented' | 'backticks' | 'tildes';
  keepInlineImagesIn?: string[];
  encoding?: string;
  debug?: boolean;
  stripTags?: string[];
  preserveTags?: string[];
  preprocessing?: HtmlPreprocessingOptions;
}
```

---

## Result Types

### `ExtractionResult`

Complete extraction result from document processing.

```typescript
interface ExtractionResult {
  // Main content
  content: string;

  // Document type
  mimeType: string;

  // Metadata (format-specific)
  metadata: Metadata;

  // Extracted structures
  tables: Table[];

  // Optional processed data
  detectedLanguages: string[] | null;
  chunks: Chunk[] | null;        // From chunking config
  images: ExtractedImage[] | null;  // From image extraction
  elements?: Element[] | null;   // From element_based result format
  pages?: PageContent[] | null;  // From page extraction
  keywords?: ExtractedKeyword[] | null;  // From keyword extraction
}
```

### `Table`

Extracted table data with cell structure.

```typescript
interface Table {
  cells: string[][];    // 2D array of cell contents (rows × columns)
  markdown: string;     // Markdown representation
  pageNumber: number;   // 1-indexed page number
}
```

### `Chunk`

Text chunk for RAG or vector database indexing.

```typescript
interface Chunk {
  content: string;
  embedding?: number[] | null;  // Vector embedding if computed
  metadata: ChunkMetadata;
}

interface ChunkMetadata {
  byteStart: number;      // UTF-8 byte offset in original text
  byteEnd: number;        // UTF-8 byte offset
  tokenCount?: number | null;
  chunkIndex: number;     // Zero-based index
  totalChunks: number;    // Total number of chunks
  firstPage?: number | null;   // 1-indexed, if page tracking enabled
  lastPage?: number | null;
}
```

### `ExtractedImage`

Image extracted from document.

```typescript
interface ExtractedImage {
  data: Uint8Array;      // Raw image bytes
  format: string;        // Format (e.g., 'png', 'jpeg', 'tiff')
  imageIndex: number;    // Sequential index (0-indexed)
  pageNumber?: number | null;
  width?: number | null;
  height?: number | null;
  colorspace?: string | null;
  bitsPerComponent?: number | null;
  isMask: boolean;
  description?: string | null;
  ocrResult?: ExtractionResult | null;  // OCR result if processed
}
```

### `PageContent`

Per-page content when page extraction is enabled.

```typescript
interface PageContent {
  pageNumber: number;    // 1-indexed
  content: string;       // Page text content
  tables: Table[];       // Tables on this page
  images: ExtractedImage[];  // Images on this page
}
```

### `Metadata`

Extraction result metadata (format-specific).

```typescript
interface Metadata {
  // Common fields
  language?: string | null;
  date?: string | null;
  subject?: string | null;
  format_type?: 'pdf' | 'excel' | 'email' | 'pptx' | 'archive' | 'image' | 'xml' | 'text' | 'html' | 'ocr';

  // PDF metadata
  title?: string | null;
  author?: string | null;
  creator?: string | null;
  producer?: string | null;
  creation_date?: string | null;
  modification_date?: string | null;
  page_count?: number;

  // Excel metadata
  sheet_count?: number;
  sheet_names?: string[];

  // Email metadata
  from_email?: string | null;
  from_name?: string | null;
  to_emails?: string[];
  cc_emails?: string[];
  bcc_emails?: string[];
  message_id?: string | null;
  attachments?: string[];

  // Image metadata
  width?: number;
  height?: number;
  exif?: Record<string, string>;

  // OCR metadata
  psm?: number;
  output_format?: string;
  table_count?: number;

  // HTML metadata
  canonical_url?: string | null;
  html_language?: string | null;
  text_direction?: 'ltr' | 'rtl' | 'auto' | null;
  open_graph?: Record<string, string>;
  twitter_card?: Record<string, string>;
  meta_tags?: Record<string, string>;
  html_headers?: HeaderMetadata[];
  html_links?: LinkMetadata[];
  html_images?: HtmlImageMetadata[];
  structured_data?: StructuredData[];

  // Text metadata
  line_count?: number;
  word_count?: number;
  character_count?: number;
  headers?: string[] | null;
  links?: [string, string][] | null;
  code_blocks?: [string, string][] | null;

  // Page structure
  page_structure?: PageStructure | null;

  // Custom fields from postprocessors
  [key: string]: unknown;
}
```

---

## Error Handling

### Error Classes

```typescript
import {
  KreuzbergError,
  ParsingError,
  OcrError,           // Note: camelCase, not "OCRError"
  ValidationError,
  MissingDependencyError,
  CacheError,
  ImageProcessingError,
  PluginError,
  ErrorCode,
} from '@kreuzberg/node';
```

**Error Hierarchy**:
- `KreuzbergError` — Base class for all Kreuzberg errors
  - `ParsingError` — Document format invalid or corrupted
  - `OcrError` — OCR processing failed
  - `ValidationError` — Extraction validation failed
  - `MissingDependencyError` — Required dependency unavailable
  - `CacheError` — Cache operation failed
  - `ImageProcessingError` — Image extraction or processing failed
  - `PluginError` — Plugin registration or execution failed

### Error Diagnostics

```typescript
import {
  classifyError,
  getErrorCodeDescription,
  getErrorCodeName,
  getLastErrorCode,
  getLastPanicContext,
} from '@kreuzberg/node';

try {
  const result = await extractFile('document.pdf');
} catch (error) {
  const classification = classifyError(error.message);
  console.log(`Error code: ${getErrorCodeName(classification.code)}`);
  console.log(`Description: ${getErrorCodeDescription(classification.code)}`);
  console.log(`Confidence: ${classification.confidence}`);
}
```

### `ErrorCode` Enum

```typescript
enum ErrorCode {
  Success = 0,
  GenericError = 1,
  Panic = 2,
  InvalidArgument = 3,
  IoError = 4,
  ParsingError = 5,
  OcrError = 6,
  MissingDependency = 7,
}
```

---

## Plugin System

### Post-Processors

Custom post-processors can enrich extraction results without failing the extraction if they encounter errors.

#### `registerPostProcessor(processor): void`

Register a custom post-processor.

```typescript
import { registerPostProcessor, extractFile } from '@kreuzberg/node';

const processor = {
  name() {
    return 'my_processor';
  },

  async process(result) {
    // Enrich result with custom metadata
    result.metadata['custom_field'] = 'value';
    return result;
  },

  processingStage() {
    return 'late';  // 'early', 'middle', or 'late'
  },

  async initialize() {
    // Called once when registered
  },

  async shutdown() {
    // Called when unregistered
  },
};

registerPostProcessor(processor);
const result = await extractFile('document.pdf');
```

#### `unregisterPostProcessor(name): void`

Remove a registered post-processor.

```typescript
import { unregisterPostProcessor } from '@kreuzberg/node';

unregisterPostProcessor('my_processor');
```

#### `listPostProcessors(): string[]`

List all registered post-processor names.

```typescript
import { listPostProcessors } from '@kreuzberg/node';

const processors = listPostProcessors();
console.log('Registered processors:', processors);
```

#### `clearPostProcessors(): void`

Unregister all post-processors.

```typescript
import { clearPostProcessors } from '@kreuzberg/node';

clearPostProcessors();
```

### Validators

Custom validators check extraction results and fail the extraction if validation fails (unlike post-processors).

#### `registerValidator(validator): void`

Register a custom validator.

```typescript
import { registerValidator, extractFile } from '@kreuzberg/node';

const validator = {
  name() {
    return 'content_length_validator';
  },

  validate(result) {
    if (result.content.length < 10) {
      throw new Error('Content too short');
    }
  },

  priority() {
    return 100;  // Higher = runs first
  },

  shouldValidate(result) {
    return result.mimeType === 'application/pdf';  // Conditional validation
  },

  async initialize() {
    // Called once when registered
  },

  async shutdown() {
    // Called when unregistered
  },
};

registerValidator(validator);
const result = await extractFile('document.pdf');
```

#### `unregisterValidator(name): void`

Remove a registered validator.

```typescript
import { unregisterValidator } from '@kreuzberg/node';

unregisterValidator('content_length_validator');
```

#### `listValidators(): string[]`

List all registered validator names.

```typescript
import { listValidators } from '@kreuzberg/node';

const validators = listValidators();
```

#### `clearValidators(): void`

Unregister all validators.

```typescript
import { clearValidators } from '@kreuzberg/node';

clearValidators();
```

### OCR Backends

Custom OCR backends can be registered to handle image text extraction.

#### `registerOcrBackend(backend): void`

Register a custom OCR backend.

```typescript
import { registerOcrBackend, extractFile } from '@kreuzberg/node';

const backend = {
  name() {
    return 'my-ocr';
  },

  supportedLanguages() {
    return ['eng', 'deu', 'fra'];
  },

  async processImage(imageBytes, language) {
    // imageBytes: Uint8Array or Base64 string
    const buffer = typeof imageBytes === 'string'
      ? Buffer.from(imageBytes, 'base64')
      : Buffer.from(imageBytes);

    // Process and extract text
    return {
      content: 'extracted text',
      mime_type: 'text/plain',
      metadata: { confidence: 0.95, language },
      tables: [],
    };
  },

  async initialize() {
    // Load models, setup resources
  },

  async shutdown() {
    // Cleanup resources
  },
};

registerOcrBackend(backend);
```

#### `GutenOcrBackend`

Built-in OCR backend implementation using Guten-OCR.

```typescript
import { GutenOcrBackend, registerOcrBackend, extractFile } from '@kreuzberg/node';

const backend = new GutenOcrBackend();
await backend.initialize();
registerOcrBackend(backend);

const result = await extractFile('scanned.pdf', null, {
  ocr: { backend: 'guten-ocr', language: 'eng' }
});
```

#### `unregisterOcrBackend(name): void`

Remove a registered OCR backend.

```typescript
import { unregisterOcrBackend } from '@kreuzberg/node';

unregisterOcrBackend('my-ocr');
```

#### `listOcrBackends(): string[]`

List all registered OCR backend names.

```typescript
import { listOcrBackends } from '@kreuzberg/node';

const backends = listOcrBackends();
```

#### `clearOcrBackends(): void`

Unregister all OCR backends.

```typescript
import { clearOcrBackends } from '@kreuzberg/node';

clearOcrBackends();
```

---

## MIME Type Utilities

### `detectMimeType(data): string | null`

Detect MIME type from file content (magic bytes).

```typescript
import { detectMimeType } from '@kreuzberg/node';
import { readFileSync } from 'fs';

const data = readFileSync('document');
const mimeType = detectMimeType(data);
console.log(`Detected MIME type: ${mimeType}`);
```

### `detectMimeTypeFromPath(filePath): string | null`

Detect MIME type from file extension.

```typescript
import { detectMimeTypeFromPath } from '@kreuzberg/node';

const mimeType = detectMimeTypeFromPath('document.pdf');
console.log(`MIME type: ${mimeType}`);  // 'application/pdf'
```

### `getExtensionsForMime(mimeType): string[]`

Get file extensions for a MIME type.

```typescript
import { getExtensionsForMime } from '@kreuzberg/node';

const extensions = getExtensionsForMime('application/pdf');
console.log(`Extensions: ${extensions}`);  // ['.pdf']
```

### `validateMimeType(mimeType): boolean`

Check if a MIME type is valid.

```typescript
import { validateMimeType } from '@kreuzberg/node';

if (validateMimeType('application/pdf')) {
  console.log('Valid MIME type');
}
```

---

## Configuration Loading

### `ExtractionConfig.fromFile(filePath): ExtractionConfig`

Load extraction configuration from a file (JSON, YAML, or TOML).

```typescript
import { ExtractionConfig, extractFile } from '@kreuzberg/node';

const config = ExtractionConfig.fromFile('./kreuzberg.toml');
const result = await extractFile('document.pdf', null, config);
```

### `ExtractionConfig.discover(): ExtractionConfig | null`

Auto-discover extraction configuration file in current and parent directories.

```typescript
import { ExtractionConfig, extractFile } from '@kreuzberg/node';

// Searches for kreuzberg.{toml,yaml,json} in current directory and parents
const config = ExtractionConfig.discover();
if (config) {
  const result = await extractFile('document.pdf', null, config);
}
```

---

## Embeddings

### `getEmbeddingPreset(name): EmbeddingPreset | null`

Get a named embedding model preset.

```typescript
import { getEmbeddingPreset } from '@kreuzberg/node';

const preset = getEmbeddingPreset('default');
if (preset) {
  console.log(`Model: ${preset.modelName}`);
  console.log(`Dimensions: ${preset.dimensions}`);
}
```

### `listEmbeddingPresets(): string[]`

List all available embedding presets.

```typescript
import { listEmbeddingPresets } from '@kreuzberg/node';

const presets = listEmbeddingPresets();
console.log('Available presets:', presets);
```

### `EmbeddingPreset`

Type definition for embedding model presets.

```typescript
interface EmbeddingPreset {
  name: string;           // Preset name (e.g., "fast", "balanced", "quality", "multilingual")
  chunkSize: number;      // Recommended chunk size in characters
  overlap: number;        // Recommended overlap in characters
  modelName: string;      // Model identifier (e.g., "AllMiniLML6V2Q", "BGEBaseENV15")
  dimensions: number;     // Embedding vector dimensions
  description: string;    // Human-readable description
}
```

---

## Plugin Protocols

### `PostProcessorProtocol`

Interface for custom post-processors.

```typescript
interface PostProcessorProtocol {
  name(): string;

  process(result: ExtractionResult): ExtractionResult | Promise<ExtractionResult>;

  processingStage?(): ProcessingStage;  // 'early' | 'middle' | 'late'

  initialize?(): void | Promise<void>;

  shutdown?(): void | Promise<void>;
}
```

### `ValidatorProtocol`

Interface for custom validators.

```typescript
interface ValidatorProtocol {
  name(): string;

  validate(result: ExtractionResult): void | Promise<void>;

  priority?(): number;  // Higher = runs first

  shouldValidate?(result: ExtractionResult): boolean;

  initialize?(): void | Promise<void>;

  shutdown?(): void | Promise<void>;
}
```

### `OcrBackendProtocol`

Interface for custom OCR backends.

```typescript
interface OcrBackendProtocol {
  name(): string;

  supportedLanguages(): string[];

  processImage(
    imageBytes: Uint8Array | string,
    language: string,
  ): Promise<{
    content: string;
    mime_type: string;
    metadata: Record<string, unknown>;
    tables: unknown[];
  }>;

  initialize?(): void | Promise<void>;

  shutdown?(): void | Promise<void>;
}
```

---

## Supported Document Formats

- **Documents**: PDF, DOCX, PPTX, XLSX, DOC, PPT (with LibreOffice)
- **Text**: Markdown, Plain Text, XML, JSON, YAML, TOML
- **Web**: HTML (converted to Markdown)
- **Email**: EML, MSG
- **Images**: PNG, JPEG, TIFF (with OCR support)
- **Archives**: ZIP, TAR, GZIP (file listing)

---

## Registry Functions

### Document Extractors

```typescript
import {
  listDocumentExtractors,
  unregisterDocumentExtractor,
  clearDocumentExtractors,
} from '@kreuzberg/node';

// List registered extractors
const extractors = listDocumentExtractors();

// Unregister a specific extractor
unregisterDocumentExtractor('pdf');

// Clear all extractors
clearDocumentExtractors();
```

---

## Type Exports

All types are exported from `@kreuzberg/node`:

```typescript
export type {
  Chunk,
  ChunkingConfig,
  ExtractionConfig,
  ExtractionResult,
  ExtractedImage,
  KeywordConfig,
  LanguageDetectionConfig,
  OcrBackendProtocol,
  OcrConfig,
  PageContent,
  PageExtractionConfig,
  PdfConfig,
  PostProcessorProtocol,
  Table,
  TokenReductionConfig,
  ValidatorProtocol,
  WorkerPool,
  WorkerPoolStats,
  EmbeddingPreset,
  // ... and many more
};
```

---

## Best Practices

1. **Use batch APIs for multiple documents**: `batchExtractFiles()` provides superior performance vs. calling `extractFile()` in a loop.

2. **Enable chunking for RAG/vector DB**: Set `chunking` config to automatically break documents into overlapping chunks.

3. **Use worker pools for high-concurrency scenarios**: Distribute CPU-bound work across multiple threads for 4+ concurrent extractions.

4. **Configure language detection**: Enable automatic language detection for multilingual documents.

5. **Register validators early**: Set up validators before calling extraction functions to catch quality issues immediately.

6. **Use specific MIME types**: Provide explicit MIME types when available to avoid detection overhead.

7. **Clean up resources**: Always call `closeWorkerPool()` when done to prevent resource leaks.

8. **Handle extraction errors gracefully**: Catch specific error types (`ParsingError`, `OcrError`, etc.) for appropriate error handling.

---

## Version

**Package Version**: 4.2.14
