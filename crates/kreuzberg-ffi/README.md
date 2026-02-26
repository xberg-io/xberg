# kreuzberg-ffi

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <!-- Language Bindings -->
  <a href="https://crates.io/crates/kreuzberg">
    <img src="https://img.shields.io/crates/v/kreuzberg?label=Rust&color=007ec6" alt="Rust">
  </a>
  <a href="https://hex.pm/packages/kreuzberg">
    <img src="https://img.shields.io/hexpm/v/kreuzberg?label=Elixir&color=007ec6" alt="Elixir">
  </a>
  <a href="https://pypi.org/project/kreuzberg/">
    <img src="https://img.shields.io/pypi/v/kreuzberg?label=Python&color=007ec6" alt="Python">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/node">
    <img src="https://img.shields.io/npm/v/@kreuzberg/node?label=Node.js&color=007ec6" alt="Node.js">
  </a>
  <a href="https://www.npmjs.com/package/@kreuzberg/wasm">
    <img src="https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM&color=007ec6" alt="WASM">
  </a>

  <a href="https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg">
    <img src="https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java&color=007ec6" alt="Java">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go&color=007ec6&filter=v4.3.8" alt="Go">
  </a>
  <a href="https://www.nuget.org/packages/Kreuzberg/">
    <img src="https://img.shields.io/nuget/v/Kreuzberg?label=C%23&color=007ec6" alt="C#">
  </a>
  <a href="https://packagist.org/packages/kreuzberg/kreuzberg">
    <img src="https://img.shields.io/packagist/v/kreuzberg/kreuzberg?label=PHP&color=007ec6" alt="PHP">
  </a>
  <a href="https://rubygems.org/gems/kreuzberg">
    <img src="https://img.shields.io/gem/v/kreuzberg?label=Ruby&color=007ec6" alt="Ruby">
  </a>
  <a href="https://kreuzberg-dev.r-universe.dev/kreuzberg">
    <img src="https://img.shields.io/badge/R-kreuzberg-007ec6" alt="R">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/pkgs/container/kreuzberg">
    <img src="https://img.shields.io/badge/Docker-007ec6?logo=docker&logoColor=white" alt="Docker">
  </a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg/releases">
    <img src="https://img.shields.io/badge/C-FFI-007ec6" alt="C">
  </a>

  <!-- Project Info -->
  <a href="https://github.com/kreuzberg-dev/kreuzberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.kreuzberg.dev">
    <img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Documentation">
  </a>
</div>

<img width="1128" height="191" alt="Banner2" src="https://github.com/user-attachments/assets/419fc06c-8313-4324-b159-4b4d3cfce5c0" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
      <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

C Foreign Function Interface (FFI) bindings for the Kreuzberg document intelligence library.

## Overview

This crate provides a C-compatible API that bridges the high-performance Rust core of Kreuzberg with multiple programming languages and FFI systems. It is the foundation for language bindings in Java (Panama FFI), Go (cgo), C# (P/Invoke), and other languages with C interoperability.

The FFI exposes extraction functions, configuration management, plugin registration, and error handling through a stable C interface with thread-safe callbacks.

## Architecture

### FFI Bridge Layers

```
Language-Specific Bindings
    ↓
Kreuzberg FFI C Library (crates/kreuzberg-ffi) ← This crate
    ↓
Rust Core Library (crates/kreuzberg)
    ↓
Document Extraction Engines
```

### Binding Support

This FFI layer is consumed by:

- **Java** (packages/java): Using Java 25 Foreign Function & Memory API (Panama FFI)
- **Go** (packages/go): Using cgo wrapper bindings
- **C#** (packages/csharp): Using P/Invoke interop
- **Zig** and other C-compatible languages

### Key Components

- **Core Extraction** (`extract_file`, `extract_bytes`): Document text and data extraction
- **Batch Operations**: Parallel processing of multiple documents
- **MIME Detection**: File format identification
- **Configuration Management**: Loading and applying extraction settings
- **Plugin System**: OCR backend registration and callbacks
- **Error Handling**: Thread-local error message storage
- **Memory Management**: Safe pointer handling and FFI boundaries

## Installation

### Build from Source

```bash
cargo build --release -p kreuzberg-ffi
```

### Output Artifacts

After building, you will have:

- **Dynamic Library**: `target/release/libkreuzberg_ffi.{so,dylib,dll}`
  - For loading at runtime
  - Platform-specific extensions (`.so` Linux, `.dylib` macOS, `.dll` Windows)

- **Static Library**: `target/release/libkreuzberg_ffi.{a,lib}`
  - For static linking into applications
  - Platform-specific extensions (`.a` Unix, `.lib` Windows)

- **Header File**: Auto-generated via `cbindgen` during build

### Header Generation

The C header file is automatically generated during the build process via `cbindgen`:

```bash
cargo build --release -p kreuzberg-ffi
# Header is generated at build time based on #[no_mangle] functions
```

For manual header generation:

```bash
cargo build --features html,embeddings -p kreuzberg-ffi
```

### pkg-config File Generation

The build process automatically generates pkg-config files for library discovery:

```bash
cargo build --release -p kreuzberg-ffi
```

This creates two variants in `crates/kreuzberg-ffi/`:
- **kreuzberg-ffi.pc**: Development version (prefix points to repository)
- **kreuzberg-ffi-install.pc**: Installation version (prefix=/usr/local)

The development variant enables monorepo developers to use pkg-config:

```bash
export PKG_CONFIG_PATH="$PWD/crates/kreuzberg-ffi:$PKG_CONFIG_PATH"
pkg-config --cflags kreuzberg-ffi  # Returns -I/path/to/repo/crates/kreuzberg-ffi
pkg-config --libs kreuzberg-ffi    # Returns -L/path/to/repo/target/release -lkreuzberg_ffi
```

The installation variant is used in release artifacts for third-party use.

### Installing from Release Artifacts

Pre-built C FFI packages are available from the [releases page](https://github.com/kreuzberg-dev/kreuzberg/releases).

Each `c-ffi-{platform}.tar.gz` archive contains:

```
include/kreuzberg.h
lib/libkreuzberg_ffi.{so|dylib|dll}
lib/libkreuzberg_ffi.{a|lib}
lib/pkgconfig/kreuzberg.pc
cmake/kreuzberg-ffi-config.cmake
cmake/kreuzberg-ffi-config-version.cmake
LICENSE
```

Platforms: `linux-x86_64`, `linux-aarch64`, `macos-arm64`, `windows-x86_64`

Installation:

```bash
# Download and extract
tar -xzf c-ffi-linux-x86_64.tar.gz

# System-wide installation (requires sudo)
sudo cp include/kreuzberg.h /usr/local/include/
sudo cp lib/libkreuzberg_ffi.* /usr/local/lib/
sudo mkdir -p /usr/local/lib/pkgconfig
sudo cp lib/pkgconfig/kreuzberg.pc /usr/local/lib/pkgconfig/
sudo mkdir -p /usr/local/lib/cmake/kreuzberg-ffi
sudo cp cmake/*.cmake /usr/local/lib/cmake/kreuzberg-ffi/
sudo ldconfig  # Linux only

# Verify
pkg-config --modversion kreuzberg
```

### Homebrew

```bash
brew install kreuzberg-dev/tap/kreuzberg-ffi
```

### CMake

After installing, use `find_package` in your `CMakeLists.txt`:

```cmake
find_package(kreuzberg-ffi REQUIRED)
target_link_libraries(my_app PRIVATE kreuzberg-ffi::kreuzberg-ffi)
```

### Version Constants

The generated header includes compile-time version constants:

```c
#define KREUZBERG_VERSION_MAJOR 4
#define KREUZBERG_VERSION_MINOR 3
#define KREUZBERG_VERSION_PATCH 8
#define KREUZBERG_VERSION "4.3.8"
```

Use these for compile-time version checks, and `kreuzberg_version()` for runtime checks.

## Quick Start: C Example

### Basic Extraction

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Include the auto-generated Kreuzberg FFI header
#include "kreuzberg.h"

int main() {
    // Extract text from a PDF file
    const char* file_path = "document.pdf";

    // Perform extraction (returns opaque pointer, NULL on error)
    CExtractionResult* result = kreuzberg_extract_file_sync(file_path);

    // Check for errors
    if (result == NULL || !result->success) {
        const char* error = kreuzberg_last_error();
        fprintf(stderr, "Extraction failed: %s\n", error ? error : "unknown error");
        if (result) kreuzberg_free_result(result);
        return 1;
    }

    // Process the result
    printf("Extracted content:\n%s\n", result->content);
    printf("MIME Type: %s\n", result->mime_type);

    // Free resources
    kreuzberg_free_result(result);

    return 0;
}
```

### Compilation

```bash
# Link against the dynamic library
gcc -o extract_example extract.c \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi
```

## API Reference

### Core Extraction Functions

All extraction functions return opaque pointers (NULL on error). Error messages are accessible via `kreuzberg_last_error()`.

#### `CExtractionResult* kreuzberg_extract_file_sync(const char *file_path)`

Extract text and metadata from a file.

**Parameters:**
- `file_path`: Absolute or relative file path (must exist, null-terminated C string)

**Returns:**
- `CExtractionResult*`: Opaque pointer to extraction result (NULL on error). Must be freed with `kreuzberg_free_result()`.

**Example:**

```c
CExtractionResult* result = kreuzberg_extract_file_sync("document.pdf");

if (result == NULL || !result->success) {
    const char* error = kreuzberg_last_error();
    printf("Error: %s\n", error ? error : "unknown");
    if (result) kreuzberg_free_result(result);
} else {
    printf("Content: %s\n", result->content);
    printf("MIME: %s\n", result->mime_type);
    kreuzberg_free_result(result);
}
```

#### `CExtractionResult* kreuzberg_extract_file_sync_with_config(const char *file_path, const ExtractionConfig *config)`

Extract text and metadata from a file using a custom configuration.

**Parameters:**
- `file_path`: Absolute or relative file path (must exist)
- `config`: Pointer to an `ExtractionConfig` (opaque, created via config builder functions). Pass NULL for defaults.

**Returns:**
- `CExtractionResult*`: Opaque pointer to extraction result (NULL on error). Must be freed with `kreuzberg_free_result()`.

#### `CExtractionResult* kreuzberg_extract_bytes_sync(const uint8_t *data, size_t data_len, const char *mime_type)`

Extract from a byte buffer (e.g., file in memory).

**Parameters:**
- `data`: Pointer to byte buffer
- `data_len`: Buffer length in bytes
- `mime_type`: MIME type as null-terminated C string (required)

**Returns:**
- `CExtractionResult*`: Opaque pointer (NULL on error). Must be freed with `kreuzberg_free_result()`.

**Example:**

```c
// Read file into buffer
FILE* f = fopen("document.pdf", "rb");
fseek(f, 0, SEEK_END);
size_t size = ftell(f);
fseek(f, 0, SEEK_SET);
unsigned char* buffer = malloc(size);
fread(buffer, 1, size, f);
fclose(f);

// Extract from buffer
CExtractionResult* result = kreuzberg_extract_bytes_sync(
    buffer,
    size,
    "application/pdf"
);

if (result != NULL && result->success) {
    printf("Content: %s\n", result->content);
}

kreuzberg_free_result(result);
free(buffer);
```

#### `CBatchResult* kreuzberg_batch_extract_files_sync(const char **paths, size_t paths_count, const ExtractionConfig *config)`

Process multiple files in parallel.

**Parameters:**
- `paths`: Array of file path strings
- `paths_count`: Number of paths in array
- `config`: Pointer to an `ExtractionConfig` (pass NULL for defaults)

**Returns:**
- `CBatchResult*`: Opaque pointer containing array of results. Must be freed with `kreuzberg_free_batch_result()`.

**Example:**

```c
const char* files[] = {"doc1.pdf", "doc2.docx", "doc3.xlsx"};
size_t count = 3;

CBatchResult* batch = kreuzberg_batch_extract_files_sync(files, count, NULL);

if (batch != NULL && batch->success) {
    for (size_t i = 0; i < batch->count; i++) {
        CExtractionResult* r = batch->results[i];
        if (r && r->success) {
            printf("File %zu MIME: %s\n", i, r->mime_type);
        }
    }
}

kreuzberg_free_batch_result(batch);
```

#### `char* kreuzberg_detect_mime_type(const char *path)`

Identify the MIME type of a file using magic bytes and file signatures.

**Parameters:**
- `path`: File path to analyze

**Returns:**
- `char*`: MIME type string (must be freed with `kreuzberg_free_string`), or NULL on error.

**Example:**

```c
char* mime = kreuzberg_detect_mime_type("unknown-file");
if (mime) {
    printf("MIME Type: %s\n", mime);
    kreuzberg_free_string(mime);
}
```

### Configuration Management

Configuration is managed through an opaque `ExtractionConfig` type and a builder pattern.

#### Config Builder

```c
// Create a config builder
ConfigBuilder* builder = kreuzberg_config_builder_new();

// Set options
kreuzberg_config_builder_set_use_cache(builder, true);
kreuzberg_config_builder_set_ocr(builder, "tesseract", "eng");

// Build the config (consumes the builder)
ExtractionConfig* config = kreuzberg_config_builder_build(builder);

// Use with extraction
CExtractionResult* result = kreuzberg_extract_file_sync_with_config("doc.pdf", config);

// Free the config when done
kreuzberg_config_free(config);
```

#### JSON Configuration

```c
// Load config from JSON string
ExtractionConfig* config = kreuzberg_config_from_json("{\"use_cache\": true}");

// Export config as JSON
char* json = kreuzberg_config_to_json(config);
printf("Config: %s\n", json);
kreuzberg_free_string(json);

kreuzberg_config_free(config);
```

### Plugin System

#### `bool kreuzberg_register_ocr_backend(const char *backend_name, OcrBackendCallback callback)`

Register a custom OCR backend implemented in the calling language.

**Parameters:**
- `backend_name`: Unique identifier for the backend (e.g., "custom-ocr")
- `callback`: Function pointer matching `char* (*)(const uint8_t* data, size_t len, const char* lang)`

**Returns:**
- `true` on success, `false` on failure (e.g., NULL name)

**Example (C):**

```c
// Define the callback function
char* custom_ocr_callback(
    const uint8_t* image_data,
    size_t image_size,
    const char* language
) {
    // Perform OCR and return allocated string
    // (caller will free with kreuzberg_free_string)
    return NULL;
}

// Register the backend
bool ok = kreuzberg_register_ocr_backend("easyocr", custom_ocr_callback);

// List registered backends
char* backends = kreuzberg_list_ocr_backends();
printf("Backends: %s\n", backends);
kreuzberg_free_string(backends);
```

### Error Handling

#### `const char* kreuzberg_last_error()`

Retrieve the last error message.

**Returns:**
- `const char*`: Error message string (NULL if no error). This is a thread-local pointer; do not free it.

Note: Error messages are thread-local and persist until the next Kreuzberg function call on the same thread.

#### `int32_t kreuzberg_last_error_code()`

Retrieve the error code for the last error.

**Returns:**
- Error code: 0 (Success), 1 (GenericError), 2 (Panic), 3 (InvalidArgument), 4 (IoError), 5 (ParsingError), 6 (OcrError), 7 (MissingDependency)

**Example:**

```c
CExtractionResult* result = kreuzberg_extract_file_sync("invalid.pdf");

if (result == NULL) {
    const char* error = kreuzberg_last_error();
    int32_t code = kreuzberg_last_error_code();
    printf("Error (code %d): %s\n", code, error ? error : "unknown");
}
```

### Memory Management

#### `void kreuzberg_free_string(char *ptr)`

Free a string allocated by FFI functions. Passing NULL is a safe no-op.

**Note:** Only use for strings returned by FFI functions, not for caller-allocated strings.

**Example:**

```c
char* mime = kreuzberg_detect_mime_type("file.pdf");
kreuzberg_free_string(mime);
```

#### `void kreuzberg_free_result(CExtractionResult *result)`

Free an extraction result and all its string fields. Passing NULL is a safe no-op.

**Example:**

```c
CExtractionResult* result = kreuzberg_extract_file_sync("doc.pdf");
// ... use result ...
kreuzberg_free_result(result);
```

#### `void kreuzberg_free_batch_result(CBatchResult *batch)`

Free a batch extraction result, including all individual results. Passing NULL is a safe no-op.

**Example:**

```c
CBatchResult* batch = kreuzberg_batch_extract_files_sync(files, 3, NULL);
// ... use batch->results ...
kreuzberg_free_batch_result(batch);
```

## Type Definitions

### CExtractionResult

Opaque struct with 19 string pointer fields and a success flag. All string fields are null-terminated UTF-8 (or NULL if not available). Total size: 160 bytes on 64-bit systems.

```c
typedef struct {
    char* content;                    // Extracted text content
    char* mime_type;                  // Detected MIME type
    char* language;                   // Document language (or NULL)
    char* date;                       // Document date (or NULL)
    char* subject;                    // Document subject (or NULL)
    char* tables_json;                // Tables as JSON array (or NULL)
    char* detected_languages_json;    // Detected languages JSON (or NULL)
    char* metadata_json;              // Metadata JSON object (or NULL)
    char* chunks_json;                // Text chunks JSON array (or NULL)
    char* images_json;                // Extracted images JSON (or NULL)
    char* page_structure_json;        // Page structure JSON (or NULL)
    char* pages_json;                 // Per-page content JSON (or NULL)
    char* elements_json;              // Semantic elements JSON (or NULL)
    char* ocr_elements_json;          // OCR elements JSON (or NULL)
    char* document_json;              // Document structure JSON (or NULL)
    char* extracted_keywords_json;    // Keywords JSON (or NULL)
    char* quality_score_json;         // Quality score JSON (or NULL)
    char* processing_warnings_json;   // Warnings JSON (or NULL)
    char* annotations_json;           // Annotations JSON (or NULL)
    bool success;                     // Whether extraction succeeded
    uint8_t _padding[7];             // Alignment padding
} CExtractionResult;
```

### CBatchResult

```c
typedef struct {
    CExtractionResult** results;  // Array of pointers to extraction results
    size_t count;                 // Number of results
    bool success;                 // Whether batch operation succeeded
    uint8_t _padding[7];         // Alignment padding
} CBatchResult;
```

### Opaque Types

The following types are opaque pointers -- callers should not access their internals:

- `ExtractionConfig` -- extraction configuration (created via builder or JSON)
- `ExtractionResult` -- extraction result (accessed via `kreuzberg_result_*` functions)
- `ConfigBuilder` -- builder for constructing `ExtractionConfig`
- `ResultPool` -- memory pool for reusing result allocations

## FFI Safety Guidelines

### Thread Safety

All FFI functions are **thread-safe**. The Kreuzberg core uses Arc, Mutex, and RwLock for safe concurrent access:

```c
// Safe to call from multiple threads
for (int i = 0; i < 10; i++) {
    pthread_t thread;
    pthread_create(&thread, NULL, extract_worker, (void*)(intptr_t)i);
}
```

### Pointer Validation

The FFI layer validates all input pointers:

```c
// Safe: NULL pointers are handled gracefully
CExtractionResult* result = kreuzberg_extract_file_sync(NULL);
// Returns NULL, check kreuzberg_last_error() for details

// Safe: Invalid paths return errors
CExtractionResult* result = kreuzberg_extract_file_sync("/nonexistent");
// Returns NULL, check kreuzberg_last_error() for details
```

### Memory Lifetime

- **Returned strings** must be freed with `kreuzberg_free_string()`
- **Result pointers** must be freed with `kreuzberg_free_result()` or `kreuzberg_free_batch_result()`
- **Input parameters** are copied internally; caller retains ownership

```c
// Correct: Free returned values
char* mime = kreuzberg_detect_mime_type("file.pdf");
kreuzberg_free_string(mime);

// Correct: Input paths can be freed immediately after call
{
    char path[256];
    snprintf(path, sizeof(path), "document.pdf");
    CExtractionResult* result = kreuzberg_extract_file_sync(path);
    // path can be freed here; FFI has already copied it
    kreuzberg_free_result(result);
}
```

### Error Handling Pattern

```c
CExtractionResult* result = kreuzberg_extract_file_sync(path);

// Check return value first
if (result == NULL || !result->success) {
    const char* error = kreuzberg_last_error();
    fprintf(stderr, "FFI Error: %s\n", error ? error : "unknown");
    if (result) kreuzberg_free_result(result);
    return;
}

// Process successful result
printf("Content: %s\n", result->content);
printf("MIME: %s\n", result->mime_type);

// Always cleanup
kreuzberg_free_result(result);
```

## Building from C

### Static Linking

```bash
# Build the library
cargo build --release -p kreuzberg-ffi

# Create your C program
gcc -c -o myapp.o myapp.c

# Link statically
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -l:libkreuzberg_ffi.a \
    -pthread -ldl  # May need additional system libs

# Run
./myapp
```

### Dynamic Linking

```bash
# Build the library
cargo build --release -p kreuzberg-ffi

# Create your C program
gcc -c -o myapp.o myapp.c

# Link dynamically
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi

# Set library path and run
export LD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$LD_LIBRARY_PATH
./myapp
```

### macOS Considerations

```bash
# Build
cargo build --release -p kreuzberg-ffi

# Link
gcc -o myapp myapp.o \
    -L/path/to/kreuzberg/target/release \
    -lkreuzberg_ffi

# Set runtime path
export DYLD_LIBRARY_PATH=/path/to/kreuzberg/target/release:$DYLD_LIBRARY_PATH
./myapp
```

## Language Binding Integration

### Java Integration (Panama FFI)

The FFI is wrapped in Java 25's Foreign Function & Memory API:

```java
// Java code that calls FFI
Arena arena = Arena.ofConfined();
MemorySegment path = arena.allocateUtf8String("document.pdf");
MemorySegment config = arena.allocateUtf8String("{}");

ExtractionResult result = KreuzbergFFI.extract_file(path, MemorySegment.NULL, config);
```

### Go Integration (cgo)

The FFI is exposed through cgo bindings:

```go
// Go code that calls FFI
C.kreuzberg_extract_file(C.CString("document.pdf"), nil, C.CString("{}"))
```

### C# Integration (P/Invoke)

The FFI is declared in C# through P/Invoke:

```csharp
[DllImport("kreuzberg_ffi", CharSet = CharSet.Ansi)]
private static extern IntPtr kreuzberg_extract_file(
    string path,
    string mimeType,
    string configJson
);
```

## Supported Features

### Default Features
- `html`: HTML to Markdown conversion support
- `embeddings`: Text embedding extraction via fastembed-rs (requires ONNX Runtime - must be installed separately)

### System Requirements for Embeddings

If using the `embeddings` feature, ONNX Runtime must be installed on the system:

```bash
# macOS
brew install onnxruntime

# Ubuntu/Debian
sudo apt install libonnxruntime libonnxruntime-dev

# Windows (MSVC)
scoop install onnxruntime
# OR download from https://github.com/microsoft/onnxruntime/releases
```

Without ONNX Runtime, embeddings functionality will raise errors at runtime.

### Core Feature (Windows MinGW Compatibility)
- `core`: Minimal feature set for cross-platform compatibility
  - Includes: `html` (HTML to Markdown conversion)
  - Excludes: `embeddings` (ONNX Runtime not available on MinGW)
  - Use case: Windows Go bindings with MinGW toolchain

### Platform-Specific Build Requirements

**Windows MinGW (Go bindings):**

The Windows ONNX Runtime library only provides MSVC-compatible .lib files. MinGW cannot link against these, requiring the core feature:

```bash
# Windows MinGW - Use core feature
cargo build --release -p kreuzberg-ffi --target x86_64-pc-windows-gnu --no-default-features --features core

# Windows MSVC - Full features available
cargo build --release -p kreuzberg-ffi --target x86_64-pc-windows-msvc

# Unix (Linux/macOS) - Full features available
cargo build --release -p kreuzberg-ffi
```

**Why MinGW Requires core Feature:**
- ONNX Runtime distributes Windows binaries compiled with MSVC toolchain
- MSVC .lib files use different name mangling and linking conventions than MinGW
- MinGW's GNU toolchain cannot consume MSVC import libraries
- The `core` feature excludes the `embeddings` dependency, which depends on ort-sys (ONNX Runtime)
- HTML support (via html-to-markdown-rs) is pure Rust and works on all platforms

### Building with Features

```bash
# Build with HTML and embeddings support (default)
cargo build --release -p kreuzberg-ffi

# Build with core feature only (Windows MinGW compatibility)
cargo build --release -p kreuzberg-ffi --no-default-features --features core

# Build without any features (minimal FFI)
cargo build --release -p kreuzberg-ffi --no-default-features
```

## Performance Characteristics

- **Single extraction**: 10-100ms (varies by file size and format)
- **Batch processing**: Near-linear scaling with CPU cores
- **OCR processing**: 100-500ms per page
- **Memory overhead**: ~2-5MB per extraction operation
- **Thread safety**: Zero synchronization overhead on single-threaded extraction

## Key Files

- `src/lib.rs`: FFI function implementations and plugin system
- `Cargo.toml`: Dependencies and features
- Generated header: Auto-created by cbindgen during build

## Building

### Development Build

```bash
cargo build -p kreuzberg-ffi
```

### Release Build

```bash
cargo build --release -p kreuzberg-ffi
```

### With All Features

```bash
cargo build --release -p kreuzberg-ffi --features html,embeddings
```

## Testing

```bash
# Run FFI tests
cargo test -p kreuzberg-ffi

# With logging
RUST_LOG=debug cargo test -p kreuzberg-ffi -- --nocapture
```

## Troubleshooting

### Library Not Found

Ensure the built library is in the linker search path:

```bash
# Check for built libraries
ls -la target/release/libkreuzberg_ffi*

# Add to library path
export LD_LIBRARY_PATH=target/release:$LD_LIBRARY_PATH
export DYLD_LIBRARY_PATH=target/release:$DYLD_LIBRARY_PATH  # macOS
```

### Undefined Reference Errors

Ensure you're linking against the FFI library, not the core library:

```bash
# Correct
gcc -o app app.c -lkreuzberg_ffi

# Incorrect
gcc -o app app.c -lkreuzberg  # Wrong library
```

### Memory Leaks

Always free returned strings and result structures:

```c
// Problem: Memory leak
char* mime = kreuzberg_detect_mime_type("file.pdf");
printf("%s\n", mime);
// mime not freed!

// Solution: Free the string
char* mime = kreuzberg_detect_mime_type("file.pdf");
printf("%s\n", mime);
kreuzberg_free_string(mime);
```

### Thread-Local Error Messages

Each thread has its own error message storage. Check both return values and `kreuzberg_last_error()`:

```c
// Safe across threads
#pragma omp parallel for
for (int i = 0; i < 10; i++) {
    CExtractionResult* result = kreuzberg_extract_file_sync(files[i]);
    if (result == NULL) {
        printf("Thread %d error: %s\n", i, kreuzberg_last_error());
    } else {
        kreuzberg_free_result(result);
    }
}
```

## References

- **Kreuzberg Core**: `../kreuzberg/`
- **C FFI Standards**: https://en.cppreference.com/w/c
- **cbindgen Documentation**: https://rust-lang.github.io/cbindgen/
- **Project Homepage**: https://kreuzberg.dev
- **GitHub Repository**: https://github.com/kreuzberg-dev/kreuzberg

## Contributing

We welcome contributions! Please see the main Kreuzberg repository for contribution guidelines.

## License

MIT
