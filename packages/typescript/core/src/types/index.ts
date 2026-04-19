/**
 * Kreuzberg Core Types Package
 *
 * This package exports all shared type definitions used by Kreuzberg bindings
 * (Node.js, WASM, etc.). It provides:
 *
 * - Configuration types for extraction options
 * - Result types for extraction outputs
 * - Metadata types for document information
 * - Protocol interfaces for custom plugins
 * - Error types and error codes
 */

export * from "../constants/index.js";
export * from "../utils/index.js";
export type {
	AccelerationConfig,
	ChunkingConfig,
	ConcurrencyConfig,
	ContentFilterConfig,
	EmailConfig,
	EmbeddingConfig,
	EmbeddingModelType,
	ExtractedKeyword,
	ExtractionConfig,
	HierarchyConfig,
	HtmlConversionOptions,
	HtmlPreprocessingOptions,
	ImageExtractionConfig,
	KeywordAlgorithm,
	KeywordConfig,
	LanguageDetectionConfig,
	OcrConfig,
	PageConfig,
	PdfConfig,
	PostProcessorConfig,
	RakeParams,
	TesseractConfig,
	TokenReductionConfig,
	YakeParams,
} from "./config.js";
export {
	CacheError,
	EmbeddingError,
	ErrorCode,
	ImageProcessingError,
	KreuzbergError,
	MissingDependencyError,
	OcrError,
	type PanicContext,
	ParsingError,
	PluginError,
	ValidationError,
} from "./errors.js";
export type {
	ArchiveMetadata,
	EmailMetadata,
	ErrorMetadata,
	ExcelMetadata,
	HtmlMetadata,
	ImageMetadata,
	ImagePreprocessingMetadata,
	Metadata,
	OcrMetadata,
	PdfMetadata,
	PptxMetadata,
	TextMetadata,
	XmlMetadata,
} from "./metadata.js";
export type {
	OcrBackendProtocol,
	PostProcessorProtocol,
	ProcessingStage,
	ValidatorProtocol,
} from "./protocols.js";
export type {
	ArchiveEntry,
	BoundingBox,
	Chunk,
	ChunkMetadata,
	Element,
	ElementMetadata,
	ElementType,
	ExtractedImage,
	ExtractionResult,
	HeadingContext,
	HeadingLevel,
	HierarchicalBlock,
	LayoutRegion,
	OutputFormat,
	PageContent,
	PageHierarchy,
	PageUnitType,
	ProcessingWarning,
	RelationshipKind,
	ResultFormat,
	Table,
	Uri,
	UriKind,
} from "./results.js";
