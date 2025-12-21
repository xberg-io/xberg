package kreuzberg

import (
	"encoding/json"
	"fmt"
	"unsafe"
)

/*
#include "internal/ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

// BoolPtr returns a pointer to a bool value. Useful for initializing nullable config fields.
func BoolPtr(b bool) *bool {
	return &b
}

// StringPtr returns a pointer to a string value. Useful for initializing nullable config fields.
func StringPtr(s string) *string {
	return &s
}

// IntPtr returns a pointer to an int value. Useful for initializing nullable config fields.
func IntPtr(i int) *int {
	return &i
}

// FloatPtr returns a pointer to a float64 value. Useful for initializing nullable config fields.
func FloatPtr(f float64) *float64 {
	return &f
}

// ExtractionConfig mirrors the Rust ExtractionConfig structure and is serialized to JSON
// before crossing the FFI boundary. Use pointer fields to omit values and rely on Kreuzberg
// defaults whenever possible.
type ExtractionConfig struct {
	// UseCache enables caching of extraction results for identical inputs.
	UseCache *bool `json:"use_cache,omitempty"`
	// EnableQualityProcessing applies quality improvements like deskewing and denoising.
	EnableQualityProcessing *bool `json:"enable_quality_processing,omitempty"`
	// OCR configures optical character recognition settings.
	OCR *OCRConfig `json:"ocr,omitempty"`
	// ForceOCR forces OCR processing even for text-based documents.
	ForceOCR *bool `json:"force_ocr,omitempty"`
	// Chunking configures text chunking for RAG/retrieval workflows.
	Chunking *ChunkingConfig `json:"chunking,omitempty"`
	// Images configures image extraction from documents.
	Images *ImageExtractionConfig `json:"images,omitempty"`
	// PdfOptions contains PDF-specific extraction settings.
	PdfOptions *PdfConfig `json:"pdf_options,omitempty"`
	// TokenReduction configures token pruning before embeddings.
	TokenReduction *TokenReductionConfig `json:"token_reduction,omitempty"`
	// LanguageDetection enables automatic language detection.
	LanguageDetection *LanguageDetectionConfig `json:"language_detection,omitempty"`
	// Keywords configures keyword extraction.
	Keywords *KeywordConfig `json:"keywords,omitempty"`
	// Postprocessor configures post-processing steps.
	Postprocessor *PostProcessorConfig `json:"postprocessor,omitempty"`
	// HTMLOptions configures HTML-to-Markdown conversion options.
	HTMLOptions *HTMLConversionOptions `json:"html_options,omitempty"`
	// Pages configures page-level extraction and tracking.
	Pages *PageConfig `json:"pages,omitempty"`
	// MaxConcurrentExtractions limits the number of concurrent extraction operations.
	MaxConcurrentExtractions *int `json:"max_concurrent_extractions,omitempty"`
}

// OCRConfig selects and configures OCR backends.
type OCRConfig struct {
	// Backend selects the OCR backend (e.g., "tesseract", "easyocr").
	Backend string `json:"backend,omitempty"`
	// Language specifies the language for OCR (e.g., "eng", "deu").
	Language *string `json:"language,omitempty"`
	// Tesseract contains Tesseract-specific configuration options.
	Tesseract *TesseractConfig `json:"tesseract_config,omitempty"`
}

// TesseractConfig exposes fine-grained controls for the Tesseract backend.
type TesseractConfig struct {
	// Language is the ISO 639 language code for OCR (e.g., "eng", "deu").
	Language string `json:"language,omitempty"`
	// PSM is the Page Segmentation Mode (0-13); see Tesseract documentation.
	PSM *int `json:"psm,omitempty"`
	// OutputFormat specifies the output format (e.g., "text", "hocr").
	OutputFormat string `json:"output_format,omitempty"`
	// OEM is the OCR Engine Mode (0-3); see Tesseract documentation.
	OEM *int `json:"oem,omitempty"`
	// MinConfidence is the minimum confidence threshold (0.0-1.0) for accepting text.
	MinConfidence *float64 `json:"min_confidence,omitempty"`
	// Preprocessing configures image preprocessing (DPI, rotation, etc.).
	Preprocessing *ImagePreprocessingConfig `json:"preprocessing,omitempty"`
	// EnableTableDetection enables automatic table detection during OCR.
	EnableTableDetection *bool `json:"enable_table_detection,omitempty"`
	// TableMinConfidence is the minimum confidence for table detection (0.0-1.0).
	TableMinConfidence *float64 `json:"table_min_confidence,omitempty"`
	// TableColumnThreshold is the pixel threshold for detecting table columns.
	TableColumnThreshold *int `json:"table_column_threshold,omitempty"`
	// TableRowThresholdRatio is the ratio threshold for detecting table rows.
	TableRowThresholdRatio *float64 `json:"table_row_threshold_ratio,omitempty"`
	// UseCache enables Tesseract result caching.
	UseCache *bool `json:"use_cache,omitempty"`
	// ClassifyUsePreAdaptedTemplates uses pre-adapted classifier templates.
	ClassifyUsePreAdaptedTemplates *bool `json:"classify_use_pre_adapted_templates,omitempty"`
	// LanguageModelNgramOn enables language model n-gram processing.
	LanguageModelNgramOn *bool `json:"language_model_ngram_on,omitempty"`
	// TesseditDontBlkrejGoodWds prevents rejection of good words in block mode.
	TesseditDontBlkrejGoodWds *bool `json:"tessedit_dont_blkrej_good_wds,omitempty"`
	// TesseditDontRowrejGoodWds prevents rejection of good words in row mode.
	TesseditDontRowrejGoodWds *bool `json:"tessedit_dont_rowrej_good_wds,omitempty"`
	// TesseditEnableDictCorrection enables dictionary-based word correction.
	TesseditEnableDictCorrection *bool `json:"tessedit_enable_dict_correction,omitempty"`
	// TesseditCharWhitelist specifies characters to allow (empty = all allowed).
	TesseditCharWhitelist string `json:"tessedit_char_whitelist,omitempty"`
	// TesseditCharBlacklist specifies characters to reject.
	TesseditCharBlacklist string `json:"tessedit_char_blacklist,omitempty"`
	// TesseditUsePrimaryParamsModel uses the primary model parameters.
	TesseditUsePrimaryParamsModel *bool `json:"tessedit_use_primary_params_model,omitempty"`
	// TextordSpaceSizeIsVariable allows variable spacing in text ordering.
	TextordSpaceSizeIsVariable *bool `json:"textord_space_size_is_variable,omitempty"`
	// ThresholdingMethod selects the image thresholding method.
	ThresholdingMethod *bool `json:"thresholding_method,omitempty"`
}

// ImagePreprocessingConfig tunes DPI normalization and related steps for OCR.
type ImagePreprocessingConfig struct {
	// TargetDPI sets the target DPI for image normalization (typically 300).
	TargetDPI *int `json:"target_dpi,omitempty"`
	// AutoRotate automatically rotates images to correct orientation.
	AutoRotate *bool `json:"auto_rotate,omitempty"`
	// Deskew applies skew correction to images.
	Deskew *bool `json:"deskew,omitempty"`
	// Denoise applies noise reduction to images.
	Denoise *bool `json:"denoise,omitempty"`
	// ContrastEnhance enhances image contrast.
	ContrastEnhance *bool `json:"contrast_enhance,omitempty"`
	// BinarizationMode selects the image binarization method.
	BinarizationMode string `json:"binarization_method,omitempty"`
	// InvertColors inverts black and white in images.
	InvertColors *bool `json:"invert_colors,omitempty"`
}

// ChunkingConfig configures text chunking for downstream RAG/Retrieval workloads.
type ChunkingConfig struct {
	// MaxChars is the maximum number of characters per chunk.
	MaxChars *int `json:"max_chars,omitempty"`
	// MaxOverlap is the maximum overlap between chunks in characters.
	MaxOverlap *int `json:"max_overlap,omitempty"`
	// ChunkSize is the target chunk size in characters.
	ChunkSize *int `json:"chunk_size,omitempty"`
	// ChunkOverlap is the number of overlapping characters between chunks.
	ChunkOverlap *int `json:"chunk_overlap,omitempty"`
	// Preset selects a predefined chunking strategy (e.g., "default", "semantic").
	Preset *string `json:"preset,omitempty"`
	// Embedding configures embedding generation for chunks.
	Embedding *EmbeddingConfig `json:"embedding,omitempty"`
	// Enabled enables or disables chunking.
	Enabled *bool `json:"enabled,omitempty"`
}

// ImageExtractionConfig controls inline image extraction from PDFs/Office docs.
type ImageExtractionConfig struct {
	// ExtractImages enables image extraction from documents.
	ExtractImages *bool `json:"extract_images,omitempty"`
	// TargetDPI sets the target DPI for extracted images.
	TargetDPI *int `json:"target_dpi,omitempty"`
	// MaxImageDimension limits the maximum width or height of extracted images.
	MaxImageDimension *int `json:"max_image_dimension,omitempty"`
	// AutoAdjustDPI automatically adjusts DPI based on image content.
	AutoAdjustDPI *bool `json:"auto_adjust_dpi,omitempty"`
	// MinDPI is the minimum DPI for extracted images.
	MinDPI *int `json:"min_dpi,omitempty"`
	// MaxDPI is the maximum DPI for extracted images.
	MaxDPI *int `json:"max_dpi,omitempty"`
}

// PdfConfig exposes PDF-specific options.
type PdfConfig struct {
	// ExtractImages enables image extraction from PDFs.
	ExtractImages *bool `json:"extract_images,omitempty"`
	// Passwords provides password(s) for encrypted PDFs (tried in order).
	Passwords []string `json:"passwords,omitempty"`
	// ExtractMetadata enables extraction of PDF metadata.
	ExtractMetadata *bool `json:"extract_metadata,omitempty"`
}

// TokenReductionConfig governs token pruning before embeddings.
type TokenReductionConfig struct {
	// Mode selects the token reduction strategy (e.g., "aggressive", "conservative").
	Mode string `json:"mode,omitempty"`
	// PreserveImportantWords preserves semantically important words during reduction.
	PreserveImportantWords *bool `json:"preserve_important_words,omitempty"`
}

// LanguageDetectionConfig enables automatic language detection.
type LanguageDetectionConfig struct {
	// Enabled enables automatic language detection.
	Enabled *bool `json:"enabled,omitempty"`
	// MinConfidence is the minimum confidence threshold (0.0-1.0) for language detection.
	MinConfidence *float64 `json:"min_confidence,omitempty"`
	// DetectMultiple enables detection of multiple languages in the document.
	DetectMultiple *bool `json:"detect_multiple,omitempty"`
}

// PostProcessorConfig determines which post processors run.
type PostProcessorConfig struct {
	// Enabled enables post-processing.
	Enabled *bool `json:"enabled,omitempty"`
	// EnabledProcessors lists specific processors to enable (overrides defaults).
	EnabledProcessors []string `json:"enabled_processors,omitempty"`
	// DisabledProcessors lists specific processors to disable.
	DisabledProcessors []string `json:"disabled_processors,omitempty"`
}

// EmbeddingModelType configures embedding model selection.
type EmbeddingModelType struct {
	// Type selects the embedding model type (e.g., "sentence-transformers", "openai").
	Type string `json:"type"`
	// Name is the name of the embedding model.
	Name string `json:"name,omitempty"`
	// Model is the model identifier or path.
	Model string `json:"model,omitempty"`
	// ModelID is an alternative model identifier.
	ModelID string `json:"model_id,omitempty"`
	// Dimensions is the embedding vector dimensionality.
	Dimensions *int `json:"dimensions,omitempty"`
}

// EmbeddingConfig configures embedding generation for chunks.
type EmbeddingConfig struct {
	// Model specifies the embedding model to use.
	Model *EmbeddingModelType `json:"model,omitempty"`
	// Normalize normalizes embedding vectors to unit length.
	Normalize *bool `json:"normalize,omitempty"`
	// BatchSize is the batch size for embedding generation.
	BatchSize *int `json:"batch_size,omitempty"`
	// ShowDownloadProgress shows progress when downloading embedding models.
	ShowDownloadProgress *bool `json:"show_download_progress,omitempty"`
	// CacheDir is the directory for caching embedding models.
	CacheDir *string `json:"cache_dir,omitempty"`
}

// KeywordConfig configures keyword extraction.
type KeywordConfig struct {
	// Algorithm selects the keyword extraction algorithm (e.g., "yake", "rake").
	Algorithm string `json:"algorithm,omitempty"`
	// MaxKeywords limits the maximum number of keywords to extract.
	MaxKeywords *int `json:"max_keywords,omitempty"`
	// MinScore is the minimum score threshold for keyword candidates (0.0-1.0).
	MinScore *float64 `json:"min_score,omitempty"`
	// NgramRange specifies the [min, max] n-gram size for keyword extraction.
	NgramRange *[2]int `json:"ngram_range,omitempty"`
	// Language is the language for keyword extraction (e.g., "en", "de").
	Language *string `json:"language,omitempty"`
	// Yake contains YAKE-specific parameters.
	Yake *YakeParams `json:"yake_params,omitempty"`
	// Rake contains RAKE-specific parameters.
	Rake *RakeParams `json:"rake_params,omitempty"`
}

// YakeParams holds YAKE-specific tuning.
type YakeParams struct {
	// WindowSize is the context window size for YAKE extraction.
	WindowSize *int `json:"window_size,omitempty"`
}

// RakeParams holds RAKE-specific tuning.
type RakeParams struct {
	// MinWordLength is the minimum word length for RAKE extraction.
	MinWordLength *int `json:"min_word_length,omitempty"`
	// MaxWordsPerPhrase is the maximum number of words per phrase.
	MaxWordsPerPhrase *int `json:"max_words_per_phrase,omitempty"`
}

// HTMLPreprocessingOptions configures HTML cleaning.
type HTMLPreprocessingOptions struct {
	// Enabled enables HTML preprocessing.
	Enabled *bool `json:"enabled,omitempty"`
	// Preset selects a preprocessing strategy (e.g., "aggressive", "conservative").
	Preset *string `json:"preset,omitempty"`
	// RemoveNavigation removes navigation elements from HTML.
	RemoveNavigation *bool `json:"remove_navigation,omitempty"`
	// RemoveForms removes form elements from HTML.
	RemoveForms *bool `json:"remove_forms,omitempty"`
}

// HTMLConversionOptions mirrors html_to_markdown_rs::ConversionOptions for HTML-to-Markdown conversion.
type HTMLConversionOptions struct {
	// HeadingStyle specifies Markdown heading style (e.g., "atx", "setext").
	HeadingStyle *string `json:"heading_style,omitempty"`
	// ListIndentType specifies list indentation style (e.g., "tab", "spaces").
	ListIndentType *string `json:"list_indent_type,omitempty"`
	// ListIndentWidth is the number of spaces per indentation level.
	ListIndentWidth *int `json:"list_indent_width,omitempty"`
	// Bullets specifies the bullet character (e.g., "*", "-", "+").
	Bullets *string `json:"bullets,omitempty"`
	// StrongEmSymbol specifies symbols for strong/emphasis (e.g., "*", "_").
	StrongEmSymbol *string `json:"strong_em_symbol,omitempty"`
	// EscapeAsterisks escapes asterisks in the output.
	EscapeAsterisks *bool `json:"escape_asterisks,omitempty"`
	// EscapeUnderscores escapes underscores in the output.
	EscapeUnderscores *bool `json:"escape_underscores,omitempty"`
	// EscapeMisc escapes miscellaneous special characters.
	EscapeMisc *bool `json:"escape_misc,omitempty"`
	// EscapeASCII escapes ASCII special characters.
	EscapeASCII *bool `json:"escape_ascii,omitempty"`
	// CodeLanguage specifies the language for code block syntax highlighting.
	CodeLanguage *string `json:"code_language,omitempty"`
	// Autolinks automatically links URLs.
	Autolinks *bool `json:"autolinks,omitempty"`
	// DefaultTitle uses a default title if none is provided.
	DefaultTitle *bool `json:"default_title,omitempty"`
	// BrInTables preserves <br> tags in tables.
	BrInTables *bool `json:"br_in_tables,omitempty"`
	// HocrSpatialTables uses spatial information for hOCR tables.
	HocrSpatialTables *bool `json:"hocr_spatial_tables,omitempty"`
	// HighlightStyle specifies code highlight style.
	HighlightStyle *string `json:"highlight_style,omitempty"`
	// ExtractMetadata extracts metadata from HTML.
	ExtractMetadata *bool `json:"extract_metadata,omitempty"`
	// WhitespaceMode specifies whitespace handling (e.g., "preserve", "collapse").
	WhitespaceMode *string `json:"whitespace_mode,omitempty"`
	// StripNewlines removes newlines from output.
	StripNewlines *bool `json:"strip_newlines,omitempty"`
	// Wrap enables text wrapping.
	Wrap *bool `json:"wrap,omitempty"`
	// WrapWidth specifies the maximum line width for wrapping.
	WrapWidth *int `json:"wrap_width,omitempty"`
	// ConvertAsInline treats content as inline elements.
	ConvertAsInline *bool `json:"convert_as_inline,omitempty"`
	// SubSymbol specifies the symbol for subscript text.
	SubSymbol *string `json:"sub_symbol,omitempty"`
	// SupSymbol specifies the symbol for superscript text.
	SupSymbol *string `json:"sup_symbol,omitempty"`
	// NewlineStyle specifies newline style (e.g., "unix", "windows").
	NewlineStyle *string `json:"newline_style,omitempty"`
	// CodeBlockStyle specifies code block formatting style.
	CodeBlockStyle *string `json:"code_block_style,omitempty"`
	// KeepInlineImagesIn lists elements to keep inline images in.
	KeepInlineImagesIn []string `json:"keep_inline_images_in,omitempty"`
	// Encoding specifies the character encoding.
	Encoding *string `json:"encoding,omitempty"`
	// Debug enables debug output.
	Debug *bool `json:"debug,omitempty"`
	// StripTags lists HTML tags to remove from output.
	StripTags []string `json:"strip_tags,omitempty"`
	// PreserveTags lists HTML tags to preserve in output.
	PreserveTags []string `json:"preserve_tags,omitempty"`
	// Preprocessing configures HTML preprocessing options.
	Preprocessing *HTMLPreprocessingOptions `json:"preprocessing,omitempty"`
}

// PageConfig configures page tracking and extraction.
type PageConfig struct {
	// ExtractPages enables per-page content extraction.
	ExtractPages *bool `json:"extract_pages,omitempty"`
	// InsertPageMarkers inserts page markers in the extracted content.
	InsertPageMarkers *bool `json:"insert_page_markers,omitempty"`
	// MarkerFormat specifies the format for page markers.
	MarkerFormat *string `json:"marker_format,omitempty"`
}

// ConfigFromJSON parses an ExtractionConfig from a JSON string via FFI.
// This is the primary method for converting JSON to a config structure.
func ConfigFromJSON(jsonStr string) (*ExtractionConfig, error) {
	if jsonStr == "" {
		return nil, newValidationError("JSON string cannot be empty", nil)
	}

	cJSON := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cJSON))

	ptr := C.kreuzberg_config_from_json(cJSON)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_config_free(ptr)

	// Parse the config back from JSON to populate Go struct
	cfg := &ExtractionConfig{}
	if err := json.Unmarshal([]byte(jsonStr), cfg); err != nil {
		return nil, newSerializationError("failed to decode config JSON", err)
	}
	return cfg, nil
}

// IsValidJSON validates a JSON config string without fully parsing it.
// Returns true if the JSON is valid, false otherwise.
func IsValidJSON(jsonStr string) bool {
	if jsonStr == "" {
		return false
	}

	cJSON := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cJSON))

	result := int32(C.kreuzberg_config_is_valid(cJSON))
	return result == 1
}

// ConfigToJSON serializes an ExtractionConfig to a JSON string via FFI.
func ConfigToJSON(config *ExtractionConfig) (string, error) {
	if config == nil {
		return "", newValidationError("config cannot be nil", nil)
	}

	// Serialize to JSON first
	data, err := json.Marshal(config)
	if err != nil {
		return "", newSerializationError("failed to encode config", err)
	}

	// Create a C config from JSON to get the serialized representation
	jsonStr := string(data)
	cJSON := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cJSON))

	ptr := C.kreuzberg_config_from_json(cJSON)
	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_config_free(ptr)

	// Get the serialized form from the FFI
	cSerialized := C.kreuzberg_config_to_json(ptr)
	if cSerialized == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(cSerialized)

	return C.GoString(cSerialized), nil
}

// ConfigGetField retrieves a specific field value from a config.
// Field paths use dot notation for nested fields (e.g., "ocr.backend").
// Returns the field value as a JSON string, or an error if the field doesn't exist.
func ConfigGetField(config *ExtractionConfig, fieldName string) (interface{}, error) {
	if config == nil {
		return nil, newValidationError("config cannot be nil", nil)
	}
	if fieldName == "" {
		return nil, newValidationError("field name cannot be empty", nil)
	}

	// Serialize config to JSON first
	data, err := json.Marshal(config)
	if err != nil {
		return nil, newSerializationError("failed to encode config", err)
	}

	cJSON := C.CString(string(data))
	defer C.free(unsafe.Pointer(cJSON))

	ptr := C.kreuzberg_config_from_json(cJSON)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_config_free(ptr)

	cFieldName := C.CString(fieldName)
	defer C.free(unsafe.Pointer(cFieldName))

	cValue := C.kreuzberg_config_get_field(ptr, cFieldName)
	if cValue == nil {
		return nil, newValidationError(fmt.Sprintf("field not found: %s", fieldName), nil)
	}
	defer C.kreuzberg_free_string(cValue)

	jsonStr := C.GoString(cValue)
	var value interface{}
	if err := json.Unmarshal([]byte(jsonStr), &value); err != nil {
		return nil, newSerializationError("failed to parse field value", err)
	}
	return value, nil
}

// ConfigMerge merges an override config into a base config.
// Non-nil/default fields from override are copied into base.
// Returns an error if the merge fails.
func ConfigMerge(base, override *ExtractionConfig) error {
	if base == nil {
		return newValidationError("base config cannot be nil", nil)
	}
	if override == nil {
		return newValidationError("override config cannot be nil", nil)
	}

	// Serialize both configs to JSON
	baseData, err := json.Marshal(base)
	if err != nil {
		return newSerializationError("failed to encode base config", err)
	}

	overrideData, err := json.Marshal(override)
	if err != nil {
		return newSerializationError("failed to encode override config", err)
	}

	cBaseJSON := C.CString(string(baseData))
	defer C.free(unsafe.Pointer(cBaseJSON))

	cOverrideJSON := C.CString(string(overrideData))
	defer C.free(unsafe.Pointer(cOverrideJSON))

	basePtr := C.kreuzberg_config_from_json(cBaseJSON)
	if basePtr == nil {
		return lastError()
	}
	defer C.kreuzberg_config_free(basePtr)

	overridePtr := C.kreuzberg_config_from_json(cOverrideJSON)
	if overridePtr == nil {
		return lastError()
	}
	defer C.kreuzberg_config_free(overridePtr)

	result := int32(C.kreuzberg_config_merge(basePtr, overridePtr))
	if result != 1 {
		return lastError()
	}

	// Get the merged config back as JSON and update base
	cMerged := C.kreuzberg_config_to_json(basePtr)
	if cMerged == nil {
		return lastError()
	}
	defer C.kreuzberg_free_string(cMerged)

	mergedStr := C.GoString(cMerged)
	if err := json.Unmarshal([]byte(mergedStr), base); err != nil {
		return newSerializationError("failed to decode merged config", err)
	}

	return nil
}
