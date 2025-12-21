package kreuzberg

/*
#include "internal/ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"fmt"
	"unsafe"
)

// ValidateBinarizationMethod validates a binarization method string via FFI.
// Valid values include "otsu", "adaptive", "sauvola", and others.
func ValidateBinarizationMethod(method string) error {
	if method == "" {
		return newValidationError("binarization method cannot be empty", nil)
	}

	cMethod := C.CString(method)
	defer C.free(unsafe.Pointer(cMethod))

	result := int32(C.kreuzberg_validate_binarization_method(cMethod))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid binarization method: %s", method), nil)
	}
	return nil
}

// ValidateOCRBackend validates an OCR backend string via FFI.
// Valid values include "tesseract", "easyocr", "paddleocr", and others.
func ValidateOCRBackend(backend string) error {
	if backend == "" {
		return newValidationError("OCR backend cannot be empty", nil)
	}

	cBackend := C.CString(backend)
	defer C.free(unsafe.Pointer(cBackend))

	result := int32(C.kreuzberg_validate_ocr_backend(cBackend))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid OCR backend: %s", backend), nil)
	}
	return nil
}

// ValidateLanguageCode validates a language code (ISO 639-1 or 639-3 format) via FFI.
// Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes (e.g., "eng", "deu").
func ValidateLanguageCode(code string) error {
	if code == "" {
		return newValidationError("language code cannot be empty", nil)
	}

	cCode := C.CString(code)
	defer C.free(unsafe.Pointer(cCode))

	result := int32(C.kreuzberg_validate_language_code(cCode))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid language code: %s", code), nil)
	}
	return nil
}

// ValidateTokenReductionLevel validates a token reduction level string via FFI.
// Valid values include "off", "light", "moderate", "aggressive", and others.
func ValidateTokenReductionLevel(level string) error {
	if level == "" {
		return newValidationError("token reduction level cannot be empty", nil)
	}

	cLevel := C.CString(level)
	defer C.free(unsafe.Pointer(cLevel))

	result := int32(C.kreuzberg_validate_token_reduction_level(cLevel))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid token reduction level: %s", level), nil)
	}
	return nil
}

// ValidateTesseractPSM validates a Tesseract Page Segmentation Mode (PSM) value.
// Valid range is 0-13.
func ValidateTesseractPSM(psm int) error {
	result := int32(C.kreuzberg_validate_tesseract_psm(C.int32_t(psm)))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid Tesseract PSM value: %d (valid range: 0-13)", psm), nil)
	}
	return nil
}

// ValidateTesseractOEM validates a Tesseract OCR Engine Mode (OEM) value.
// Valid range is 0-3.
func ValidateTesseractOEM(oem int) error {
	result := int32(C.kreuzberg_validate_tesseract_oem(C.int32_t(oem)))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid Tesseract OEM value: %d (valid range: 0-3)", oem), nil)
	}
	return nil
}

// ValidateOutputFormat validates a Tesseract output format string.
// Valid values include "text", "markdown", "hocr", and others.
func ValidateOutputFormat(format string) error {
	if format == "" {
		return newValidationError("output format cannot be empty", nil)
	}

	cFormat := C.CString(format)
	defer C.free(unsafe.Pointer(cFormat))

	result := int32(C.kreuzberg_validate_output_format(cFormat))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid output format: %s", format), nil)
	}
	return nil
}

// ValidateConfidence validates a confidence threshold value.
// Confidence values must be between 0.0 and 1.0 inclusive.
func ValidateConfidence(confidence float64) error {
	result := int32(C.kreuzberg_validate_confidence(C.double(confidence)))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid confidence threshold: %.2f (must be between 0.0 and 1.0)", confidence), nil)
	}
	return nil
}

// ValidateDPI validates a DPI (dots per inch) value.
// DPI must be a positive integer, typically 72-600.
func ValidateDPI(dpi int) error {
	result := int32(C.kreuzberg_validate_dpi(C.int32_t(dpi)))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid DPI value: %d (must be a positive integer)", dpi), nil)
	}
	return nil
}

// ValidateChunkingParams validates chunking configuration parameters.
// Checks that maxChars > 0 and maxOverlap < maxChars.
func ValidateChunkingParams(maxChars int, maxOverlap int) error {
	if maxChars <= 0 {
		return newValidationError(fmt.Sprintf("invalid max_chars: %d (must be > 0)", maxChars), nil)
	}
	if maxOverlap < 0 {
		return newValidationError(fmt.Sprintf("invalid max_overlap: %d (must be >= 0)", maxOverlap), nil)
	}
	if maxOverlap >= maxChars {
		return newValidationError(fmt.Sprintf("invalid chunking parameters: max_overlap (%d) must be < max_chars (%d)", maxOverlap, maxChars), nil)
	}

	result := int32(C.kreuzberg_validate_chunking_params(C.uintptr_t(maxChars), C.uintptr_t(maxOverlap)))
	if result != 1 {
		return newValidationError(fmt.Sprintf("invalid chunking parameters: max_chars=%d, max_overlap=%d", maxChars, maxOverlap), nil)
	}
	return nil
}

// GetValidBinarizationMethods returns a list of all valid binarization methods.
func GetValidBinarizationMethods() ([]string, error) {
	ptr := C.kreuzberg_get_valid_binarization_methods()
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var methods []string
	if err := json.Unmarshal([]byte(jsonStr), &methods); err != nil {
		return nil, newSerializationError("failed to parse binarization methods list", err)
	}
	return methods, nil
}

// GetValidLanguageCodes returns a list of all valid language codes.
func GetValidLanguageCodes() ([]string, error) {
	ptr := C.kreuzberg_get_valid_language_codes()
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var codes []string
	if err := json.Unmarshal([]byte(jsonStr), &codes); err != nil {
		return nil, newSerializationError("failed to parse language codes list", err)
	}
	return codes, nil
}

// GetValidOCRBackends returns a list of all valid OCR backends.
func GetValidOCRBackends() ([]string, error) {
	ptr := C.kreuzberg_get_valid_ocr_backends()
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var backends []string
	if err := json.Unmarshal([]byte(jsonStr), &backends); err != nil {
		return nil, newSerializationError("failed to parse OCR backends list", err)
	}
	return backends, nil
}

// GetValidTokenReductionLevels returns a list of all valid token reduction levels.
func GetValidTokenReductionLevels() ([]string, error) {
	ptr := C.kreuzberg_get_valid_token_reduction_levels()
	if ptr == nil {
		return nil, lastError()
	}
	defer C.kreuzberg_free_string(ptr)

	jsonStr := C.GoString(ptr)
	var levels []string
	if err := json.Unmarshal([]byte(jsonStr), &levels); err != nil {
		return nil, newSerializationError("failed to parse token reduction levels list", err)
	}
	return levels, nil
}
