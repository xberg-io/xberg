package kreuzberg

/*
#include <stdint.h>

// Phase 2 Error Classification FFI functions
uint32_t kreuzberg_error_code_count(void);
const char *kreuzberg_error_code_name(uint32_t code);
const char *kreuzberg_error_code_description(uint32_t code);
*/
import "C"

import (
	"fmt"
	"strings"
)

// ErrorKind identifies the category of a Kreuzberg error.
type ErrorKind string

const (
	ErrorKindUnknown           ErrorKind = "unknown"
	ErrorKindIO                ErrorKind = "io"
	ErrorKindValidation        ErrorKind = "validation"
	ErrorKindParsing           ErrorKind = "parsing"
	ErrorKindOCR               ErrorKind = "ocr"
	ErrorKindCache             ErrorKind = "cache"
	ErrorKindImageProcessing   ErrorKind = "image_processing"
	ErrorKindSerialization     ErrorKind = "serialization"
	ErrorKindMissingDependency ErrorKind = "missing_dependency"
	ErrorKindPlugin            ErrorKind = "plugin"
	ErrorKindUnsupportedFormat ErrorKind = "unsupported_format"
	ErrorKindRuntime           ErrorKind = "runtime"
)

// ErrorCode represents FFI error codes from kreuzberg-ffi.
// These codes map to the Rust ErrorCode enum and are the single source of truth.
type ErrorCode uint32

const (
	ErrorCodeValidation        ErrorCode = 0
	ErrorCodeParsing           ErrorCode = 1
	ErrorCodeOcr               ErrorCode = 2
	ErrorCodeMissingDependency ErrorCode = 3
	ErrorCodeIo                ErrorCode = 4
	ErrorCodePlugin            ErrorCode = 5
	ErrorCodeUnsupportedFormat ErrorCode = 6
	ErrorCodeInternal          ErrorCode = 7
)

// String returns the string representation of an ErrorCode.
func (ec ErrorCode) String() string {
	namePtr := C.kreuzberg_error_code_name(C.uint32_t(ec))
	if namePtr == nil {
		return "Unknown"
	}
	return C.GoString(namePtr)
}

// Description returns a human-readable description of the error code.
func (ec ErrorCode) Description() string {
	descPtr := C.kreuzberg_error_code_description(C.uint32_t(ec))
	if descPtr == nil {
		return "Unknown error code"
	}
	return C.GoString(descPtr)
}

// PanicContext contains panic context information from kreuzberg-ffi.
type PanicContext struct {
	File         string `json:"file"`
	Line         int    `json:"line"`
	Function     string `json:"function"`
	Message      string `json:"message"`
	TimestampSec int64  `json:"timestamp_secs"`
}

// String returns a formatted string representation of PanicContext.
func (pc *PanicContext) String() string {
	if pc == nil {
		return ""
	}
	return fmt.Sprintf("%s:%d in %s: %s", pc.File, pc.Line, pc.Function, pc.Message)
}

// KreuzbergError is implemented by all custom error types returned by the Go binding.
type KreuzbergError interface {
	error
	Kind() ErrorKind
	Code() ErrorCode
	PanicCtx() *PanicContext
}

type baseError struct {
	kind       ErrorKind
	message    string
	cause      error
	panicCtx   *PanicContext
	nativeCode ErrorCode
}

func (e *baseError) Error() string {
	return e.message
}

func (e *baseError) Kind() ErrorKind {
	return e.kind
}

func (e *baseError) Unwrap() error {
	return e.cause
}

func (e *baseError) PanicCtx() *PanicContext {
	return e.panicCtx
}

func (e *baseError) Code() ErrorCode {
	return e.nativeCode
}

type ValidationError struct {
	baseError
}

type ParsingError struct {
	baseError
}

type OCRError struct {
	baseError
}

type CacheError struct {
	baseError
}

type ImageProcessingError struct {
	baseError
}

type SerializationError struct {
	baseError
}

type MissingDependencyError struct {
	baseError
	Dependency string
}

type PluginError struct {
	baseError
	PluginName string
}

type UnsupportedFormatError struct {
	baseError
	Format string
}

type IOError struct {
	baseError
}

type RuntimeError struct {
	baseError
}

func makeBaseError(kind ErrorKind, message string, cause error, code ErrorCode, panicCtx *PanicContext) baseError {
	var msg string
	if panicCtx != nil {
		msg = formatErrorMessageWithCause(message, cause) + " [panic: " + panicCtx.String() + "]"
	} else {
		msg = formatErrorMessageWithCause(message, cause)
	}
	return baseError{
		kind:       kind,
		message:    msg,
		cause:      cause,
		panicCtx:   panicCtx,
		nativeCode: code,
	}
}

func newValidationErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ValidationError {
	return &ValidationError{baseError: makeBaseError(ErrorKindValidation, message, cause, code, panicCtx)}
}

func newParsingErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ParsingError {
	return &ParsingError{baseError: makeBaseError(ErrorKindParsing, message, cause, code, panicCtx)}
}

func newOCRErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *OCRError {
	return &OCRError{baseError: makeBaseError(ErrorKindOCR, message, cause, code, panicCtx)}
}

func newCacheErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *CacheError {
	return &CacheError{baseError: makeBaseError(ErrorKindCache, message, cause, code, panicCtx)}
}

func newImageProcessingErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *ImageProcessingError {
	return &ImageProcessingError{baseError: makeBaseError(ErrorKindImageProcessing, message, cause, code, panicCtx)}
}

func newSerializationErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *SerializationError {
	return &SerializationError{baseError: makeBaseError(ErrorKindSerialization, message, cause, code, panicCtx)}
}

func newMissingDependencyErrorWithContext(dependency string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *MissingDependencyError {
	return &MissingDependencyError{
		baseError:  makeBaseError(ErrorKindMissingDependency, messageWithFallback(message, fmt.Sprintf("Missing dependency: %s", dependency)), cause, code, panicCtx),
		Dependency: dependency,
	}
}

func newPluginErrorWithContext(plugin string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *PluginError {
	return &PluginError{
		baseError:  makeBaseError(ErrorKindPlugin, messageWithFallback(message, "Plugin error"), cause, code, panicCtx),
		PluginName: plugin,
	}
}

func newUnsupportedFormatErrorWithContext(format string, message string, cause error, code ErrorCode, panicCtx *PanicContext) *UnsupportedFormatError {
	return &UnsupportedFormatError{
		baseError: makeBaseError(ErrorKindUnsupportedFormat, messageWithFallback(message, fmt.Sprintf("Unsupported format: %s", format)), cause, code, panicCtx),
		Format:    format,
	}
}

func newIOErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *IOError {
	return &IOError{baseError: makeBaseError(ErrorKindIO, message, cause, code, panicCtx)}
}

func newRuntimeErrorWithContext(message string, cause error, code ErrorCode, panicCtx *PanicContext) *RuntimeError {
	return &RuntimeError{baseError: makeBaseError(ErrorKindRuntime, message, cause, code, panicCtx)}
}

func messageWithFallback(message string, fallback string) string {
	trimmed := strings.TrimSpace(message)
	if trimmed != "" {
		return trimmed
	}
	return fallback
}

func formatErrorMessageWithCause(message string, cause error) string {
	msg := formatErrorMessage(message)
	if cause != nil {
		return fmt.Sprintf("%s: %v", msg, cause)
	}
	return msg
}

func formatErrorMessage(message string) string {
	trimmed := strings.TrimSpace(message)
	if trimmed == "" {
		trimmed = "unknown error"
	}
	if strings.HasPrefix(strings.ToLower(trimmed), "kreuzberg:") {
		return trimmed
	}
	return "kreuzberg: " + trimmed
}

// classifyNativeError converts a native error message and code into a typed Kreuzberg error.
// Uses the FFI-provided error code to classify errors instead of string matching.
func classifyNativeError(message string, code ErrorCode, panicCtx *PanicContext) error {
	trimmed := strings.TrimSpace(message)
	if trimmed == "" {
		trimmed = "unknown error"
	}

	switch code {
	case ErrorCodeValidation:
		return newValidationErrorWithContext(trimmed, nil, code, panicCtx)
	case ErrorCodeParsing:
		return newParsingErrorWithContext(trimmed, nil, code, panicCtx)
	case ErrorCodeOcr:
		return newOCRErrorWithContext(trimmed, nil, code, panicCtx)
	case ErrorCodeMissingDependency:
		dependency := extractDependencyName(trimmed)
		return newMissingDependencyErrorWithContext(dependency, trimmed, nil, code, panicCtx)
	case ErrorCodeIo:
		return newIOErrorWithContext(trimmed, nil, code, panicCtx)
	case ErrorCodePlugin:
		plugin := extractPluginName(trimmed)
		return newPluginErrorWithContext(plugin, trimmed, nil, code, panicCtx)
	case ErrorCodeUnsupportedFormat:
		format := extractFormatName(trimmed)
		return newUnsupportedFormatErrorWithContext(format, trimmed, nil, code, panicCtx)
	case ErrorCodeInternal:
		return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
	default:
		return newRuntimeErrorWithContext(trimmed, nil, code, panicCtx)
	}
}

// extractDependencyName extracts the dependency name from an error message.
func extractDependencyName(message string) string {
	if idx := strings.Index(message, ":"); idx != -1 {
		return strings.TrimSpace(message[idx+1:])
	}
	return ""
}

// extractPluginName extracts the plugin name from an error message.
func extractPluginName(message string) string {
	start := strings.Index(message, "'")
	if start == -1 {
		return ""
	}
	rest := message[start+1:]
	end := strings.Index(rest, "'")
	if end == -1 {
		return ""
	}
	return rest[:end]
}

// extractFormatName extracts the format name from an error message.
func extractFormatName(message string) string {
	if idx := strings.Index(message, ":"); idx != -1 {
		return strings.TrimSpace(message[idx+1:])
	}
	return ""
}

func parsePluginName(message string) string {
	start := strings.Index(message, "'")
	if start == -1 {
		return ""
	}
	rest := message[start+1:]
	end := strings.Index(rest, "'")
	if end == -1 {
		return ""
	}
	return rest[:end]
}

// Phase 2 FFI Error Classification Wrappers

// ErrorCodeCount returns the total number of valid error codes (8).
func ErrorCodeCount() uint32 {
	return uint32(C.kreuzberg_error_code_count())
}

// ErrorCodeName returns the name of an error code as a string.
// Returns "unknown" for invalid codes.
func ErrorCodeName(code uint32) string {
	namePtr := C.kreuzberg_error_code_name(C.uint32_t(code))
	if namePtr == nil {
		return "unknown"
	}
	return C.GoString(namePtr)
}

// ErrorCodeDescription returns a human-readable description of an error code.
// Returns "Unknown error code" for invalid codes.
func ErrorCodeDescription(code uint32) string {
	descPtr := C.kreuzberg_error_code_description(C.uint32_t(code))
	if descPtr == nil {
		return "Unknown error code"
	}
	return C.GoString(descPtr)
}
