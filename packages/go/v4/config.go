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
	UseCache                 *bool                    `json:"use_cache,omitempty"`
	EnableQualityProcessing  *bool                    `json:"enable_quality_processing,omitempty"`
	OCR                      *OCRConfig               `json:"ocr,omitempty"`
	ForceOCR                 *bool                    `json:"force_ocr,omitempty"`
	Chunking                 *ChunkingConfig          `json:"chunking,omitempty"`
	Images                   *ImageExtractionConfig   `json:"images,omitempty"`
	PdfOptions               *PdfConfig               `json:"pdf_options,omitempty"`
	TokenReduction           *TokenReductionConfig    `json:"token_reduction,omitempty"`
	LanguageDetection        *LanguageDetectionConfig `json:"language_detection,omitempty"`
	Keywords                 *KeywordConfig           `json:"keywords,omitempty"`
	Postprocessor            *PostProcessorConfig     `json:"postprocessor,omitempty"`
	HTMLOptions              *HTMLConversionOptions   `json:"html_options,omitempty"`
	Pages                    *PageConfig              `json:"pages,omitempty"`
	MaxConcurrentExtractions *int                     `json:"max_concurrent_extractions,omitempty"`
}

// OCRConfig selects and configures OCR backends.
type OCRConfig struct {
	Backend   string           `json:"backend,omitempty"`
	Language  *string          `json:"language,omitempty"`
	Tesseract *TesseractConfig `json:"tesseract_config,omitempty"`
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

// ChunkingConfig configures text chunking for downstream RAG/Retrieval workloads.
type ChunkingConfig struct {
	MaxChars     *int             `json:"max_chars,omitempty"`
	MaxOverlap   *int             `json:"max_overlap,omitempty"`
	ChunkSize    *int             `json:"chunk_size,omitempty"`
	ChunkOverlap *int             `json:"chunk_overlap,omitempty"`
	Preset       *string          `json:"preset,omitempty"`
	Embedding    *EmbeddingConfig `json:"embedding,omitempty"`
	Enabled      *bool            `json:"enabled,omitempty"`
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
	ExtractImages   *bool       `json:"extract_images,omitempty"`
	Passwords       []string    `json:"passwords,omitempty"`
	ExtractMetadata *bool       `json:"extract_metadata,omitempty"`
	FontConfig      *FontConfig `json:"font_config,omitempty"`
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

// PageConfig configures page tracking and extraction.
type PageConfig struct {
	ExtractPages      *bool   `json:"extract_pages,omitempty"`
	InsertPageMarkers *bool   `json:"insert_page_markers,omitempty"`
	MarkerFormat      *string `json:"marker_format,omitempty"`
}

// ConfigFromJSON parses an ExtractionConfig from a JSON string via FFI.
// This is the primary method for converting JSON to a config structure.
func ConfigFromJSON(jsonStr string) (*ExtractionConfig, error) {
	if jsonStr == "" {
		return nil, newValidationErrorWithContext("JSON string cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cJSON := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cJSON))

	ptr := C.kreuzberg_config_from_json(cJSON)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_config_free(ptr)

	cfg := &ExtractionConfig{}
	if err := json.Unmarshal([]byte(jsonStr), cfg); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode config JSON", err, ErrorCodeValidation, nil)
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
		return "", newValidationErrorWithContext("config cannot be nil", nil, ErrorCodeValidation, nil)
	}

	data, err := json.Marshal(config)
	if err != nil {
		return "", newSerializationErrorWithContext("failed to encode config", err, ErrorCodeValidation, nil)
	}

	jsonStr := string(data)
	cJSON := C.CString(jsonStr)
	defer C.free(unsafe.Pointer(cJSON))

	ptr := C.kreuzberg_config_from_json(cJSON)
	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_config_free(ptr)

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
		return nil, newValidationErrorWithContext("config cannot be nil", nil, ErrorCodeValidation, nil)
	}
	if fieldName == "" {
		return nil, newValidationErrorWithContext("field name cannot be empty", nil, ErrorCodeValidation, nil)
	}

	data, err := json.Marshal(config)
	if err != nil {
		return nil, newSerializationErrorWithContext("failed to encode config", err, ErrorCodeValidation, nil)
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
		return nil, newValidationErrorWithContext(fmt.Sprintf("field not found: %s", fieldName), nil, ErrorCodeValidation, nil)
	}
	defer C.kreuzberg_free_string(cValue)

	jsonStr := C.GoString(cValue)
	var value interface{}
	if err := json.Unmarshal([]byte(jsonStr), &value); err != nil {
		return nil, newSerializationErrorWithContext("failed to parse field value", err, ErrorCodeValidation, nil)
	}
	return value, nil
}

// ConfigMerge merges an override config into a base config.
// Non-nil/default fields from override are copied into base.
// Returns an error if the merge fails.
func ConfigMerge(base, override *ExtractionConfig) error {
	if base == nil {
		return newValidationErrorWithContext("base config cannot be nil", nil, ErrorCodeValidation, nil)
	}
	if override == nil {
		return newValidationErrorWithContext("override config cannot be nil", nil, ErrorCodeValidation, nil)
	}

	if override.UseCache != nil {
		base.UseCache = override.UseCache
	}
	if override.EnableQualityProcessing != nil {
		base.EnableQualityProcessing = override.EnableQualityProcessing
	}
	if override.OCR != nil {
		base.OCR = override.OCR
	}
	if override.ForceOCR != nil {
		base.ForceOCR = override.ForceOCR
	}
	if override.Chunking != nil {
		base.Chunking = override.Chunking
	}
	if override.Images != nil {
		base.Images = override.Images
	}
	if override.PdfOptions != nil {
		base.PdfOptions = override.PdfOptions
	}
	if override.TokenReduction != nil {
		base.TokenReduction = override.TokenReduction
	}
	if override.LanguageDetection != nil {
		base.LanguageDetection = override.LanguageDetection
	}
	if override.Keywords != nil {
		base.Keywords = override.Keywords
	}
	if override.Postprocessor != nil {
		base.Postprocessor = override.Postprocessor
	}
	if override.HTMLOptions != nil {
		base.HTMLOptions = override.HTMLOptions
	}
	if override.Pages != nil {
		base.Pages = override.Pages
	}
	if override.MaxConcurrentExtractions != nil {
		base.MaxConcurrentExtractions = override.MaxConcurrentExtractions
	}

	return nil
}
