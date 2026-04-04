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
CBatchResult *kreuzberg_batch_extract_files_sync(const char * const *file_paths, const char * const *file_config_jsons, uintptr_t count, const char *config_json);
CBatchResult *kreuzberg_batch_extract_bytes_sync(const CBytesWithMime *items, const char * const *file_config_jsons, uintptr_t count, const char *config_json);
char *kreuzberg_detect_mime_type_from_bytes(const uint8_t *data, uintptr_t data_len);
char *kreuzberg_detect_mime_type_from_path(const char *path);
char *kreuzberg_get_extensions_for_mime(const char *mime_type);
char *kreuzberg_validate_mime_type(const char *mime_type);
char *kreuzberg_load_extraction_config_from_file(const char *path);
char *kreuzberg_list_embedding_presets(void);
char *kreuzberg_get_embedding_preset(const char *name);
char *kreuzberg_embed(const char *texts_json, const char *config_json);

// PDF rendering and iterator function declarations
CPageImage *kreuzberg_render_pdf_page(const char *file_path, uintptr_t page_index, int32_t dpi);
void kreuzberg_free_render_page_result(CPageImage *page);
CPdfPageIterator *kreuzberg_pdf_page_iterator_new(const char *file_path, int32_t dpi);
CPageIterResult *kreuzberg_pdf_page_iterator_next(CPdfPageIterator *iter);
uintptr_t kreuzberg_pdf_page_iterator_page_count(const CPdfPageIterator *iter);
void kreuzberg_pdf_page_iterator_free(CPdfPageIterator *iter);
void kreuzberg_pdf_page_iterator_free_result(CPageIterResult *result);

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

// Phase 2 Error Classification FFI functions
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
	"sync"
	"unsafe"
)

// ffiMutex serializes all FFI calls to prevent concurrent access to PDFium.
// PDFium is not thread-safe, and concurrent calls from multiple goroutines
// cause signal stack crashes on macOS (SIGTRAP) and other platforms.
var ffiMutex sync.Mutex

// BytesWithMime represents an in-memory document and its MIME type.
type BytesWithMime struct {
	Data     []byte
	MimeType string
}

// ExtractFileSync extracts content and metadata from the file at the provided path.
func ExtractFileSync(path string, config *ExtractionConfig) (*ExtractionResult, error) {
	// Validate path is not empty
	if path == "" {
		return nil, newValidationErrorWithContext("path is required", nil, ErrorCodeValidation, nil)
	}

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

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
	if mimeType == "" {
		return nil, newValidationErrorWithContext("mimeType is required", nil, ErrorCodeValidation, nil)
	}

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
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

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

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

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
	}

	cStrings := make([]*C.char, len(paths))
	for i, path := range paths {
		if path == "" {
			return nil, newValidationErrorWithContext(fmt.Sprintf("path at index %d is empty", i), nil, ErrorCodeValidation, nil)
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

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	batch := C.kreuzberg_batch_extract_files_sync((**C.char)(unsafe.Pointer(&cStrings[0])), nil, C.uintptr_t(len(paths)), cfgPtr)
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

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
	}

	cItems := make([]C.CBytesWithMime, len(items))
	cBuffers := make([]unsafe.Pointer, len(items))

	for i, item := range items {
		if len(item.Data) == 0 {
			return nil, newValidationErrorWithContext(fmt.Sprintf("data at index %d is empty", i), nil, ErrorCodeValidation, nil)
		}
		if item.MimeType == "" {
			return nil, newValidationErrorWithContext(fmt.Sprintf("mimeType at index %d is empty", i), nil, ErrorCodeValidation, nil)
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

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	batch := C.kreuzberg_batch_extract_bytes_sync((*C.CBytesWithMime)(unsafe.Pointer(&cItems[0])), nil, C.uintptr_t(len(items)), cfgPtr)
	if batch == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_batch_result(batch)

	return convertCBatchResult(batch)
}

// RenderPdfPage renders a single page of a PDF as a PNG image.
func RenderPdfPage(path string, pageIndex int, dpi int) ([]byte, error) {
	if path == "" {
		return nil, newValidationErrorWithContext("path is required", nil, ErrorCodeValidation, nil)
	}
	if pageIndex < 0 {
		return nil, newValidationErrorWithContext("pageIndex must be non-negative", nil, ErrorCodeValidation, nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	cResult := C.kreuzberg_render_pdf_page(cPath, C.uintptr_t(pageIndex), C.int32_t(dpi))
	if cResult == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_render_page_result(cResult)

	goSlice := unsafe.Slice((*byte)(unsafe.Pointer(cResult.data)), cResult.len)
	result := make([]byte, cResult.len)
	copy(result, goSlice)
	return result, nil
}

// PdfPageIterator provides lazy, one-page-at-a-time rendering of a PDF file.
// It wraps the native kreuzberg_pdf_page_iterator FFI functions.
// Callers must call Close when done to free native resources.
type PdfPageIterator struct {
	handle unsafe.Pointer
	closed bool
}

// NewPdfPageIterator creates a new iterator over all pages of the PDF at path.
// Each call to Next renders the next page as PNG bytes.
// The caller must call Close when iteration is complete.
func NewPdfPageIterator(path string, dpi int) (*PdfPageIterator, error) {
	if path == "" {
		return nil, newValidationErrorWithContext("path is required", nil, ErrorCodeValidation, nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	handle := C.kreuzberg_pdf_page_iterator_new(cPath, C.int32_t(dpi))
	if handle == nil {
		return nil, lastError()
	}

	return &PdfPageIterator{handle: unsafe.Pointer(handle), closed: false}, nil
}

// Next advances the iterator and returns the next page's PNG bytes.
// pageIndex is the zero-based index of the returned page.
// ok is false when the iterator is exhausted or the iterator is closed.
func (it *PdfPageIterator) Next() (pageIndex int, png []byte, ok bool, err error) {
	if it.closed || it.handle == nil {
		return 0, nil, false, nil
	}

	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	cResult := C.kreuzberg_pdf_page_iterator_next((*C.CPdfPageIterator)(it.handle))
	if cResult == nil {
		// NULL means exhausted or error. The FFI clears the error before
		// each call, so a non-NULL kreuzberg_last_error() indicates failure.
		errPtr := C.kreuzberg_last_error()
		if errPtr != nil {
			return 0, nil, false, lastError()
		}
		return 0, nil, false, nil
	}
	defer C.kreuzberg_pdf_page_iterator_free_result(cResult)

	goSlice := unsafe.Slice((*byte)(unsafe.Pointer(cResult.data)), cResult.len)
	pngBytes := make([]byte, cResult.len)
	copy(pngBytes, goSlice)
	return int(cResult.page_index), pngBytes, true, nil
}

// PageCount returns the total number of pages in the PDF.
func (it *PdfPageIterator) PageCount() int {
	if it.closed || it.handle == nil {
		return 0
	}

	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	return int(C.kreuzberg_pdf_page_iterator_page_count((*C.CPdfPageIterator)(it.handle)))
}

// Close frees the native iterator resources. It is safe to call multiple times.
func (it *PdfPageIterator) Close() {
	if it.closed || it.handle == nil {
		return
	}
	it.closed = true

	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	C.kreuzberg_pdf_page_iterator_free((*C.CPdfPageIterator)(it.handle))
	it.handle = nil
}

// BatchExtractFilesWithConfigs extracts multiple files with per-file configuration overrides.
// Each FileItem pairs a path with an optional FileExtractionConfig that overrides the batch-level config.
func BatchExtractFilesWithConfigs(items []FileItem, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if len(items) == 0 {
		return []*ExtractionResult{}, nil
	}

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
	}

	cPaths := make([]*C.char, len(items))
	cFileConfigs := make([]*C.char, len(items))
	var fileConfigCleanups []func()

	for i, item := range items {
		if item.Path == "" {
			return nil, newValidationErrorWithContext(fmt.Sprintf("path at index %d is empty", i), nil, ErrorCodeValidation, nil)
		}
		cPaths[i] = C.CString(item.Path)

		if item.Config != nil {
			data, err := json.Marshal(item.Config)
			if err != nil {
				return nil, newSerializationErrorWithContext(fmt.Sprintf("failed to encode file config at index %d", i), err, ErrorCodeValidation, nil)
			}
			cStr := C.CString(string(data))
			cFileConfigs[i] = cStr
			fileConfigCleanups = append(fileConfigCleanups, func() {
				C.free(unsafe.Pointer(cStr))
			})
		}
		// nil Config leaves cFileConfigs[i] as nil (NULL)
	}
	defer func() {
		for _, ptr := range cPaths {
			C.free(unsafe.Pointer(ptr))
		}
		for _, cleanup := range fileConfigCleanups {
			cleanup()
		}
	}()

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	batch := C.kreuzberg_batch_extract_files_sync(
		(**C.char)(unsafe.Pointer(&cPaths[0])),
		(**C.char)(unsafe.Pointer(&cFileConfigs[0])),
		C.uintptr_t(len(items)),
		cfgPtr,
	)
	if batch == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_batch_result(batch)

	return convertCBatchResult(batch)
}

// BatchExtractBytesWithConfigs processes multiple in-memory documents with per-file configuration overrides.
// Each BytesItem pairs document data and MIME type with an optional FileExtractionConfig.
func BatchExtractBytesWithConfigs(items []BytesItem, config *ExtractionConfig) ([]*ExtractionResult, error) {
	if len(items) == 0 {
		return []*ExtractionResult{}, nil
	}

	// Validate chunking parameters if provided in config
	if config != nil && config.Chunking != nil {
		if err := validateChunkingConfig(config.Chunking); err != nil {
			return nil, err
		}
	}

	cItems := make([]C.CBytesWithMime, len(items))
	cBuffers := make([]unsafe.Pointer, len(items))
	cFileConfigs := make([]*C.char, len(items))
	var fileConfigCleanups []func()

	for i, item := range items {
		if len(item.Data) == 0 {
			return nil, newValidationErrorWithContext(fmt.Sprintf("data at index %d is empty", i), nil, ErrorCodeValidation, nil)
		}
		if item.MimeType == "" {
			return nil, newValidationErrorWithContext(fmt.Sprintf("mimeType at index %d is empty", i), nil, ErrorCodeValidation, nil)
		}
		buf := C.CBytes(item.Data)
		cBuffers[i] = buf
		mime := C.CString(item.MimeType)

		cItems[i] = C.CBytesWithMime{
			data:      (*C.uint8_t)(buf),
			data_len:  C.uintptr_t(len(item.Data)),
			mime_type: mime,
		}

		if item.Config != nil {
			data, err := json.Marshal(item.Config)
			if err != nil {
				return nil, newSerializationErrorWithContext(fmt.Sprintf("failed to encode file config at index %d", i), err, ErrorCodeValidation, nil)
			}
			cStr := C.CString(string(data))
			cFileConfigs[i] = cStr
			fileConfigCleanups = append(fileConfigCleanups, func() {
				C.free(unsafe.Pointer(cStr))
			})
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
		for _, cleanup := range fileConfigCleanups {
			cleanup()
		}
	}()

	cfgPtr, cfgCleanup, err := newConfigJSON(config)
	if err != nil {
		return nil, err
	}
	if cfgCleanup != nil {
		defer cfgCleanup()
	}

	// Serialize FFI calls to prevent concurrent PDFium access
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

	batch := C.kreuzberg_batch_extract_bytes_sync(
		(*C.CBytesWithMime)(unsafe.Pointer(&cItems[0])),
		(**C.char)(unsafe.Pointer(&cFileConfigs[0])),
		C.uintptr_t(len(items)),
		cfgPtr,
	)
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
	ffiMutex.Lock()
	defer ffiMutex.Unlock()
	return C.GoString(C.kreuzberg_version())
}

// LastErrorCode returns the error code from the last FFI call.
// Returns 0 (Success) if no error occurred.
func LastErrorCode() ErrorCode {
	ffiMutex.Lock()
	defer ffiMutex.Unlock()
	return ErrorCode(C.kreuzberg_last_error_code())
}

// LastPanicContext returns the panic context from the last FFI call if it was a panic.
// Returns nil if the last error was not a panic or if no panic context is available.
func LastPanicContext() *PanicContext {
	ffiMutex.Lock()
	defer ffiMutex.Unlock()

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
	}

	if err := decodeJSONCString(cRes.tables_json, &result.Tables); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode tables", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.detected_languages_json, &result.DetectedLanguages); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode detected languages", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.metadata_json, &result.Metadata); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode metadata", err, ErrorCodeValidation, nil)
	}

	if result.Metadata.Language == nil && cRes.language != nil {
		if lang := C.GoString(cRes.language); lang != "" {
			result.Metadata.Language = stringPtr(lang)
		}
	}
	if result.Metadata.Subject == nil && cRes.subject != nil {
		if subj := C.GoString(cRes.subject); subj != "" {
			result.Metadata.Subject = stringPtr(subj)
		}
	}

	if err := decodeJSONCString(cRes.chunks_json, &result.Chunks); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode chunks", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.children_json, &result.Children); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode children", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.images_json, &result.Images); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode images", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.pages_json, &result.Pages); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode pages", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.elements_json, &result.Elements); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode elements", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.ocr_elements_json, &result.OcrElements); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode ocr elements", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.document_json, &result.Document); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode document structure", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.extracted_keywords_json, &result.ExtractedKeywords); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode extracted keywords", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.quality_score_json, &result.QualityScore); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode quality score", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.processing_warnings_json, &result.ProcessingWarnings); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode processing warnings", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.annotations_json, &result.Annotations); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode annotations", err, ErrorCodeValidation, nil)
	}

	if err := decodeJSONCString(cRes.uris_json, &result.Uris); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode uris", err, ErrorCodeValidation, nil)
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
		return nil, nil, newSerializationErrorWithContext("failed to encode config", err, ErrorCodeValidation, nil)
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
		return newRuntimeErrorWithContext("unknown error", nil, ErrorCodeInternal, nil)
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
		return nil, newValidationErrorWithContext("config path cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ffiMutex.Lock()
	ptr := C.kreuzberg_load_extraction_config_from_file(cPath)
	ffiMutex.Unlock()

	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	raw := C.GoString(ptr)
	cfg := &ExtractionConfig{}
	if err := json.Unmarshal([]byte(raw), cfg); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode config JSON", err, ErrorCodeValidation, nil)
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
		return nil, newIOErrorWithContext("failed to get current directory", err, ErrorCodeIo, nil)
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
		return "", newValidationErrorWithContext("data cannot be empty", nil, ErrorCodeValidation, nil)
	}

	buf := C.CBytes(data)
	defer C.free(buf)

	ffiMutex.Lock()
	ptr := C.kreuzberg_detect_mime_type_from_bytes((*C.uint8_t)(buf), C.uintptr_t(len(data)))
	ffiMutex.Unlock()

	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	return C.GoString(ptr), nil
}

// DetectMimeTypeFromPath detects MIME type from a file path (checks extension and content).
func DetectMimeTypeFromPath(path string) (string, error) {
	if path == "" {
		return "", newValidationErrorWithContext("path cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	ffiMutex.Lock()
	ptr := C.kreuzberg_detect_mime_type_from_path(cPath)
	ffiMutex.Unlock()

	if ptr == nil {
		return "", lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	return C.GoString(ptr), nil
}

// GetExtensionsForMime returns file extensions associated with a MIME type.
func GetExtensionsForMime(mimeType string) ([]string, error) {
	if mimeType == "" {
		return nil, newValidationErrorWithContext("mimeType cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cMime := C.CString(mimeType)
	defer C.free(unsafe.Pointer(cMime))

	ffiMutex.Lock()
	ptr := C.kreuzberg_get_extensions_for_mime(cMime)
	ffiMutex.Unlock()

	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var extensions []string
	if err := json.Unmarshal([]byte(jsonStr), &extensions); err != nil {
		return nil, newSerializationErrorWithContext("failed to parse extensions list", err, ErrorCodeValidation, nil)
	}
	return extensions, nil
}

// ValidateMimeType validates that the given MIME type is supported.
func ValidateMimeType(mimeType string) (string, error) {
	if mimeType == "" {
		return "", newValidationErrorWithContext("mimeType cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cMime := C.CString(mimeType)
	defer C.free(unsafe.Pointer(cMime))

	ffiMutex.Lock()
	ptr := C.kreuzberg_validate_mime_type(cMime)
	ffiMutex.Unlock()

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
	ffiMutex.Lock()
	ptr := C.kreuzberg_list_embedding_presets()
	ffiMutex.Unlock()

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
		return nil, newSerializationErrorWithContext("failed to decode preset names", err, ErrorCodeValidation, nil)
	}
	return names, nil
}

// GetEmbeddingPreset returns preset metadata by name.
func GetEmbeddingPreset(name string) (*EmbeddingPreset, error) {
	if name == "" {
		return nil, newValidationErrorWithContext("preset name cannot be empty", nil, ErrorCodeValidation, nil)
	}

	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))

	ffiMutex.Lock()
	ptr := C.kreuzberg_get_embedding_preset(cName)
	ffiMutex.Unlock()

	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	var preset EmbeddingPreset
	if err := json.Unmarshal([]byte(C.GoString(ptr)), &preset); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode embedding preset", err, ErrorCodeValidation, nil)
	}
	return &preset, nil
}

// EmbedTexts generates vector embeddings for the given texts synchronously.
//
// Requires the embeddings feature to be enabled (ONNX Runtime must be available).
// Returns one float32 slice per input text. An empty input returns an empty slice.
func EmbedTexts(texts []string, config *EmbeddingConfig) ([][]float32, error) {
	textsJSON, err := json.Marshal(texts)
	if err != nil {
		return nil, newSerializationErrorWithContext("failed to serialize texts", err, ErrorCodeValidation, nil)
	}
	cTexts := C.CString(string(textsJSON))
	defer C.free(unsafe.Pointer(cTexts))

	var cConfig *C.char
	if config != nil {
		configJSON, err := json.Marshal(config)
		if err != nil {
			return nil, newSerializationErrorWithContext("failed to serialize embedding config", err, ErrorCodeValidation, nil)
		}
		cConfig = C.CString(string(configJSON))
		defer C.free(unsafe.Pointer(cConfig))
	}

	ffiMutex.Lock()
	ptr := C.kreuzberg_embed(cTexts, cConfig)
	ffiMutex.Unlock()

	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	var embeddings [][]float32
	if err := json.Unmarshal([]byte(C.GoString(ptr)), &embeddings); err != nil {
		return nil, newSerializationErrorWithContext("failed to decode embeddings", err, ErrorCodeValidation, nil)
	}
	return embeddings, nil
}

// EmbedTextsAsync generates vector embeddings for the given texts asynchronously.
//
// The embedding generation is offloaded to a goroutine; the call blocks until
// the embeddings are ready or the context is canceled.
func EmbedTextsAsync(ctx context.Context, texts []string, config *EmbeddingConfig) ([][]float32, error) {
	type embedResult struct {
		embeddings [][]float32
		err        error
	}
	ch := make(chan embedResult, 1)
	go func() {
		embeddings, err := EmbedTexts(texts, config)
		ch <- embedResult{embeddings, err}
	}()
	select {
	case <-ctx.Done():
		return nil, ctx.Err()
	case r := <-ch:
		return r.embeddings, r.err
	}
}

// validateChunkingConfig validates chunking configuration parameters.
// It checks that ChunkSize and ChunkOverlap are positive when set, and that overlap < chunk size.
// These validations are performed before FFI calls.
func validateChunkingConfig(cfg *ChunkingConfig) error {
	// Maximum reasonable chunk size (100MB)
	const maxReasonableChunkSize = 104857600

	// Validate ChunkSize if provided
	if cfg.ChunkSize != nil {
		if *cfg.ChunkSize < 0 {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid chunk size: %d (must be >= 0)", *cfg.ChunkSize),
				nil, ErrorCodeValidation, nil)
		}
		if *cfg.ChunkSize > maxReasonableChunkSize {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid chunk size: %d (exceeds maximum reasonable size of %d bytes)", *cfg.ChunkSize, maxReasonableChunkSize),
				nil, ErrorCodeValidation, nil)
		}
	}

	// Validate ChunkOverlap if provided
	if cfg.ChunkOverlap != nil && *cfg.ChunkOverlap < 0 {
		return newValidationErrorWithContext(
			fmt.Sprintf("invalid chunk overlap: %d (must be >= 0)", *cfg.ChunkOverlap),
			nil, ErrorCodeValidation, nil)
	}

	// If both are set, validate that overlap < chunk size
	if cfg.ChunkSize != nil && cfg.ChunkOverlap != nil {
		if *cfg.ChunkOverlap >= *cfg.ChunkSize {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid chunking parameters: chunk overlap (%d) must be < chunk size (%d)", *cfg.ChunkOverlap, *cfg.ChunkSize),
				nil, ErrorCodeValidation, nil)
		}
	}

	// Also validate MaxChars and MaxOverlap if provided (for backward compatibility)
	if cfg.MaxChars != nil {
		if *cfg.MaxChars <= 0 {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid max_chars: %d (must be > 0)", *cfg.MaxChars),
				nil, ErrorCodeValidation, nil)
		}
		if *cfg.MaxChars > maxReasonableChunkSize {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid max_chars: %d (exceeds maximum reasonable size of %d bytes)", *cfg.MaxChars, maxReasonableChunkSize),
				nil, ErrorCodeValidation, nil)
		}
	}

	if cfg.MaxOverlap != nil && *cfg.MaxOverlap < 0 {
		return newValidationErrorWithContext(
			fmt.Sprintf("invalid max_overlap: %d (must be >= 0)", *cfg.MaxOverlap),
			nil, ErrorCodeValidation, nil)
	}

	// If both MaxChars and MaxOverlap are set, validate that overlap < max_chars
	if cfg.MaxChars != nil && cfg.MaxOverlap != nil {
		if *cfg.MaxOverlap >= *cfg.MaxChars {
			return newValidationErrorWithContext(
				fmt.Sprintf("invalid chunking parameters: max_overlap (%d) must be < max_chars (%d)", *cfg.MaxOverlap, *cfg.MaxChars),
				nil, ErrorCodeValidation, nil)
		}
	}

	return nil
}
