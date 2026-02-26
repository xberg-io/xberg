# Kreuzberg Go Test Suite

Comprehensive test suite for Kreuzberg v4.3.6 Go bindings (github.com/kreuzberg-dev/kreuzberg/packages/go/v4).

This directory contains two test approaches:

1. **Traditional Unit Tests** (`extraction_test.go`) - 72 tests using Go's `go test` framework
2. **Comprehensive API Test** (`main.go`) - Standalone 86-test application validating the entire public API

## Quick Start

### Prerequisites
- go 1.26 or higher
- Kreuzberg FFI library compiled: `cargo build --release -p kreuzberg-ffi`
- Rust toolchain (for building FFI)

### Run Unit Tests

```bash
# From the test_apps/go directory
cd test_apps/go

# Option 1: Using the convenience script
bash run_tests.sh

# Option 2: Manual execution with environment variables
export DYLD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$DYLD_LIBRARY_PATH
export LD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$LD_LIBRARY_PATH
export PKG_CONFIG_PATH=/path/to/kreuzberg/crates/kreuzberg-ffi:$PKG_CONFIG_PATH
go test -v ./...
```

### Run Comprehensive API Tests

```bash
# Build the test app
go build -o main main.go

# Run it (macOS)
export DYLD_LIBRARY_PATH=/path/to/kreuzberg/target/release
./main

# Run it (Linux)
export LD_LIBRARY_PATH=/path/to/kreuzberg/target/release
./main
```

### Expected Output

**Unit Tests:** All 72 tests should pass
```
ok  	kreuzberg-test-suite	0.670s
```

**Comprehensive Tests:** All 86 tests should pass
```
================================================================================
TEST SUMMARY
================================================================================
Total Tests: 86
  Passed:  86
  Failed:  0
  Skipped: 0

✓✓✓ ALL TESTS PASSED ✓✓✓
```

## Test Coverage

### main.go: 86 Comprehensive API Tests (14 Sections)

The new standalone test application validates the entire public API surface:

1. **Configuration Structs** (20 tests) - All ExtractionConfig subtypes and nested structures
2. **Pointer Helpers** (4 tests) - BoolPtr, StringPtr, IntPtr, FloatPtr
3. **Config Functions** (8 tests) - JSON parsing, serialization, merging, field access
4. **MIME Type Functions** (4 tests) - Detection from bytes/paths, validation, extension mapping
5. **Validation Functions** (11 tests) - OCR backend, language codes, Tesseract PSM/OEM, binarization, token reduction
6. **Error Types and Handling** (10 tests) - Full error type hierarchy and KreuzbergError interface
7. **FFI Error Code Functions** (3 tests) - ErrorCodeCount, ErrorCodeName, ErrorCodeDescription
8. **Extraction Functions (Sync)** (5 tests) - ExtractFileSync, ExtractBytesSync with various configurations
9. **Extraction Functions (Context)** (2 tests) - Context-aware extraction with cancellation support
10. **Batch Extraction** (2 tests) - BatchExtractFilesSync, BatchExtractBytesSync
11. **Library Info** (3 tests) - LibraryVersion, LastErrorCode, LastPanicContext
12. **Result Types and Accessors** (8 tests) - ExtractionResult, Metadata, Table, Chunk, ExtractedImage
13. **Plugin Registry Functions** (4 tests) - ListOCRBackends, ListPostProcessors, ListValidators, ListDocumentExtractors
14. **Embedding Presets** (2 tests) - ListEmbeddingPresets, GetEmbeddingPreset

### extraction_test.go: 72 Comprehensive Tests Organized Into 12 Categories

#### 1. Type Verification (8 tests)
Verify all exported types are accessible and work correctly:
- ExtractionResult structure
- ExtractionConfig structure
- Metadata, Table, Chunk, ExtractedImage, PageContent types
- Pointer helper functions (BoolPtr, StringPtr, IntPtr, FloatPtr)

#### 2. Synchronous File Extraction (8 tests)
Extract text from various document types:
- PDF, DOCX, XLSX, JPG, PNG, ODT, Markdown
- With and without custom configuration

#### 3. File Byte Extraction (8 tests)
Extract from in-memory byte slices with proper MIME type handling:
- All document types via bytes
- Configuration support
- Empty data validation

#### 4. Batch Extraction APIs (5 tests)
Test efficient batch processing:
- Multiple file extraction
- Multiple byte extraction
- Mixed document types
- Empty batch handling
- Configuration propagation

#### 5. MIME Type Detection (5 tests)
Verify MIME type detection capabilities:
- From file bytes
- From file path
- Extension retrieval for MIME types
- Support for multiple formats

#### 6. File Type Coverage (7 tests)
Comprehensive testing of supported formats:
- PDFs (tiny, medium, large sizes)
- DOCX (multiple variants)
- XLSX spreadsheets
- Images (JPEG, PNG)
- OpenDocument Text (ODT)
- Markdown documents

#### 7. Configuration Handling (2 tests)
Configuration creation and usage:
- Building ExtractionConfig with options
- Nil and empty config handling (uses defaults)

#### 8. Result Structure Validation (5 tests)
Verify extraction results contain expected fields:
- Content, MimeType, Success flags
- Metadata extraction
- Table detection
- Proper field population

#### 9. Error Handling (8 tests)
Comprehensive error testing:
- Missing file detection
- Empty path validation
- Empty data validation
- Empty MIME type validation
- Invalid input handling
- Error wrapping with errors.Is()
- Proper error messages

#### 10. Context Support (3 tests)
Context handling for async operations:
- Background context
- Context cancellation
- Context timeout

#### 11. Metadata Validation (4 tests)
Test metadata structure and accessors:
- FormatType() discriminator
- PdfMetadata() type-safe accessor
- ExcelMetadata() type-safe accessor
- ChunkMetadata field validation

#### 12. MIME Type Validation (1 test)
Validate MIME type support:
- Canonical MIME type retrieval
- Format validation

## File Structure

```
test_apps/go/
├── README.md                 # This file
├── main.go                  # 86 comprehensive API tests (standalone executable)
├── extraction_test.go       # 72 unit tests (uses go test)
├── go.mod                   # Module definition with v4.0
├── go.sum                   # Dependency lock file
├── run_tests.sh             # Convenience test runner script
└── test_documents/
    ├── tiny.pdf             # Small PDF for extraction tests
    ├── lorem_ipsum.docx     # Word document
    ├── stanley_cups.xlsx    # Spreadsheet
    ├── ocr_image.jpg        # JPEG image
    └── test_hello_world.png # PNG image
```

### Test Files Comparison

| Aspect | extraction_test.go | main.go |
|--------|-------------------|---------|
| Tests | 72 | 86 |
| Framework | Go testing package | Custom runner |
| Executable | `go test` | Binary (`./main`) |
| Scope | Extraction functions | Entire public API |
| Configuration | Traditional testing | API verification |
| Run Command | `go test -v ./...` | `./main` |

## Test Documents

Tests use real documents from `test_documents/` directory:

### PDFs
- `pdfs_with_tables/tiny.pdf` (7 rows × 3 columns)
- `pdfs_with_tables/medium.pdf` (multiple tables)
- `pdfs_with_tables/large.pdf` (complex content)

### Office Documents
- `documents/lorem_ipsum.docx`
- `documents/docx_tables.docx`
- `documents/word_sample.docx`
- `spreadsheets/stanley_cups.xlsx`

### Images
- `images/example.jpg`
- `images/sample.png`
- `images/ocr_image.jpg`

### Other Formats
- `odt/paragraph.odt`
- `extraction_test.md`

## Key Features Tested

### Core Functionality
- ✓ Synchronous extraction from files
- ✓ Synchronous extraction from bytes
- ✓ Batch file extraction
- ✓ Batch byte extraction
- ✓ MIME type detection (content + path)
- ✓ Configuration-driven extraction

### Supported Formats
- ✓ PDF (with table detection)
- ✓ Microsoft Word (DOCX)
- ✓ Microsoft Excel (XLSX)
- ✓ JPEG/PNG images
- ✓ OpenDocument Text (ODT)
- ✓ Markdown

### Configuration Options
- ✓ UseCache
- ✓ EnableQualityProcessing
- ✓ ForceOCR
- ✓ MaxConcurrentExtractions
- ✓ Custom configurations

### Result Data
- ✓ Text content
- ✓ MIME type detection
- ✓ Document metadata
- ✓ Table extraction
- ✓ Page boundaries
- ✓ Image metadata
- ✓ Language detection

### Error Handling
- ✓ Missing file detection
- ✓ Input validation
- ✓ Error wrapping (errors.Is compatible)
- ✓ Meaningful error messages

## Usage Examples

### Basic Extraction
```go
import "github.com/kreuzberg-dev/kreuzberg/packages/go/v4"

// Extract from file
result, err := kreuzberg.ExtractFileSync("document.pdf", nil)
if err != nil {
    log.Fatalf("extraction failed: %v", err)
}
fmt.Printf("Content: %s\nMIME: %s\n", result.Content, result.MimeType)
```

### With Configuration
```go
config := &kreuzberg.ExtractionConfig{
    UseCache: kreuzberg.BoolPtr(true),
    EnableQualityProcessing: kreuzberg.BoolPtr(true),
}
result, err := kreuzberg.ExtractFileSync("document.pdf", config)
```

### From Bytes
```go
data, _ := os.ReadFile("image.png")
result, err := kreuzberg.ExtractBytesSync(data, "image/png", nil)
```

### Batch Processing
```go
files := []string{"file1.pdf", "file2.docx", "file3.xlsx"}
results, err := kreuzberg.BatchExtractFilesSync(files, nil)
for _, result := range results {
    if result.Success {
        fmt.Printf("Extracted %d bytes\n", len(result.Content))
    }
}
```

### MIME Type Detection
```go
// From bytes
mimeType, err := kreuzberg.DetectMimeType(data)

// From file path
mimeType, err := kreuzberg.DetectMimeTypeFromPath("document.pdf")

// Validate MIME type
canonical, err := kreuzberg.ValidateMimeType("application/pdf")
```

## Development

### Running Specific Tests
```bash
# Run type verification tests only
go test -v -run TestTypeVerification

# Run extraction tests
go test -v -run TestExtractFile

# Run error tests
go test -v -run TestError

# Run with coverage
go test -v -cover ./...
```

### Adding New Tests
1. Add test function to `extraction_test.go` following pattern: `TestNameOfFeatureTestedOutcome`
2. Use table-driven tests for similar scenarios
3. Use testify/assert for fluent assertions
4. Add descriptive assertion messages
5. Run `go test -v ./...` to validate

## Troubleshooting

### Library Not Found Error
```
Package kreuzberg-ffi was not found in the pkg-config search path
```

Solution:
```bash
export PKG_CONFIG_PATH=/path/to/kreuzberg/crates/kreuzberg-ffi:$PKG_CONFIG_PATH
```

### Missing Library at Runtime
```
dyld: Library not loaded: @rpath/libkreuzberg_ffi.dylib
```

Solution:
```bash
export DYLD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$DYLD_LIBRARY_PATH
```

### Test File Not Found
```
File does not exist: /path/to/test_document.pdf
```

Ensure:
1. Working directory is correct
2. Test documents exist in `test_documents/`
3. FFI library is built: `cargo build --release -p kreuzberg-ffi`

## Test Quality

### Code Standards
- ✓ go 1.26 compatible
- ✓ Table-driven test patterns
- ✓ testify/assert assertions
- ✓ Descriptive error messages
- ✓ Proper error wrapping (errors.Is)
- ✓ golangci-lint compatible

### Coverage
- All exported types verified
- Happy path (success) testing
- Error path (failure) testing
- Configuration variations
- Batch operations
- MIME type detection
- Metadata extraction
- Type accessors

## Environment

### Verified With
- go 1.26.5
- macOS (ARM64)
- Kreuzberg v4.0

### Platform Support
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (via WSL or native Go with FFI)

## Performance Notes

- Tiny PDF: ~60ms extraction
- Medium PDF: ~20ms extraction
- Large PDF: ~210ms extraction (with table detection)
- Batch operations: Leverage optimized pipeline
- Caching: Reduces redundant extractions

## See Also

- [Kreuzberg Go Bindings](../../packages/go/v4/README.md)
- [Test Report](TEST_REPORT.md)
- [Main Kreuzberg Project](../../)

## License

Same as Kreuzberg project (see main repository)
