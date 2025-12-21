package kreuzberg

/*
#include "internal/ffi/kreuzberg.h"
#include <stdlib.h>
#include <stdint.h>

// Extraction API function declarations
const char *kreuzberg_last_error(void);
int32_t kreuzberg_last_error_code(void);
char *kreuzberg_last_panic_context(void);
const char *kreuzberg_version(void);
void kreuzberg_free_string(char *ptr);
void kreuzberg_free_result(CExtractionResult *result);
void kreuzberg_free_batch_result(CBatchResult *batch);
CExtractionResult *kreuzberg_extract_file_sync(const char *path);
CExtractionResult *kreuzberg_extract_file_sync_with_config(const char *path, const char *config_json);
CExtractionResult *kreuzberg_extract_bytes_sync(const uint8_t *data, uintptr_t data_len, const char *mime_type);
CExtractionResult *kreuzberg_extract_bytes_sync_with_config(const uint8_t *data, uintptr_t data_len, const char *mime_type, const char *config_json);
CBatchResult *kreuzberg_batch_extract_files_sync(const char * const *paths, uintptr_t count, const char *config_json);
CBatchResult *kreuzberg_batch_extract_bytes_sync(const CBytesWithMime *items, uintptr_t count, const char *config_json);
char *kreuzberg_detect_mime_type_from_bytes(const uint8_t *data, uintptr_t data_len);
char *kreuzberg_detect_mime_type_from_path(const char *path);
char *kreuzberg_get_extensions_for_mime(const char *mime_type);
char *kreuzberg_validate_mime_type(const char *mime_type);
char *kreuzberg_load_extraction_config_from_file(const char *path);
char *kreuzberg_list_embedding_presets(void);
char *kreuzberg_get_embedding_preset(const char *name);

// Validation FFI functions
int32_t kreuzberg_validate_binarization_method(const char *method);
int32_t kreuzberg_validate_ocr_backend(const char *backend);
int32_t kreuzberg_validate_language_code(const char *code);
int32_t kreuzberg_validate_token_reduction_level(const char *level);
int32_t kreuzberg_validate_tesseract_psm(int32_t psm);
int32_t kreuzberg_validate_tesseract_oem(int32_t oem);
int32_t kreuzberg_validate_output_format(const char *format);
int32_t kreuzberg_validate_confidence(double confidence);
int32_t kreuzberg_validate_dpi(int32_t dpi);
int32_t kreuzberg_validate_chunking_params(uintptr_t max_chars, uintptr_t max_overlap);

// List validation functions
char *kreuzberg_get_valid_binarization_methods(void);
char *kreuzberg_get_valid_language_codes(void);
char *kreuzberg_get_valid_ocr_backends(void);
char *kreuzberg_get_valid_token_reduction_levels(void);

// Phase 1 Configuration FFI functions
ExtractionConfig *kreuzberg_config_from_json(const char *json_config);
void kreuzberg_config_free(ExtractionConfig *config);
int32_t kreuzberg_config_is_valid(const char *json_config);
char *kreuzberg_config_to_json(const ExtractionConfig *config);
char *kreuzberg_config_get_field(const ExtractionConfig *config, const char *field_name);
int32_t kreuzberg_config_merge(ExtractionConfig *base, const ExtractionConfig *override_config);

// Phase 1 Result Accessor FFI functions
int32_t kreuzberg_result_get_page_count(const CExtractionResult *result);
int32_t kreuzberg_result_get_chunk_count(const CExtractionResult *result);
char *kreuzberg_result_get_detected_language(const CExtractionResult *result);
CMetadataField kreuzberg_result_get_metadata_field(const CExtractionResult *result, const char *field_name);

// Phase 2 Error Classification FFI functions
uint32_t kreuzberg_error_code_validation(void);
uint32_t kreuzberg_error_code_parsing(void);
uint32_t kreuzberg_error_code_ocr(void);
uint32_t kreuzberg_error_code_missing_dependency(void);
uint32_t kreuzberg_error_code_io(void);
uint32_t kreuzberg_error_code_plugin(void);
uint32_t kreuzberg_error_code_unsupported_format(void);
uint32_t kreuzberg_error_code_internal(void);
uint32_t kreuzberg_error_code_count(void);
const char *kreuzberg_error_code_name(uint32_t code);
const char *kreuzberg_error_code_description(uint32_t code);
*/
import "C"

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"unsafe"
)

// BytesWithMime represents an in-memory document and its MIME type.
type BytesWithMime struct {
	Data     []byte
	MimeType string
}

// ExtractFileSync extracts content and metadata from the file at the provided path.
func ExtractFileSync(path string, config *ExtractionConfig) (*ExtractionResult, error) {
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	var cRes *C.CExtractionResult
	if cfgPtr != nil {
		cRes = C.kreuzberg_extract_file_sync_with_config(cPath, cfgPtr)
	} else {
		cRes = C.kreuzberg_extract_file_sync(cPath)
	}

	if cRes == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_result(cRes)

	return convertCResult(cRes)
}

// ExtractBytesSync extracts content and metadata from a byte array with the given MIME type.
func ExtractBytesSync(data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error) {
	if len(data) == 0 {
		return nil, newValidationError("data cannot be empty", nil)
	}
	if mimeType == "" {
		return nil, newValidationError("mimeType is required", nil)
	}

	buf := C.CBytes(data)
	defer C.free(buf)

	cMime := C.CString(mimeType)
	defer C.free(unsafe.Pointer(cMime))

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	var cRes *C.CExtractionResult
	if cfgPtr != nil {
		cRes = C.kreuzberg_extract_bytes_sync_with_config((*C.uint8_t)(buf), C.uintptr_t(len(data)), cMime, cfgPtr)
	} else {
		cRes = C.kreuzberg_extract_bytes_sync((*C.uint8_t)(buf), C.uintptr_t(len(data)), cMime)
	}

	if cRes == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_result(cRes)

	return convertCResult(cRes)
}

// BatchExtractFilesSync extracts multiple files sequentially but leverages the optimized batch pipeline.
func BatchExtractFilesSync(paths []string, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if len(paths) == 0 {
		return []*ExtractionResult{}, nil
	}

	cStrings := make([]*C.char, len(paths))
	for i, path := range paths {
		if path == "" {
			return nil, newValidationError(fmt.Sprintf("path at index %d is empty", i), nil)
		}
		cStrings[i] = C.CString(path)
	}
	defer func() {
		for _, ptr := range cStrings {
			C.free(unsafe.Pointer(ptr))
		}
	}()

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	batch := C.kreuzberg_batch_extract_files_sync((**C.char)(unsafe.Pointer(&cStrings[0])), C.uintptr_t(len(paths)), cfgPtr)
	if batch == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_batch_result(batch)

	return convertCBatchResult(batch)
}

// BatchExtractBytesSync processes multiple in-memory documents in one pass.
func BatchExtractBytesSync(items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if len(items) == 0 {
		return []*ExtractionResult{}, nil
	}

	cItems := make([]C.CBytesWithMime, len(items))
	cBuffers := make([]unsafe.Pointer, len(items))

	for i, item := range items {
		if len(item.Data) == 0 {
			return nil, newValidationError(fmt.Sprintf("data at index %d is empty", i), nil)
		}
		if item.MimeType == "" {
			return nil, newValidationError(fmt.Sprintf("mimeType at index %d is empty", i), nil)
		}
		buf := C.CBytes(item.Data)
		cBuffers[i] = buf
		mime := C.CString(item.MimeType)

		cItems[i] = C.CBytesWithMime{
			data:      (*C.uint8_t)(buf),
			data_len:  C.uintptr_t(len(item.Data)),
			mime_type: mime,
		}
	}
	defer func() {
		for i := range cItems {
			if cItems[i].mime_type != nil {
				C.free(unsafe.Pointer(cItems[i].mime_type))
			}
		}
		for _, buf := range cBuffers {
			C.free(buf)
		}
	}()

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	batch := C.kreuzberg_batch_extract_bytes_sync((*C.CBytesWithMime)(unsafe.Pointer(&cItems[0])), C.uintptr_t(len(items)), cfgPtr)
	if batch == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_batch_result(batch)

	return convertCBatchResult(batch)
}

// ExtractFileWithContext extracts content and metadata from a file at the given path,
// respecting the provided context for cancellation. Note that extraction operations
// cannot be interrupted mid-way; this cancellation check occurs before starting extraction.
func ExtractFileWithContext(ctx context.Context, path string, config *ExtractionConfig) (*ExtractionResult, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	return ExtractFileSync(path, config)
}

// ExtractBytesWithContext extracts content and metadata from a byte array,
// respecting the provided context for cancellation. Note that extraction operations
// cannot be interrupted mid-way; this cancellation check occurs before starting extraction.
func ExtractBytesWithContext(ctx context.Context, data []byte, mimeType string, config *ExtractionConfig) (*ExtractionResult, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	return ExtractBytesSync(data, mimeType, config)
}

// BatchExtractFilesWithContext extracts multiple files respecting the provided context
// for cancellation. Note that extraction operations cannot be interrupted mid-way;
// this cancellation check occurs before starting the batch operation.
func BatchExtractFilesWithContext(ctx context.Context, paths []string, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	return BatchExtractFilesSync(paths, config)
}

// BatchExtractBytesWithContext processes multiple in-memory documents respecting the
// provided context for cancellation. Note that extraction operations cannot be
// interrupted mid-way; this cancellation check occurs before starting the batch operation.
func BatchExtractBytesWithContext(ctx context.Context, items []BytesWithMime, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	return BatchExtractBytesSync(items, config)
}

// LibraryVersion returns the underlying Rust crate version string.
func LibraryVersion() string {
	return C.GoString(C.kreuzberg_version())
}

// LastErrorCode returns the error code from the last FFI call.
// Returns 0 (Success) if no error occurred.
func LastErrorCode() ErrorCode {
	return ErrorCode(C.kreuzberg_last_error_code())
}

// LastPanicContext returns the panic context from the last FFI call if it was a panic.
// Returns nil if the last error was not a panic or if no panic context is available.
func LastPanicContext() *PanicContext {
	panicPtr := C.kreuzberg_last_panic_context()
	if panicPtr == nil {
		return nil
	}
	defer C.kreuzberg_free_string(panicPtr)

	panicJSON := C.GoString(panicPtr)
	if panicJSON == "" {
		return nil
	}

	var ctx PanicContext
	if err := json.Unmarshal([]byte(panicJSON), &ctx); err != nil {
		return nil
	}
	return &ctx
}

func convertCResult(cRes *C.CExtractionResult) (*ExtractionResult, error) {
	result := &ExtractionResult{
		Content:  C.GoString(cRes.content),
		MimeType: C.GoString(cRes.mime_type),
		Success:  bool(cRes.success),
	}

	if err := decodeJSONCString(cRes.tables_json, &result.Tables); err != nil {
		return nil, newSerializationError("failed to decode tables", err)
	}

	if err := decodeJSONCString(cRes.detected_languages_json, &result.DetectedLanguages); err != nil {
		return nil, newSerializationError("failed to decode detected languages", err)
	}

	if err := decodeJSONCString(cRes.metadata_json, &result.Metadata); err != nil {
		return nil, newSerializationError("failed to decode metadata", err)
	}

	if result.Metadata.Language == nil && cRes.language != nil {
		if lang := C.GoString(cRes.language); lang != "" {
			result.Metadata.Language = stringPtr(lang)
		}
	}
	if result.Metadata.Date == nil && cRes.date != nil {
		if date := C.GoString(cRes.date); date != "" {
			result.Metadata.Date = stringPtr(date)
		}
	}
	if result.Metadata.Subject == nil && cRes.subject != nil {
		if subj := C.GoString(cRes.subject); subj != "" {
			result.Metadata.Subject = stringPtr(subj)
		}
	}

	if err := decodeJSONCString(cRes.chunks_json, &result.Chunks); err != nil {
		return nil, newSerializationError("failed to decode chunks", err)
	}

	if err := decodeJSONCString(cRes.images_json, &result.Images); err != nil {
		return nil, newSerializationError("failed to decode images", err)
	}

	return result, nil
}

func convertCBatchResult(cBatch *C.CBatchResult) ([]*ExtractionResult, error) {
	count := int(cBatch.count)
	results := make([]*ExtractionResult, 0, count)
	if count == 0 {
		return results, nil
	}

	slice := unsafe.Slice(cBatch.results, count)
	for _, ptr := range slice {
		if ptr == nil {
			results = append(results, nil)
			continue
		}
		res, err := convertCResult(ptr)
		if err != nil {
			return nil, err
		}
		results = append(results, res)
	}
	return results, nil
}

func decodeJSONCString[T any](ptr *C.char, target *T) error {
	if ptr == nil {
		return nil
	}
	raw := C.GoString(ptr)
	if raw == "" {
		return nil
	}
	return json.Unmarshal([]byte(raw), target)
}

func newConfigJSON(config *ExtractionConfig) (*C.char, func(), error) {
	if config == nil {
		return nil, nil, nil
	}
	data, err := json.Marshal(config)
	if err != nil {
		return nil, nil, newSerializationError("failed to encode config", err)
	}
	if len(data) == 0 {
		return nil, nil, nil
	}
	cStr := C.CString(string(data))
	cleanup := func() {
		C.free(unsafe.Pointer(cStr))
	}
	return cStr, cleanup, nil
}

func lastError() error {
	errPtr := C.kreuzberg_last_error()
	if errPtr == nil {
		return newRuntimeError("unknown error", nil)
	}

	errMsg := C.GoString(errPtr)
	code := ErrorCode(C.kreuzberg_last_error_code())

	// Check for panic context regardless of error code
	var panicCtx *PanicContext
	panicPtr := C.kreuzberg_last_panic_context()
	if panicPtr != nil {
		defer C.kreuzberg_free_string(panicPtr)
		panicJSON := C.GoString(panicPtr)
		if panicJSON != "" {
			var ctx PanicContext
			if err := json.Unmarshal([]byte(panicJSON), &ctx); err == nil {
				panicCtx = &ctx
			}
		}
	}

	return classifyNativeError(errMsg, code, panicCtx)
}

func stringPtr(value string) *string {
	if value == "" {
		return nil
	}
	v := value
	return &v
}

// LoadExtractionConfigFromFile parses a TOML/YAML/JSON config file into an ExtractionConfig.
func LoadExtractionConfigFromFile(path string) (*ExtractionConfig, error) {
	if path == "" {
		return nil, newValidationError("config path cannot be empty", nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ptr := C.kreuzberg_load_extraction_config_from_file(cPath)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	raw := C.GoString(ptr)
	cfg := &ExtractionConfig{}
	if err := json.Unmarshal([]byte(raw), cfg); err != nil {
		return nil, newSerializationError("failed to decode config JSON", err)
	}
	return cfg, nil
}

// ConfigFromFile loads an ExtractionConfig from a file (alias for LoadExtractionConfigFromFile).
func ConfigFromFile(path string) (*ExtractionConfig, error) {
	return LoadExtractionConfigFromFile(path)
}

// ConfigDiscover searches parent directories for a config file and loads it.
// Returns nil without error if no config file is found.
func ConfigDiscover() (*ExtractionConfig, error) {

	configNames := []string{"kreuzberg.toml", "kreuzberg.yaml", "kreuzberg.yml", "kreuzberg.json"}

	currentDir, err := os.Getwd()
	if err != nil {
		return nil, newIOError("failed to get current directory", err)
	}

	dir := currentDir
	for {
		for _, name := range configNames {
			configPath := filepath.Join(dir, name)
			if _, err := os.Stat(configPath); err == nil {
				return LoadExtractionConfigFromFile(configPath)
			}
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}

	return nil, nil
}

// DetectMimeType detects MIME type from byte content using magic bytes.
func DetectMimeType(data []byte) (string, error) {
	if len(data) == 0 {
		return "", newValidationError("data cannot be empty", nil)
	}

	buf := C.CBytes(data)
	defer C.free(buf)

	ptr := C.kreuzberg_detect_mime_type_from_bytes((*C.uint8_t)(buf), C.uintptr_t(len(data)))
	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	return C.GoString(ptr), nil
}

// DetectMimeTypeFromPath detects MIME type from a file path (checks extension and content).
func DetectMimeTypeFromPath(path string) (string, error) {
	if path == "" {
		return "", newValidationError("path cannot be empty", nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ptr := C.kreuzberg_detect_mime_type_from_path(cPath)
	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	return C.GoString(ptr), nil
}

// GetExtensionsForMime returns file extensions associated with a MIME type.
func GetExtensionsForMime(mimeType string) ([]string, error) {
	if mimeType == "" {
		return nil, newValidationError("mimeType cannot be empty", nil)
	}

	cMime := C.CString(mimeType)
	defer C.free(unsafe.Pointer(cMime))

	ptr := C.kreuzberg_get_extensions_for_mime(cMime)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var extensions []string
	if err := json.Unmarshal([]byte(jsonStr), &extensions); err != nil {
		return nil, newSerializationError("failed to parse extensions list", err)
	}
	return extensions, nil
}

// ValidateMimeType validates that the given MIME type is supported.
func ValidateMimeType(mimeType string) (string, error) {
	if mimeType == "" {
		return "", newValidationError("mimeType cannot be empty", nil)
	}

	cMime := C.CString(mimeType)
	defer C.free(unsafe.Pointer(cMime))

	ptr := C.kreuzberg_validate_mime_type(cMime)
	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	return C.GoString(ptr), nil
}

// EmbeddingPreset describes a built-in embedding preset.
type EmbeddingPreset struct {
	Name        string `json:"name"`
	ChunkSize   int    `json:"chunk_size"`
	Overlap     int    `json:"overlap"`
	ModelName   string `json:"model_name"`
	Dimensions  int    `json:"dimensions"`
	Description string `json:"description"`
}

// ListEmbeddingPresets returns available embedding preset names.
func ListEmbeddingPresets() ([]string, error) {
	ptr := C.kreuzberg_list_embedding_presets()
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	raw := C.GoString(ptr)
	if raw == "" {
		return []string{}, nil
	}
	var names []string
	if err := json.Unmarshal([]byte(raw), &names); err != nil {
		return nil, newSerializationError("failed to decode preset names", err)
	}
	return names, nil
}

// GetEmbeddingPreset returns preset metadata by name.
func GetEmbeddingPreset(name string) (*EmbeddingPreset, error) {
	if name == "" {
		return nil, newValidationError("preset name cannot be empty", nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	ptr := C.kreuzberg_get_embedding_preset(cName)
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	var preset EmbeddingPreset
	if err := json.Unmarshal([]byte(C.GoString(ptr)), &preset); err != nil {
		return nil, newSerializationError("failed to decode embedding preset", err)
	}
	return &preset, nil
}
