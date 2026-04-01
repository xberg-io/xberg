package kreuzberg

// This file contains pure Go type definitions for Kreuzberg configuration.
// These types are intentionally separated from CGO code so they remain available
// when CGO is disabled (e.g., during linting with CGO_ENABLED=0).

// Functional option types for idiomatic Go configuration building.
// See config_options.go for usage examples and option constructors.

// ExtractionOption is a functional option for configuring ExtractionConfig.
type ExtractionOption func(*ExtractionConfig)

// OCROption is a functional option for configuring OCRConfig.
type OCROption func(*OCRConfig)

// TesseractOption is a functional option for configuring TesseractConfig.
type TesseractOption func(*TesseractConfig)

// ImagePreprocessingOption is a functional option for configuring ImagePreprocessingConfig.
type ImagePreprocessingOption func(*ImagePreprocessingConfig)

// ChunkingOption is a functional option for configuring ChunkingConfig.
type ChunkingOption func(*ChunkingConfig)

// EmbeddingOption is a functional option for configuring EmbeddingConfig.
type EmbeddingOption func(*EmbeddingConfig)

// ImageExtractionOption is a functional option for configuring ImageExtractionConfig.
type ImageExtractionOption func(*ImageExtractionConfig)

// FontConfigOption is a functional option for configuring FontConfig.
type FontConfigOption func(*FontConfig)

// AccelerationConfigOption is a functional option for configuring AccelerationConfig.
type AccelerationConfigOption func(*AccelerationConfig)

// PdfOption is a functional option for configuring PdfConfig.
type PdfOption func(*PdfConfig)

// HierarchyOption is a functional option for configuring HierarchyConfig.
type HierarchyOption func(*HierarchyConfig)

// TokenReductionOption is a functional option for configuring TokenReductionConfig.
type TokenReductionOption func(*TokenReductionConfig)

// LanguageDetectionOption is a functional option for configuring LanguageDetectionConfig.
type LanguageDetectionOption func(*LanguageDetectionConfig)

// PostProcessorOption is a functional option for configuring PostProcessorConfig.
type PostProcessorOption func(*PostProcessorConfig)

// LayoutDetectionOption is a functional option for configuring LayoutDetectionConfig.
type LayoutDetectionOption func(*LayoutDetectionConfig)

// EmbeddingModelTypeOption is a functional option for configuring EmbeddingModelType.
type EmbeddingModelTypeOption func(*EmbeddingModelType)

// KeywordOption is a functional option for configuring KeywordConfig.
type KeywordOption func(*KeywordConfig)

// YakeParamsOption is a functional option for configuring YakeParams.
type YakeParamsOption func(*YakeParams)

// RakeParamsOption is a functional option for configuring RakeParams.
type RakeParamsOption func(*RakeParams)

// HTMLPreprocessingOption is a functional option for configuring HTMLPreprocessingOptions.
type HTMLPreprocessingOption func(*HTMLPreprocessingOptions)

// HTMLConversionOption is a functional option for configuring HTMLConversionOptions.
type HTMLConversionOption func(*HTMLConversionOptions)

// ConcurrencyOption is a functional option for configuring ConcurrencyConfig.
type ConcurrencyOption func(*ConcurrencyConfig)

// TreeSitterProcessConfigOption is a functional option for configuring TreeSitterProcessConfig.
type TreeSitterProcessConfigOption func(*TreeSitterProcessConfig)

// TreeSitterConfigOption is a functional option for configuring TreeSitterConfig.
type TreeSitterConfigOption func(*TreeSitterConfig)

// PageOption is a functional option for configuring PageConfig.
type PageOption func(*PageConfig)

// ExtractionConfig mirrors the Rust ExtractionConfig structure and is serialized to JSON
// before crossing the FFI boundary. Use pointer fields to omit values and rely on Kreuzberg
// defaults whenever possible.
type ExtractionConfig struct {
	UseCache                 *bool                    `json:"use_cache,omitempty"`
	EnableQualityProcessing  *bool                    `json:"enable_quality_processing,omitempty"`
	Ocr                      *OCRConfig               `json:"ocr,omitempty"`
	ForceOcr                 *bool                    `json:"force_ocr,omitempty"`
	DisableOcr               *bool                    `json:"disable_ocr,omitempty"`
	ForceOcrPages            []uint64                 `json:"force_ocr_pages,omitempty"`
	Chunking                 *ChunkingConfig          `json:"chunking,omitempty"`
	Images                   *ImageExtractionConfig   `json:"images,omitempty"`
	PdfOptions               *PdfConfig               `json:"pdf_options,omitempty"`
	TokenReduction           *TokenReductionConfig    `json:"token_reduction,omitempty"`
	LanguageDetection        *LanguageDetectionConfig `json:"language_detection,omitempty"`
	Keywords                 *KeywordConfig           `json:"keywords,omitempty"`
	Postprocessor            *PostProcessorConfig     `json:"postprocessor,omitempty"`
	HTMLOptions              *HTMLConversionOptions   `json:"html_options,omitempty"`
	Layout                   *LayoutDetectionConfig   `json:"layout,omitempty"`
	Pages                    *PageConfig              `json:"pages,omitempty"`
	SecurityLimits           *SecurityLimitsConfig    `json:"security_limits,omitempty"`
	Acceleration             *AccelerationConfig      `json:"acceleration,omitempty"`
	Email                    *EmailConfig             `json:"email,omitempty"`
	Concurrency              *ConcurrencyConfig       `json:"concurrency,omitempty"`
	MaxConcurrentExtractions *int                     `json:"max_concurrent_extractions,omitempty"`
	IncludeDocumentStructure *bool                    `json:"include_document_structure,omitempty"`
	OutputFormat             string                   `json:"output_format,omitempty"`
	ResultFormat             string                   `json:"result_format,omitempty"`
	CacheNamespace           *string                  `json:"cache_namespace,omitempty"`
	CacheTTLSecs             *uint64                  `json:"cache_ttl_secs,omitempty"`
	ExtractionTimeoutSecs    *uint64                  `json:"extraction_timeout_secs,omitempty"`
	MaxArchiveDepth          *int                     `json:"max_archive_depth,omitempty"`
	TreeSitter               *TreeSitterConfig        `json:"tree_sitter,omitempty"`
}

// SecurityLimitsConfig controls security thresholds for archive extraction.
type SecurityLimitsConfig struct {
	MaxArchiveSize      *int `json:"max_archive_size,omitempty"`
	MaxCompressionRatio *int `json:"max_compression_ratio,omitempty"`
	MaxFilesInArchive   *int `json:"max_files_in_archive,omitempty"`
	MaxNestingDepth     *int `json:"max_nesting_depth,omitempty"`
	MaxEntityLength     *int `json:"max_entity_length,omitempty"`
	MaxContentSize      *int `json:"max_content_size,omitempty"`
	MaxIterations       *int `json:"max_iterations,omitempty"`
	MaxXMLDepth         *int `json:"max_xml_depth,omitempty"`
	MaxTableCells       *int `json:"max_table_cells,omitempty"`
}

// OcrElementConfig controls OCR element extraction behavior.
type OcrElementConfig struct {
	IncludeElements bool    `json:"include_elements"`
	MinLevel        string  `json:"min_level,omitempty"`
	MinConfidence   float64 `json:"min_confidence,omitempty"`
	BuildHierarchy  bool    `json:"build_hierarchy"`
}

// PaddleOcrConfig exposes fine-grained controls for the PaddleOCR backend.
type PaddleOcrConfig struct {
	Language             string   `json:"language,omitempty"`
	CacheDir             string   `json:"cache_dir,omitempty"`
	UseAngleCls          *bool    `json:"use_angle_cls,omitempty"`
	EnableTableDetection *bool    `json:"enable_table_detection,omitempty"`
	DetDbThresh          *float64 `json:"det_db_thresh,omitempty"`
	DetDbBoxThresh       *float64 `json:"det_db_box_thresh,omitempty"`
	DetDbUnclipRatio     *float64 `json:"det_db_unclip_ratio,omitempty"`
	DetLimitSideLen      *int     `json:"det_limit_side_len,omitempty"`
	RecBatchNum          *int     `json:"rec_batch_num,omitempty"`
	Padding              *int     `json:"padding,omitempty"`
	ModelTier            string   `json:"model_tier,omitempty"`
}

// OCRConfig selects and configures OCR backends.
type OCRConfig struct {
	Backend       string            `json:"backend,omitempty"`
	Language      *string           `json:"language,omitempty"`
	Tesseract     *TesseractConfig  `json:"tesseract_config,omitempty"`
	PaddleOcr     *PaddleOcrConfig  `json:"paddle_ocr_config,omitempty"`
	ElementConfig *OcrElementConfig `json:"element_config,omitempty"`
}

// TesseractConfig exposes fine-grained controls for the Tesseract backend.
type TesseractConfig struct {
	Language                       string                    `json:"language,omitempty"`
	PSM                            *int                      `json:"psm,omitempty"`
	OutputFormat                   string                    `json:"output_format,omitempty"`
	OEM                            *int                      `json:"oem,omitempty"`
	MinConfidence                  *float64                  `json:"min_confidence,omitempty"`
	Preprocessing                  *ImagePreprocessingConfig `json:"preprocessing,omitempty"`
	EnableTableDetection           *bool                     `json:"enable_table_detection,omitempty"`
	TableMinConfidence             *float64                  `json:"table_min_confidence,omitempty"`
	TableColumnThreshold           *int                      `json:"table_column_threshold,omitempty"`
	TableRowThresholdRatio         *float64                  `json:"table_row_threshold_ratio,omitempty"`
	UseCache                       *bool                     `json:"use_cache,omitempty"`
	ClassifyUsePreAdaptedTemplates *bool                     `json:"classify_use_pre_adapted_templates,omitempty"`
	LanguageModelNgramOn           *bool                     `json:"language_model_ngram_on,omitempty"`
	TesseditDontBlkrejGoodWds      *bool                     `json:"tessedit_dont_blkrej_good_wds,omitempty"`
	TesseditDontRowrejGoodWds      *bool                     `json:"tessedit_dont_rowrej_good_wds,omitempty"`
	TesseditEnableDictCorrection   *bool                     `json:"tessedit_enable_dict_correction,omitempty"`
	TesseditCharWhitelist          string                    `json:"tessedit_char_whitelist,omitempty"`
	TesseditCharBlacklist          string                    `json:"tessedit_char_blacklist,omitempty"`
	TesseditUsePrimaryParamsModel  *bool                     `json:"tessedit_use_primary_params_model,omitempty"`
	TextordSpaceSizeIsVariable     *bool                     `json:"textord_space_size_is_variable,omitempty"`
	ThresholdingMethod             *bool                     `json:"thresholding_method,omitempty"`
}

// ImagePreprocessingConfig tunes DPI normalization and related steps for OCR.
type ImagePreprocessingConfig struct {
	TargetDPI        *int   `json:"target_dpi,omitempty"`
	AutoRotate       *bool  `json:"auto_rotate,omitempty"`
	Deskew           *bool  `json:"deskew,omitempty"`
	Denoise          *bool  `json:"denoise,omitempty"`
	ContrastEnhance  *bool  `json:"contrast_enhance,omitempty"`
	BinarizationMode string `json:"binarization_method,omitempty"`
	InvertColors     *bool  `json:"invert_colors,omitempty"`
}

// ChunkSizingConfig controls how chunk size is measured.
//
// When Type is "tokenizer", chunks are sized by token count using the specified
// HuggingFace tokenizer model. Otherwise chunks are sized by character count.
type ChunkSizingConfig struct {
	Type     string `json:"type"`
	Model    string `json:"model,omitempty"`
	CacheDir string `json:"cache_dir,omitempty"`
}

// ChunkingConfig configures text chunking for downstream RAG/Retrieval workloads.
type ChunkingConfig struct {
	MaxChars              *int               `json:"max_chars,omitempty"`
	MaxOverlap            *int               `json:"max_overlap,omitempty"`
	ChunkSize             *int               `json:"chunk_size,omitempty"`
	ChunkOverlap          *int               `json:"chunk_overlap,omitempty"`
	Preset                *string            `json:"preset,omitempty"`
	ChunkerType           *string            `json:"chunker_type,omitempty"`
	Enabled               *bool              `json:"enabled,omitempty"`
	Embedding             *EmbeddingConfig   `json:"embedding,omitempty"`
	Sizing                *ChunkSizingConfig `json:"sizing,omitempty"`
	PrependHeadingContext *bool              `json:"prepend_heading_context,omitempty"`
}

// ImageExtractionConfig controls inline image extraction from PDFs/Office docs.
type ImageExtractionConfig struct {
	ExtractImages     *bool `json:"extract_images,omitempty"`
	TargetDPI         *int  `json:"target_dpi,omitempty"`
	MaxImageDimension *int  `json:"max_image_dimension,omitempty"`
	AutoAdjustDPI     *bool `json:"auto_adjust_dpi,omitempty"`
	MinDPI            *int  `json:"min_dpi,omitempty"`
	MaxDPI            *int  `json:"max_dpi,omitempty"`
}

// FontConfig exposes font provider configuration for PDF extraction.
type FontConfig struct {
	Enabled        bool     `json:"enabled"`
	CustomFontDirs []string `json:"custom_font_dirs,omitempty"`
}

// PdfConfig exposes PDF-specific options.
type PdfConfig struct {
	ExtractImages           *bool       `json:"extract_images,omitempty"`
	Passwords               []string    `json:"passwords,omitempty"`
	ExtractMetadata         *bool       `json:"extract_metadata,omitempty"`
	FontConfig              *FontConfig `json:"font_config,omitempty"`
	ExtractAnnotations      *bool       `json:"extract_annotations,omitempty"`
	TopMarginFraction       *float64    `json:"top_margin_fraction,omitempty"`
	BottomMarginFraction    *float64    `json:"bottom_margin_fraction,omitempty"`
	AllowSingleColumnTables *bool       `json:"allow_single_column_tables,omitempty"`
}

// HierarchyConfig controls PDF hierarchy extraction based on font sizes.
type HierarchyConfig struct {
	// Enable hierarchy extraction. Default: true.
	Enabled *bool `json:"enabled,omitempty"`

	// Number of font size clusters (2-10). Default: 6.
	KClusters *int `json:"k_clusters,omitempty"`

	// Include bounding box information. Default: true.
	IncludeBbox *bool `json:"include_bbox,omitempty"`

	// OCR coverage threshold (0.0-1.0). Default: null.
	OcrCoverageThreshold *float64 `json:"ocr_coverage_threshold,omitempty"`
}

// TokenReductionConfig governs token pruning before embeddings.
type TokenReductionConfig struct {
	Mode                   string `json:"mode,omitempty"`
	PreserveImportantWords *bool  `json:"preserve_important_words,omitempty"`
}

// LanguageDetectionConfig enables automatic language detection.
type LanguageDetectionConfig struct {
	Enabled        *bool    `json:"enabled,omitempty"`
	MinConfidence  *float64 `json:"min_confidence,omitempty"`
	DetectMultiple *bool    `json:"detect_multiple,omitempty"`
}

// LayoutDetectionConfig configures ONNX-based document layout detection.
type LayoutDetectionConfig struct {
	Preset              *string  `json:"preset,omitempty"`
	ConfidenceThreshold *float32 `json:"confidence_threshold,omitempty"`
	ApplyHeuristics     *bool    `json:"apply_heuristics,omitempty"`
	TableModel          *string  `json:"table_model,omitempty"`
}

// PostProcessorConfig determines which post processors run.
type PostProcessorConfig struct {
	Enabled            *bool    `json:"enabled,omitempty"`
	EnabledProcessors  []string `json:"enabled_processors,omitempty"`
	DisabledProcessors []string `json:"disabled_processors,omitempty"`
}

// EmbeddingModelType configures embedding model selection.
type EmbeddingModelType struct {
	Type       string `json:"type"`
	Name       string `json:"name,omitempty"`
	Model      string `json:"model,omitempty"`
	ModelID    string `json:"model_id,omitempty"`
	Dimensions *int   `json:"dimensions,omitempty"`
}

// EmbeddingConfig configures embedding generation for chunks.
type EmbeddingConfig struct {
	Model                *EmbeddingModelType `json:"model,omitempty"`
	Normalize            *bool               `json:"normalize,omitempty"`
	BatchSize            *int                `json:"batch_size,omitempty"`
	ShowDownloadProgress *bool               `json:"show_download_progress,omitempty"`
	CacheDir             *string             `json:"cache_dir,omitempty"`
}

// KeywordConfig configures keyword extraction.
type KeywordConfig struct {
	Algorithm   string      `json:"algorithm,omitempty"`
	MaxKeywords *int        `json:"max_keywords,omitempty"`
	MinScore    *float64    `json:"min_score,omitempty"`
	NgramRange  *[2]int     `json:"ngram_range,omitempty"`
	Language    *string     `json:"language,omitempty"`
	Yake        *YakeParams `json:"yake_params,omitempty"`
	Rake        *RakeParams `json:"rake_params,omitempty"`
}

// YakeParams holds YAKE-specific tuning.
type YakeParams struct {
	WindowSize *int `json:"window_size,omitempty"`
}

// RakeParams holds RAKE-specific tuning.
type RakeParams struct {
	MinWordLength     *int `json:"min_word_length,omitempty"`
	MaxWordsPerPhrase *int `json:"max_words_per_phrase,omitempty"`
}

// HTMLPreprocessingOptions configures HTML cleaning.
type HTMLPreprocessingOptions struct {
	Enabled          *bool   `json:"enabled,omitempty"`
	Preset           *string `json:"preset,omitempty"`
	RemoveNavigation *bool   `json:"remove_navigation,omitempty"`
	RemoveForms      *bool   `json:"remove_forms,omitempty"`
}

// HTMLConversionOptions mirrors html_to_markdown_rs::ConversionOptions for HTML-to-Markdown conversion.
type HTMLConversionOptions struct {
	HeadingStyle       *string                   `json:"heading_style,omitempty"`
	ListIndentType     *string                   `json:"list_indent_type,omitempty"`
	ListIndentWidth    *int                      `json:"list_indent_width,omitempty"`
	Bullets            *string                   `json:"bullets,omitempty"`
	StrongEmSymbol     *string                   `json:"strong_em_symbol,omitempty"`
	EscapeAsterisks    *bool                     `json:"escape_asterisks,omitempty"`
	EscapeUnderscores  *bool                     `json:"escape_underscores,omitempty"`
	EscapeMisc         *bool                     `json:"escape_misc,omitempty"`
	EscapeASCII        *bool                     `json:"escape_ascii,omitempty"`
	CodeLanguage       *string                   `json:"code_language,omitempty"`
	Autolinks          *bool                     `json:"autolinks,omitempty"`
	DefaultTitle       *bool                     `json:"default_title,omitempty"`
	BrInTables         *bool                     `json:"br_in_tables,omitempty"`
	HocrSpatialTables  *bool                     `json:"hocr_spatial_tables,omitempty"`
	HighlightStyle     *string                   `json:"highlight_style,omitempty"`
	ExtractMetadata    *bool                     `json:"extract_metadata,omitempty"`
	WhitespaceMode     *string                   `json:"whitespace_mode,omitempty"`
	StripNewlines      *bool                     `json:"strip_newlines,omitempty"`
	Wrap               *bool                     `json:"wrap,omitempty"`
	WrapWidth          *int                      `json:"wrap_width,omitempty"`
	ConvertAsInline    *bool                     `json:"convert_as_inline,omitempty"`
	SubSymbol          *string                   `json:"sub_symbol,omitempty"`
	SupSymbol          *string                   `json:"sup_symbol,omitempty"`
	NewlineStyle       *string                   `json:"newline_style,omitempty"`
	CodeBlockStyle     *string                   `json:"code_block_style,omitempty"`
	KeepInlineImagesIn []string                  `json:"keep_inline_images_in,omitempty"`
	Encoding           *string                   `json:"encoding,omitempty"`
	Debug              *bool                     `json:"debug,omitempty"`
	StripTags          []string                  `json:"strip_tags,omitempty"`
	PreserveTags       []string                  `json:"preserve_tags,omitempty"`
	Preprocessing      *HTMLPreprocessingOptions `json:"preprocessing,omitempty"`
}

// AccelerationConfig controls ONNX Runtime execution provider selection.
type AccelerationConfig struct {
	// Provider selects the execution provider: "auto", "cpu", "coreml", "cuda", "tensorrt".
	Provider string `json:"provider,omitempty"`
	// DeviceID is the GPU device ID (for CUDA/TensorRT).
	DeviceID uint32 `json:"device_id,omitempty"`
}

// EmailConfig controls email extraction settings.
type EmailConfig struct {
	// MsgFallbackCodepage is the fallback code page for MSG email body decoding.
	MsgFallbackCodepage *uint32 `json:"msg_fallback_codepage,omitempty"`
}

// TreeSitterProcessConfig configures tree-sitter code analysis processing options.
type TreeSitterProcessConfig struct {
	Structure    *bool `json:"structure,omitempty"`
	Imports      *bool `json:"imports,omitempty"`
	Exports      *bool `json:"exports,omitempty"`
	Comments     *bool `json:"comments,omitempty"`
	Docstrings   *bool `json:"docstrings,omitempty"`
	Symbols      *bool `json:"symbols,omitempty"`
	Diagnostics  *bool `json:"diagnostics,omitempty"`
	ChunkMaxSize *int  `json:"chunk_max_size,omitempty"`
}

// TreeSitterConfig configures tree-sitter language pack integration.
type TreeSitterConfig struct {
	CacheDir  *string                  `json:"cache_dir,omitempty"`
	Languages []string                 `json:"languages,omitempty"`
	Groups    []string                 `json:"groups,omitempty"`
	Process   *TreeSitterProcessConfig `json:"process,omitempty"`
}

// ConcurrencyConfig controls thread and concurrency limits for constrained environments.
type ConcurrencyConfig struct {
	// MaxThreads sets the maximum number of threads for all internal thread pools.
	MaxThreads *int `json:"max_threads,omitempty"`
}

// PageConfig configures page tracking and extraction.
type PageConfig struct {
	ExtractPages      *bool   `json:"extract_pages,omitempty"`
	InsertPageMarkers *bool   `json:"insert_page_markers,omitempty"`
	MarkerFormat      *string `json:"marker_format,omitempty"`
}

// OutputFormat controls the format of extracted content.
// Options: "plain", "text", "markdown", "md", "djot", "html"
// Default: "plain" (via Rust)
type OutputFormat string

const (
	OutputFormatPlain    OutputFormat = "plain"
	OutputFormatText     OutputFormat = "text" // Alias for plain
	OutputFormatMarkdown OutputFormat = "markdown"
	OutputFormatMd       OutputFormat = "md" // Alias for markdown
	OutputFormatDjot     OutputFormat = "djot"
	OutputFormatHTML     OutputFormat = "html"
)

// ResultFormat controls the result structure.
// Options: "unified", "element_based"
// Default: "unified" (via Rust)
type ResultFormat string

const (
	ResultFormatUnified      ResultFormat = "unified"
	ResultFormatElementBased ResultFormat = "element_based"
)

// FileExtractionConfig provides per-file extraction configuration overrides for batch processing.
// All fields are pointers for optionality — nil means "use the batch-level default."
// Batch-level concerns (caching, concurrency, acceleration, security) are excluded.
type FileExtractionConfig struct {
	EnableQualityProcessing  *bool                    `json:"enable_quality_processing,omitempty"`
	Ocr                      *OCRConfig               `json:"ocr,omitempty"`
	ForceOcr                 *bool                    `json:"force_ocr,omitempty"`
	DisableOcr               *bool                    `json:"disable_ocr,omitempty"`
	ForceOcrPages            []uint64                 `json:"force_ocr_pages,omitempty"`
	Chunking                 *ChunkingConfig          `json:"chunking,omitempty"`
	Images                   *ImageExtractionConfig   `json:"images,omitempty"`
	PdfOptions               *PdfConfig               `json:"pdf_options,omitempty"`
	TokenReduction           *TokenReductionConfig    `json:"token_reduction,omitempty"`
	LanguageDetection        *LanguageDetectionConfig `json:"language_detection,omitempty"`
	Pages                    *PageConfig              `json:"pages,omitempty"`
	Keywords                 *KeywordConfig           `json:"keywords,omitempty"`
	Postprocessor            *PostProcessorConfig     `json:"postprocessor,omitempty"`
	HTMLOptions              *HTMLConversionOptions   `json:"html_options,omitempty"`
	Layout                   *LayoutDetectionConfig   `json:"layout,omitempty"`
	IncludeDocumentStructure *bool                    `json:"include_document_structure,omitempty"`
	OutputFormat             string                   `json:"output_format,omitempty"`
	ResultFormat             string                   `json:"result_format,omitempty"`
	TimeoutSecs              *uint64                  `json:"timeout_secs,omitempty"`
}

// FileItem represents a file path paired with an optional per-file extraction config override.
type FileItem struct {
	Path   string
	Config *FileExtractionConfig
}

// BytesItem represents in-memory document data with MIME type and optional per-file config override.
type BytesItem struct {
	Data     []byte
	MimeType string
	Config   *FileExtractionConfig
}

// ---------------------------------------------------------------------------
// Tree-sitter ProcessResult types (serialized from Rust via serde)
// ---------------------------------------------------------------------------

// CodeSpan represents a byte/line/column range in source code.
type CodeSpan struct {
	StartByte   int `json:"start_byte"`
	EndByte     int `json:"end_byte"`
	StartLine   int `json:"start_line"`
	StartColumn int `json:"start_column"`
	EndLine     int `json:"end_line"`
	EndColumn   int `json:"end_column"`
}

// CodeFileMetrics holds aggregate metrics for a parsed source file.
type CodeFileMetrics struct {
	TotalLines   int `json:"total_lines"`
	CodeLines    int `json:"code_lines"`
	CommentLines int `json:"comment_lines"`
	BlankLines   int `json:"blank_lines"`
	TotalBytes   int `json:"total_bytes"`
	NodeCount    int `json:"node_count"`
	ErrorCount   int `json:"error_count"`
	MaxDepth     int `json:"max_depth"`
}

// CodeStructureItem represents a structural element (function, class, etc.) in source code.
type CodeStructureItem struct {
	Kind       string              `json:"kind"`
	Name       *string             `json:"name,omitempty"`
	Visibility *string             `json:"visibility,omitempty"`
	Span       CodeSpan            `json:"span"`
	Children   []CodeStructureItem `json:"children"`
	Decorators []string            `json:"decorators"`
	DocComment *string             `json:"doc_comment,omitempty"`
	Signature  *string             `json:"signature,omitempty"`
	BodySpan   *CodeSpan           `json:"body_span,omitempty"`
}

// CodeImportInfo describes a single import/include/require statement.
type CodeImportInfo struct {
	Source     string   `json:"source"`
	Items      []string `json:"items"`
	Alias      *string  `json:"alias,omitempty"`
	IsWildcard bool     `json:"is_wildcard"`
	Span       CodeSpan `json:"span"`
}

// CodeExportInfo describes a single exported symbol.
type CodeExportInfo struct {
	Name string   `json:"name"`
	Kind string   `json:"kind"`
	Span CodeSpan `json:"span"`
}

// CodeSymbolInfo describes a symbol (variable, constant, type alias, etc.).
type CodeSymbolInfo struct {
	Name           string   `json:"name"`
	Kind           string   `json:"kind"`
	TypeAnnotation *string  `json:"type_annotation,omitempty"`
	Span           CodeSpan `json:"span"`
}

// CodeCommentInfo describes a single comment in source code.
type CodeCommentInfo struct {
	Text string   `json:"text"`
	Kind string   `json:"kind"`
	Span CodeSpan `json:"span"`
}

// CodeDocSection represents a section within a docstring (e.g., @param, @returns).
type CodeDocSection struct {
	Kind    string  `json:"kind"`
	Name    *string `json:"name,omitempty"`
	Content string  `json:"content"`
}

// CodeDocstringInfo describes a documentation comment/docstring.
type CodeDocstringInfo struct {
	Text           string           `json:"text"`
	Format         string           `json:"format"`
	AssociatedItem *string          `json:"associated_item,omitempty"`
	Span           CodeSpan         `json:"span"`
	Sections       []CodeDocSection `json:"sections"`
}

// CodeDiagnostic represents a parse error or warning from tree-sitter.
type CodeDiagnostic struct {
	Message  string   `json:"message"`
	Severity string   `json:"severity"`
	Span     CodeSpan `json:"span"`
}

// CodeChunkContext provides parent context for a code chunk.
type CodeChunkContext struct {
	ParentName *string `json:"parent_name,omitempty"`
	ParentKind *string `json:"parent_kind,omitempty"`
}

// CodeChunk represents a chunk of source code with optional context.
type CodeChunk struct {
	Content  string            `json:"content"`
	Language string            `json:"language"`
	Span     CodeSpan          `json:"span"`
	Context  *CodeChunkContext `json:"context,omitempty"`
}

// CodeProcessResult is the complete result of tree-sitter code analysis.
type CodeProcessResult struct {
	Language    string              `json:"language"`
	Metrics     CodeFileMetrics     `json:"metrics"`
	Structure   []CodeStructureItem `json:"structure"`
	Imports     []CodeImportInfo    `json:"imports"`
	Exports     []CodeExportInfo    `json:"exports"`
	Comments    []CodeCommentInfo   `json:"comments"`
	Docstrings  []CodeDocstringInfo `json:"docstrings"`
	Symbols     []CodeSymbolInfo    `json:"symbols"`
	Diagnostics []CodeDiagnostic    `json:"diagnostics"`
	Chunks      []CodeChunk         `json:"chunks"`
}

// ---------------------------------------------------------------------------
// Format-specific metadata types (deserialized from FormatMetadata variants)
// ---------------------------------------------------------------------------

// CsvMetadata holds CSV/TSV file metadata.
type CsvMetadata struct {
	RowCount    int      `json:"row_count"`
	ColumnCount int      `json:"column_count"`
	Delimiter   *string  `json:"delimiter,omitempty"`
	HasHeader   bool     `json:"has_header"`
	ColumnTypes []string `json:"column_types,omitempty"`
}

// YearRange represents a year range for bibliographic metadata.
type YearRange struct {
	Min   *int  `json:"min,omitempty"`
	Max   *int  `json:"max,omitempty"`
	Years []int `json:"years,omitempty"`
}

// BibtexMetadata holds BibTeX bibliography metadata.
type BibtexMetadata struct {
	EntryCount   int            `json:"entry_count"`
	CitationKeys []string       `json:"citation_keys,omitempty"`
	Authors      []string       `json:"authors,omitempty"`
	YearRange    *YearRange     `json:"year_range,omitempty"`
	EntryTypes   map[string]int `json:"entry_types,omitempty"`
}

// CitationMetadata holds citation file metadata (RIS, PubMed, EndNote).
type CitationMetadata struct {
	CitationCount int        `json:"citation_count"`
	Format        *string    `json:"format,omitempty"`
	Authors       []string   `json:"authors,omitempty"`
	YearRange     *YearRange `json:"year_range,omitempty"`
	Dois          []string   `json:"dois,omitempty"`
	Keywords      []string   `json:"keywords,omitempty"`
}

// FictionBookMetadata holds FictionBook (FB2) metadata.
type FictionBookMetadata struct {
	Genres     []string `json:"genres,omitempty"`
	Sequences  []string `json:"sequences,omitempty"`
	Annotation *string  `json:"annotation,omitempty"`
}

// DbfFieldInfo describes a dBASE field.
type DbfFieldInfo struct {
	Name      string `json:"name"`
	FieldType string `json:"field_type"`
}

// DbfMetadata holds dBASE (DBF) file metadata.
type DbfMetadata struct {
	RecordCount int            `json:"record_count"`
	FieldCount  int            `json:"field_count"`
	Fields      []DbfFieldInfo `json:"fields,omitempty"`
}

// ContributorRole represents a JATS contributor with role.
type ContributorRole struct {
	Name string  `json:"name"`
	Role *string `json:"role,omitempty"`
}

// JatsMetadata holds JATS (Journal Article Tag Suite) metadata.
type JatsMetadata struct {
	Copyright        *string           `json:"copyright,omitempty"`
	License          *string           `json:"license,omitempty"`
	HistoryDates     map[string]string `json:"history_dates,omitempty"`
	ContributorRoles []ContributorRole `json:"contributor_roles,omitempty"`
}

// EpubMetadata holds EPUB metadata (Dublin Core extensions).
type EpubMetadata struct {
	Coverage   *string `json:"coverage,omitempty"`
	DcFormat   *string `json:"dc_format,omitempty"`
	Relation   *string `json:"relation,omitempty"`
	Source     *string `json:"source,omitempty"`
	DcType     *string `json:"dc_type,omitempty"`
	CoverImage *string `json:"cover_image,omitempty"`
}

// PstMetadata holds Outlook PST archive metadata.
type PstMetadata struct {
	MessageCount int `json:"message_count"`
}

// PptxMetadata holds PowerPoint presentation metadata.
type PptxMetadata struct {
	SlideCount int      `json:"slide_count"`
	SlideNames []string `json:"slide_names"`
	ImageCount *int     `json:"image_count,omitempty"`
	TableCount *int     `json:"table_count,omitempty"`
}
