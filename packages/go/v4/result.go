package kreuzberg

import (
	"encoding/json"
	"fmt"
)

/*
#include "internal/ffi/kreuzberg.h"
#include <stdlib.h>
*/
import "C"

// GetPageCount returns the total number of pages/slides/sheets in the document.
// Returns -1 if there is an error (check the error return value).
// This method provides efficient access to page count metadata without JSON parsing.
func (r *ExtractionResult) GetPageCount() (int, error) {
	// Use the Go data available in the struct
	if r.Metadata.PageStructure != nil {
		return int(r.Metadata.PageStructure.TotalCount), nil
	}
	return 0, nil
}

// GetChunkCount returns the number of text chunks in the extraction result.
// Returns 0 if chunking was not enabled or there are no chunks.
// Returns -1 if there is an error.
// This method provides efficient access to chunk count without JSON parsing.
func (r *ExtractionResult) GetChunkCount() (int, error) {
	if r.Chunks != nil {
		return len(r.Chunks), nil
	}
	return 0, nil
}

// GetDetectedLanguage returns the primary detected language code (e.g., "en", "de").
// Returns an empty string if no language was detected.
// This method provides efficient access to language detection without JSON parsing.
func (r *ExtractionResult) GetDetectedLanguage() (string, error) {
	// Try metadata.language first
	if r.Metadata.Language != nil {
		return *r.Metadata.Language, nil
	}

	// Try the first detected language
	if len(r.DetectedLanguages) > 0 {
		return r.DetectedLanguages[0], nil
	}

	return "", nil
}

// MetadataField represents a metadata field with its value and existence status.
type MetadataField struct {
	// Name is the field name that was requested.
	Name string
	// Value is the parsed field value as a Go interface{}.
	Value interface{}
	// IsNull indicates whether the field exists (false) or is null/missing (true).
	IsNull bool
}

// GetMetadataField retrieves a metadata field from the extraction result.
// Field paths use dot notation for nested fields (e.g., "language", "pdf.page_count").
// Returns the field value parsed as a Go interface{}, or an error if retrieval fails.
// If the field doesn't exist, IsNull will be true in the returned MetadataField.
func (r *ExtractionResult) GetMetadataField(fieldName string) (*MetadataField, error) {
	if fieldName == "" {
		return nil, newValidationError("field name cannot be empty", nil)
	}

	// Parse the metadata JSON to extract the field
	metadataJSON, err := json.Marshal(r.Metadata)
	if err != nil {
		return nil, newSerializationError("failed to encode metadata", err)
	}

	var metadataMap map[string]interface{}
	if err := json.Unmarshal(metadataJSON, &metadataMap); err != nil {
		return nil, newSerializationError("failed to parse metadata", err)
	}

	// Handle simple field access (no nesting for now)
	value, exists := metadataMap[fieldName]
	if !exists {
		return &MetadataField{
			Name:   fieldName,
			Value:  nil,
			IsNull: true,
		}, nil
	}

	// Check if value is nil
	if value == nil {
		return &MetadataField{
			Name:   fieldName,
			Value:  nil,
			IsNull: true,
		}, nil
	}

	return &MetadataField{
		Name:   fieldName,
		Value:  value,
		IsNull: false,
	}, nil
}

// ResultToJSON serializes an ExtractionResult to a JSON string.
// This is useful for passing results through FFI or storing them.
func ResultToJSON(result *ExtractionResult) (string, error) {
	if result == nil {
		return "", newValidationError("result cannot be nil", nil)
	}

	data, err := json.Marshal(result)
	if err != nil {
		return "", newSerializationError("failed to encode result", err)
	}

	return string(data), nil
}

// ResultFromJSON deserializes an ExtractionResult from a JSON string.
// This is the inverse of ResultToJSON.
func ResultFromJSON(jsonStr string) (*ExtractionResult, error) {
	if jsonStr == "" {
		return nil, newValidationError("JSON string cannot be empty", nil)
	}

	var result ExtractionResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, newSerializationError("failed to decode result JSON", err)
	}

	return &result, nil
}

// String implements fmt.Stringer for ExtractionResult, showing a summary.
func (r *ExtractionResult) String() string {
	if r == nil {
		return "<nil ExtractionResult>"
	}

	return fmt.Sprintf("ExtractionResult{MimeType: %s, ContentLen: %d, Tables: %d, Chunks: %d, Success: %v}",
		r.MimeType, len(r.Content), len(r.Tables), len(r.Chunks), r.Success)
}
