/**
 * Kreuzberg - Multi-language document intelligence framework.
 *
 * This is a TypeScript SDK around a high-performance Rust core.
 * All extraction logic, chunking, quality processing, and language detection
 * are implemented in Rust for maximum performance.
 *
 * ## Module Organization
 *
 * The SDK is organized into logical domains:
 * - **Extraction**: Single and batch document extraction with worker pool support
 * - **Types**: Core type definitions and interfaces
 * - **Errors**: Error classes and diagnostic utilities
 * - **Plugins**: Custom post-processors, validators, and OCR backends
 * - **Registry**: Plugin and document extractor management
 * - **Config**: Configuration loading and management
 * - **MIME**: MIME type detection and validation
 * - **Embeddings**: Embedding model presets
 *
 * ## API Usage Recommendations
 *
 * **For processing multiple documents**, prefer batch APIs:
 * - Use `batchExtractFiles()` / `batchExtractFilesSync()` for multiple files
 * - Use `batchExtractBytes()` / `batchExtractBytesSync()` for multiple byte arrays
 * - Use worker pool APIs for high-concurrency scenarios
 *
 * **Batch APIs provide**:
 * - Better performance (parallel processing in Rust)
 * - More reliable memory management
 * - Recommended for all multi-document workflows
 *
 * **Single extraction APIs** (`extractFile`, `extractBytes`) are suitable for:
 * - One-off document processing
 * - Interactive applications processing documents on-demand
 * - Avoid calling these in tight loops - use batch APIs instead
 *
 * ## Supported Formats
 *
 * - **Documents**: PDF, DOCX, PPTX, XLSX, DOC, PPT (with LibreOffice)
 * - **Text**: Markdown, Plain Text, XML
 * - **Web**: HTML (converted to Markdown)
 * - **Data**: JSON, YAML, TOML
 * - **Email**: EML, MSG
 * - **Images**: PNG, JPEG, TIFF (with OCR support)
 *
 * @example
 * ```typescript
 * import { extractFile, batchExtractFiles } from '@kreuzberg/node';
 *
 * // Single file extraction
 * const result = await extractFile('document.pdf');
 * console.log(result.content);
 *
 * // Multiple files (recommended approach)
 * const files = ['doc1.pdf', 'doc2.docx', 'doc3.xlsx'];
 * const results = await batchExtractFiles(files);
 * results.forEach(r => console.log(r.content));
 * ```
 *
 * @module @kreuzberg/node
 */

// ============================================================================
// Types
// ============================================================================

export type {
	Chunk,
	ChunkingConfig,
	ErrorClassification,
	ExtractedImage,
	ExtractionConfig,
	ExtractionResult,
	HtmlConversionOptions,
	HtmlPreprocessingOptions,
	ImageExtractionConfig,
	KeywordConfig,
	LanguageDetectionConfig,
	OcrBackendProtocol,
	OcrConfig,
	PageContent,
	PageExtractionConfig,
	PdfConfig,
	PostProcessorConfig,
	PostProcessorProtocol,
	Table,
	TesseractConfig,
	TokenReductionConfig,
	ValidatorProtocol,
	WorkerPool,
	WorkerPoolStats,
} from "./types.js";

// ============================================================================
// Errors and Error Handling
// ============================================================================

export {
	classifyError,
	getErrorCodeDescription,
	getErrorCodeName,
	getLastErrorCode,
	getLastPanicContext,
} from "./errors/diagnostics.js";
export type { PanicContext } from "./errors.js";
export {
	CacheError,
	ErrorCode,
	ImageProcessingError,
	KreuzbergError,
	MissingDependencyError,
	OcrError,
	ParsingError,
	PluginError,
	ValidationError,
} from "./errors.js";

// ============================================================================
// Core Extraction APIs
// ============================================================================

export {
	batchExtractBytes,
	batchExtractBytesSync,
	batchExtractFiles,
	batchExtractFilesSync,
} from "./extraction/batch.js";
export {
	extractBytes,
	extractBytesSync,
	extractFile,
	extractFileSync,
} from "./extraction/single.js";

// ============================================================================
// Worker Pool APIs
// ============================================================================

export {
	batchExtractFilesInWorker,
	closeWorkerPool,
	createWorkerPool,
	extractFileInWorker,
	getWorkerPoolStats,
} from "./extraction/worker-pool.js";

// ============================================================================
// Plugin System: Post-Processors
// ============================================================================

export {
	clearPostProcessors,
	listPostProcessors,
	registerPostProcessor,
	unregisterPostProcessor,
} from "./plugins/post-processors.js";

// ============================================================================
// Plugin System: Validators
// ============================================================================

export {
	clearValidators,
	listValidators,
	registerValidator,
	unregisterValidator,
} from "./plugins/validators.js";

// ============================================================================
// Plugin System: OCR Backends
// ============================================================================

export { GutenOcrBackend } from "./ocr/guten-ocr.js";
export {
	clearOcrBackends,
	listOcrBackends,
	registerOcrBackend,
	unregisterOcrBackend,
} from "./plugins/ocr-backends.js";

// ============================================================================
// Registry: Document Extractors
// ============================================================================

export {
	clearDocumentExtractors,
	listDocumentExtractors,
	unregisterDocumentExtractor,
} from "./registry/document-extractors.js";

// ============================================================================
// Configuration
// ============================================================================

export * from "./config/loader.js";

// ============================================================================
// MIME Type Utilities
// ============================================================================

export {
	detectMimeType,
	detectMimeTypeFromPath,
	getExtensionsForMime,
	validateMimeType,
} from "./mime/utilities.js";

// ============================================================================
// Embeddings
// ============================================================================

export type { EmbeddingPreset } from "./embeddings/presets.js";
export {
	getEmbeddingPreset,
	listEmbeddingPresets,
} from "./embeddings/presets.js";

// ============================================================================
// Version
// ============================================================================

export const __version__ = "4.2.4";

// ============================================================================
// Test Utilities
// ============================================================================

export { __resetBindingForTests, __setBindingForTests } from "./core/binding.js";
