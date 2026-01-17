# Kreuzberg Rust Test App

## Overview

Comprehensive API coverage test for Kreuzberg 4.0.0-rc.17 Rust library.

**Goal**: Validate the entire Rust public API with a standalone application that uses kreuzberg from crates.io.

## Quick Start

```bash
# Build the test app
cargo build --release

# Run the comprehensive test suite
cargo run --release

# Run with verbose output
RUST_LOG=debug cargo run --release
```

## Test Suite

**File:** `src/main.rs` (comprehensive test suite)

### Coverage Summary

Tests the following major API categories:

#### 1. Core Extraction APIs
- `extract_file_sync()` - Synchronous file extraction
- `extract_bytes_sync()` - Synchronous byte extraction
- `extract_file()` - Async file extraction
- `extract_bytes()` - Async byte extraction

#### 2. Configuration Classes
- `ExtractionConfig` - Main configuration builder
- `OcrConfig` - OCR-specific settings
- `ChunkingConfig` - Text chunking configuration
- `LanguageDetectionConfig` - Language detection settings
- `ImageExtractionConfig` - Image extraction settings
- `EmbeddingConfig` - Embedding model settings
- `KeywordConfig` - Keyword extraction settings
- `PdfConfig` - PDF-specific settings

#### 3. Result Objects
- `ExtractionResult` - Main result container
- `Metadata` - Extracted metadata
- `ExtractedImage` - Image extraction data
- `ExtractedTable` - Table extraction data
- `Chunk` - Chunked text segment
- `ChunkMetadata` - Chunk metadata

#### 4. Plugin System
- Registry APIs for custom extractors, OCR backends, post-processors, validators
- Plugin listing and registration
- Plugin error handling

#### 5. Validation Functions
- MIME type validation
- OCR configuration validation
- Language detection validation
- Image DPI validation
- Confidence score validation
- And more...

#### 6. Utility Functions
- MIME type detection
- Configuration serialization
- Configuration merging
- Error code/detail retrieval

#### 7. Error Handling
- `KreuzbergError` exception types
- `ParsingError`, `OcrError`, `CacheError`
- Error codes and classifications
- Proper error recovery

#### 8. Advanced Features
- Batch extraction (sync and async)
- Streaming text processing
- Concurrent extraction with Tokio
- Custom configuration composition

## Test Documents

Located in `test_documents/`:

- **tiny.pdf** (1 KB) - Basic PDF extraction
- **lorem_ipsum.docx** (14.8 KB) - DOCX text extraction
- **stanley_cups.xlsx** (6.3 KB) - XLSX spreadsheet
- **ocr_image.jpg** (73.7 KB) - OCR image testing
- **test_hello_world.png** (1 KB) - PNG image format

## Environment

- **Rust:** 1.83+
- **Edition:** 2024
- **Kreuzberg:** 4.0.0-rc.17
- **OS:** macOS (Darwin), Linux (tested)
- **Dependencies:** minimal (only kreuzberg, tokio, serde)

## Project Structure

```
test_apps/rust/
├── Cargo.toml                  # Package configuration
├── README.md                   # This file
├── src/
│   └── main.rs                 # Comprehensive test suite
└── test_documents/
    ├── tiny.pdf
    ├── lorem_ipsum.docx
    ├── stanley_cups.xlsx
    ├── ocr_image.jpg
    └── test_hello_world.png
```

## How to Run

### Basic Test Run
```bash
cd test_apps/rust
cargo run --release
```

### With Debug Output
```bash
RUST_LOG=debug cargo run --release 2>&1 | tee test_output.log
```

### Run Specific Test (pattern matching)
```bash
cargo run --release -- --test-name "extraction"
```

### Build in Release Mode
```bash
cargo build --release
./target/release/kreuzberg-test
```

## Expected Output

```
================================================================================
KREUZBERG RUST BINDINGS COMPREHENSIVE TEST SUITE
================================================================================
✓ ALL IMPORTS SUCCESSFUL (Kreuzberg v4.0.0-rc.17)

Running 120+ tests across 8 major sections...

[1] Configuration Classes
  ✓ test_config_extraction_default
  ✓ test_config_with_builder
  ...

[2] Sync Extraction APIs
  ✓ test_extract_file_sync_pdf
  ✓ test_extract_bytes_sync_docx
  ...

[3] Async Extraction APIs
  ✓ test_extract_file_async_pdf
  ✓ test_extract_bytes_async
  ...

[4] Result Objects
  ✓ test_extraction_result_structure
  ✓ test_metadata_access
  ...

[5] Error Handling
  ✓ test_invalid_file_error
  ✓ test_parsing_error
  ...

[6] Plugin System
  ✓ test_plugin_registry
  ✓ test_plugin_errors
  ...

[7] Validation Functions
  ✓ test_mime_type_validation
  ✓ test_config_validation
  ...

[8] Advanced Features
  ✓ test_batch_extraction
  ✓ test_concurrent_extraction
  ...

================================================================================
TEST SUMMARY
================================================================================
Total Tests: 120+
  Passed:  120+
  Failed:  0
  Skipped: 0

Status: ALL TESTS PASSED ✓
Time: ~5-30 seconds (depending on system)
```

Exit codes:
- `0` = All tests passed
- `1` = Some tests failed

## Code Quality Standards

Follows Kreuzberg conventions:
- Rust Edition 2024
- Zero `unwrap()` in production code (uses `?` operator)
- Proper `Result<T, E>` error handling
- `SAFETY` comments for any unsafe blocks
- Doc comments on public items
- No clippy warnings: `cargo clippy -- -D warnings`
- High test coverage with unit and integration tests

## Running the Test Suite in CI/CD

### GitHub Actions Example

```yaml
- name: Test Rust Bindings
  run: |
    cd test_apps/rust
    cargo build --release
    cargo run --release
    # Expect exit code 0 for success
```

## Performance

- **Build time:** 30-60 seconds (from crates.io)
- **Runtime:** 5-30 seconds (depending on system and features)
- **Memory:** ~150-250 MB peak
- **Single-threaded:** Tokio async runtime for I/O

## Standards & Best Practices

### Error Handling
- Never panics in test code (uses Result and proper error handling)
- All file I/O uses proper error propagation
- Network and OCR failures are handled gracefully

### Async/Await
- All async tests use `#[tokio::test]` macro
- Properly manages Tokio runtime
- Tests concurrent operations safely

### Memory Safety
- No unsafe code in test suite
- Proper resource cleanup with RAII
- Temporary files cleaned up after tests

## Troubleshooting

### Missing Test Documents
If tests fail with "file not found", ensure test_documents/ directory exists:
```bash
cp -r test_apps/python/test_documents test_apps/rust/
```

### Dependency Issues
```bash
# Clear cache and rebuild
cargo clean
cargo build --release
```

### Async Runtime Issues
If you see "Runtime not found" errors:
```bash
# Ensure tokio features are enabled (already in Cargo.toml)
cargo tree | grep tokio
```

## Related Files

- Main Kreuzberg crate: https://crates.io/crates/kreuzberg
- Source repository: https://github.com/kreuzberg-dev/kreuzberg
- Python test app: `/test_apps/python`
- TypeScript test app: `/test_apps/node`
- Go test app: `/test_apps/go`

## API Reference

For detailed API documentation:
```bash
# Generate and open API docs
cargo doc --no-deps --open
```

This will open the rustdoc for kreuzberg with all public APIs documented.

## Contributing

When adding new tests:
1. Follow the existing test structure
2. Use clear test names: `test_<feature>_<scenario>_<outcome>`
3. Add doc comments explaining what is being tested
4. Handle errors properly (no panics)
5. Clean up resources (temp files, etc.)

Example:
```rust
#[tokio::test]
async fn test_extract_file_async_with_valid_pdf_returns_result() {
    let config = ExtractionConfig::default();
    let result = extract_file(Path::new("test_documents/tiny.pdf"), None, &config).await;

    assert!(result.is_ok());
    let extraction = result.unwrap();
    assert!(!extraction.text.is_empty());
}
```

## See Also

- Main Kreuzberg repository: https://github.com/kreuzberg-dev/kreuzberg
- Documentation: https://docs.kreuzberg.dev
- Issues: https://github.com/kreuzberg-dev/kreuzberg/issues
