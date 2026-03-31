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
	if override.Ocr != nil {
		base.Ocr = override.Ocr
	}
	if override.ForceOcr != nil {
		base.ForceOcr = override.ForceOcr
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
	if override.Acceleration != nil {
		base.Acceleration = override.Acceleration
	}
	if override.MaxConcurrentExtractions != nil {
		base.MaxConcurrentExtractions = override.MaxConcurrentExtractions
	}
	if override.OutputFormat != "" {
		base.OutputFormat = override.OutputFormat
	}
	if override.ResultFormat != "" {
		base.ResultFormat = override.ResultFormat
	}

	return nil
}
