# Migrating from v3 to v4

Kreuzberg v4 represents a complete architectural rewrite with a Rust-first design. This guide helps you migrate from v3 to v4.

---

## Embeddings Breaking Change in v4

**⚠️ BREAKING CHANGE**: v4 switches embeddings from bundled ONNX Runtime to dynamic loading, requiring separate installation.

### Overview

v4 replaces the `ort-download-binaries` dependency with `ort-load-dynamic` for ONNX Runtime. This change:

- **Reduces package sizes** by 150-200MB per platform
- **Enables Windows MSVC support** for embeddings (previously unavailable)
- **Requires manual ONNX Runtime installation** if you use embeddings

### Who Is Affected?

- **If you use embeddings** (chunking with embeddings, RAG pipelines): **Action required**
- **If you don't use embeddings**: **No action needed** - all other features work without ONNX Runtime

### Installation Instructions

Install ONNX Runtime for your platform:

#### macOS

```bash title="Terminal"
brew install onnxruntime
```

#### Ubuntu/Debian

```bash title="Terminal"
sudo apt install libonnxruntime libonnxruntime-dev
```

#### Windows (MSVC)

**Option 1: Scoop (recommended)**

```bash title="Terminal"
scoop install onnxruntime
```

**Option 2: Manual download**

1. Download from [ONNX Runtime releases](https://github.com/microsoft/onnxruntime/releases)
2. Extract to a directory (e.g., `C:\onnxruntime`)
3. Add the `lib` directory to your `PATH` environment variable
4. Or set `ORT_DYLIB_PATH` to point to `onnxruntime.dll`

### Verification

Verify ONNX Runtime is installed correctly:

```bash title="Terminal"
# Linux
ldconfig -p | grep onnxruntime

# macOS
ls -la /opt/homebrew/lib/libonnxruntime*  # ARM64
ls -la /usr/local/lib/libonnxruntime*      # x86_64

# Windows (PowerShell)
where.exe onnxruntime.dll
```

### Custom Installation Paths

If ONNX Runtime is installed in a non-standard location, set the `ORT_DYLIB_PATH` environment variable:

```bash title="Terminal"
# Linux/macOS
export ORT_DYLIB_PATH=/custom/path/to/libonnxruntime.so

# Windows (PowerShell)
$env:ORT_DYLIB_PATH = "C:\custom\path\to\onnxruntime.dll"
```

### Platform-Specific Notes

#### Windows MSVC (NEW Support)

Embeddings now work on Windows MSVC builds. This was previously unavailable due to the bundled binary approach.

**Requirements:**
- Visual Studio 2019 or later
- ONNX Runtime installed via Scoop or manual download
- MSVC toolchain for Rust builds

#### Windows MinGW (No Embeddings)

Windows MinGW builds (used by Go bindings) still **do not support embeddings** because ONNX Runtime only provides MSVC-compatible libraries.

**Workaround for Go on Windows:**
- Use Windows MSVC Rust toolchain with MSVC Go compiler (experimental)
- Or build Go bindings without embeddings feature

#### Docker/Containerized Deployments

Add ONNX Runtime to your Dockerfile:

**Debian/Ubuntu base:**

```dockerfile title="Dockerfile"
FROM debian:bookworm-slim

# Install ONNX Runtime
RUN apt-get update && apt-get install -y \
    libonnxruntime \
    libonnxruntime-dev \
    && rm -rf /var/lib/apt/lists/*

# Install your Kreuzberg application
COPY . /app
WORKDIR /app
RUN pip install kreuzberg
```

**Alpine base:**

ONNX Runtime is not available in Alpine repositories. Use Debian/Ubuntu base or build from source.

### Troubleshooting

#### Error: "Missing dependency: onnxruntime"

**Cause:** ONNX Runtime is not installed or not in the library search path.

**Solution:**

1. Install ONNX Runtime using platform-specific instructions above
2. Verify installation with verification commands
3. If installed in custom location, set `ORT_DYLIB_PATH`

#### Error: "onnxruntime.dll not found" (Windows)

**Cause:** ONNX Runtime DLL is not in PATH or `ORT_DYLIB_PATH`.

**Solution:**

1. Add ONNX Runtime `lib` directory to PATH
2. Or set `ORT_DYLIB_PATH` to the full path to `onnxruntime.dll`
3. Restart your terminal/IDE after changing PATH

#### Error: "libonnxruntime.so: cannot open shared object file" (Linux)

**Cause:** Library not found by dynamic linker.

**Solution:**

1. Run `sudo ldconfig` after installing ONNX Runtime
2. Or add library path to `LD_LIBRARY_PATH`:
   ```bash title="Terminal"
   export LD_LIBRARY_PATH=/usr/lib:/usr/local/lib:$LD_LIBRARY_PATH
   ```

#### Error: "Library not loaded: @rpath/libonnxruntime.dylib" (macOS)

**Cause:** ONNX Runtime library not in dynamic linker search path.

**Solution:**

1. Install via Homebrew (recommended): `brew install onnxruntime`
2. Or set `DYLD_FALLBACK_LIBRARY_PATH`:
   ```bash title="Terminal"
   export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib:/usr/local/lib
   ```

#### Embeddings work in development but fail in production

**Cause:** ONNX Runtime installed locally but missing in production environment.

**Solution:**

1. Add ONNX Runtime to production dependencies (Docker, system packages)
2. Document ONNX Runtime requirement in deployment guides
3. Add verification step to CI/CD pipeline

### Rollback Plan

If you encounter issues with v4, you can roll back to v3:

```bash title="Terminal"
# Python
pip install kreuzberg==3.22.0

# Rust
kreuzberg = "=3.22.0"

# TypeScript
npm install @kreuzberg/node@3.22.0

# Ruby
gem install kreuzberg -v 3.22.0

# Java
<version>3.22.0</version>

# Go
go get github.com/kreuzberg-dev/kreuzberg/packages/go/v4@v3.22.0
```

Report issues at [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues) with:
- Platform and version (OS, architecture)
- ONNX Runtime installation method
- Full error message and stack trace
- Output of verification commands

---

## Overview of Changes

v4 introduces several major changes:

- **Rust Core**: Complete rewrite of core extraction logic in Rust for significant performance improvements
- **Multi-Language Support**: Native support for Python, TypeScript, and Rust
- **Plugin System**: Trait-based plugin architecture for extensibility
- **Type Safety**: Improved type definitions across all languages
- **Breaking API Changes**: Several API changes for consistency and better ergonomics

## Quick Migration Checklist

- [ ] Update dependencies to v4
- [ ] Update import statements (some modules reorganized)
- [ ] Update configuration (new dataclasses/types)
- [ ] Update error handling (exception hierarchy changed)
- [ ] Migrate custom extractors to new plugin system
- [ ] Test thoroughly (behavior may differ in edge cases)

## Installation

### Python

```bash title="Terminal"
# Install v3 (deprecated)
pip install kreuzberg==3.x

# Install v4 (current)
pip install kreuzberg>=4.0

# Install with all optional features
pip install "kreuzberg[all]"
```

### TypeScript (New in v4)

```bash title="Terminal"
npm install @kreuzberg/node
```

### Rust (New in v4)

```toml title="Cargo.toml"
[dependencies]
kreuzberg = "4.0"
```

## API Changes

### Python API

#### Import Changes

```python title="Python"
# v3 imports
from kreuzberg import extract_file, ExtractionConfig

# v4 imports (same public API, internal structure changed)
from kreuzberg import extract_file, ExtractionConfig
```

#### Configuration Changes

```python title="Python"
# v3 configuration (flat structure)
from kreuzberg import ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    use_quality_processing=True,
)

# v4 configuration (nested dataclasses)
from kreuzberg import ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),
    enable_quality_processing=True,
)
```

#### Batch Processing

```python title="Python"
# v3 batch extraction
from kreuzberg import batch_extract

results = batch_extract(["file1.pdf", "file2.pdf"])

# v4 batch extraction (renamed function)
from kreuzberg import batch_extract_files

results = batch_extract_files(["file1.pdf", "file2.pdf"])
```

#### Error Handling

```python title="Python"
# v3 error handling (single exception type)
from kreuzberg import KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4 error handling (typed exception hierarchy)
from kreuzberg import KreuzbergError, ParsingError, ValidationError

try:
    result = extract_file("doc.pdf")
except ParsingError as e:
    print(f"Parsing error: {e}")
except ValidationError as e:
    print(f"Validation error: {e}")
except KreuzbergError as e:
    print(f"Error: {e}")
```

#### OCR Configuration

```python title="Python"
# v3 OCR configuration (flat parameters)
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
)

# v4 OCR configuration (structured backend configuration)
from kreuzberg import OcrConfig, TesseractConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=3,
        ),
    ),
)
```

#### Complete Configuration (v4)

v4 provides extensive configuration options across all features:

```python title="Python"
from kreuzberg import (
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ChunkingConfig,
    ImageExtractionConfig,
    PdfConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
    PostProcessorConfig,
)

config = ExtractionConfig(
    use_cache=True,
    enable_quality_processing=True,
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(
            psm=6,
            oem=3,
        ),
    ),
    force_ocr=False,
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
        min_dpi=72,
    ),
    pdf_options=PdfConfig(
        extract_images=True,
        passwords=["password1", "password2"],
        extract_metadata=True,
    ),
    token_reduction=TokenReductionConfig(
        mode="moderate",
        preserve_important_words=True,
    ),
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.7,
        detect_multiple=True,
    ),
    postprocessor=PostProcessorConfig(
        enabled=True,
    ),
)
```

#### Metadata Access

```python title="Python"
# v3 metadata access (dictionary-based)
result = extract_file("doc.pdf")
if "pdf" in result.metadata:
    pages = result.metadata["pdf"]["page_count"]

# v4 metadata access (typed attributes)
result = extract_file("doc.pdf")
if result.metadata.pdf:
    pages = result.metadata.pdf.page_count
```

### TypeScript API (New in v4)

TypeScript support is brand new in v4:

```typescript title="TypeScript"
import {
    extractFile,
    extractFileSync,
    ExtractionConfig,
    OcrConfig,
} from '@kreuzberg/node';

const result = await extractFile('document.pdf');

const result2 = extractFileSync('document.pdf');

const config = new ExtractionConfig({
    ocr: new OcrConfig({
        backend: 'tesseract',
        language: 'eng',
    }),
});

const result3 = await extractFile('document.pdf', null, config);
```

### Rust API (New in v4)

The Rust core is now available as a standalone library:

```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Content: {}", result.content);
    Ok(())
}
```

## Feature Changes

### Custom Extractors

v3 had limited support for custom extractors. v4 introduces a comprehensive plugin system.

#### Python

```python title="Python"
from kreuzberg import register_document_extractor

class CustomExtractor:
    def name(self) -> str:
        return "custom"

    def supported_mime_types(self) -> list[str]:
        return ["application/x-custom"]

    def extract(self, data: bytes, mime_type: str, config) -> ExtractionResult:
        return ExtractionResult(content="extracted text", mime_type=mime_type)

register_document_extractor(CustomExtractor())
```

#### TypeScript

```typescript title="TypeScript"
import { registerPostProcessor, PostProcessorProtocol } from '@kreuzberg/node';

class CustomProcessor implements PostProcessorProtocol {
    name(): string {
        return 'custom';
    }

    process(result: ExtractionResult): ExtractionResult {
        return result;
    }
}

registerPostProcessor(new CustomProcessor());
```

### OCR Backends

```python title="Python"
# v3 OCR (Tesseract only)
config = ExtractionConfig(enable_ocr=True)

# v4 Tesseract backend
from kreuzberg import OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng")
)

# v4 EasyOCR backend (requires kreuzberg[easyocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

# v4 PaddleOCR backend (requires kreuzberg[paddleocr])
config = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")
)

# v4 custom OCR backend
from kreuzberg import register_ocr_backend

class MyOCR:
    def name(self) -> str:
        return "my_ocr"

    def extract_text(self, image: bytes, language: str) -> str:
        return "extracted text from custom OCR"

register_ocr_backend(MyOCR())
```

### Language Detection

```python title="Python"
# v3 language detection (not available)

# v4 automatic language detection
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        min_confidence=0.7,
    ),
)

result = extract_file("document.pdf", config=config)
print(result.detected_languages)
```

### Chunking

```python title="Python"
# v3 manual chunking
result = extract_file("doc.pdf")
chunks = [result.content[i:i+1000] for i in range(0, len(result.content), 1000)]

# v4 built-in chunking with overlap support
from kreuzberg import ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_chars=1000,
        max_overlap=100,
    ),
)

result = extract_file("doc.pdf", config=config)
for chunk in result.chunks:
    print(f"Chunk: {len(chunk)} chars")
```

### Password-Protected PDFs

```python title="Python"
# v3 password-protected PDFs (not supported)

# v4 password support (requires kreuzberg[crypto])
from kreuzberg import PdfConfig

config = ExtractionConfig(
    pdf_options=PdfConfig(
        passwords=["password1", "password2"],
        extract_metadata=True,
    ),
)

result = extract_file("encrypted.pdf", config=config)
```

### Token Reduction

```python title="Python"
# v3 token reduction (not available)

# v4 token reduction for LLM processing
from kreuzberg import TokenReductionConfig

config = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        mode="aggressive",
        preserve_important_words=True,
    ),
)

result = extract_file("document.pdf", config=config)
```

### Extract from Bytes

```python title="Python"
# v3 bytes extraction (limited support)

# v4 comprehensive bytes extraction API
from kreuzberg import extract_bytes, extract_bytes_sync

with open("document.pdf", "rb") as f:
    data = f.read()

result = extract_bytes_sync(data, "application/pdf")

import asyncio
result = asyncio.run(extract_bytes(data, "application/pdf"))

result = extract_bytes_sync(data, None)
```

### Table Extraction

```python title="Python"
# v3 table extraction (limited support, mixed into content)
result = extract_file("doc.pdf")

# v4 structured table extraction
result = extract_file("doc.pdf")
for table in result.tables:
    print(table.markdown)
    print(table.cells)
```

## Performance Improvements

v4 delivers significant performance improvements over v3 through its Rust-first architecture:

**Key Performance Enhancements:**

- **Rust core implementation** – Native compilation with LLVM optimizations
- **Streaming parsers** – Constant memory usage for large files (GB+)
- **Zero-copy operations** – Efficient memory management with ownership model
- **SIMD text processing** – Parallel operations for hot paths
- **Async concurrency** – True parallelism without GIL limitations
- **Smart caching** – Content-based deduplication

See the [Performance Guide](../concepts/performance.md) for detailed explanations of optimization techniques and architecture benefits.

## New Features in v4

### Plugin System

Four plugin types:

1. **DocumentExtractor** - Custom file format extractors
2. **OcrBackend** - Custom OCR engines
3. **PostProcessor** - Data transformation and enrichment
4. **Validator** - Fail-fast validation

### Multi-Language Support

v4 provides native APIs for:

- **Python** - PyO3 bindings
- **TypeScript/Node.js** - NAPI-RS bindings
- **Rust** - Direct library usage

### Configuration Discovery

```python title="Python"
# v4 automatic config discovery
result = extract_file("doc.pdf")

# v4 manual config loading
from kreuzberg import load_config

config = load_config("custom-config.toml")
result = extract_file("doc.pdf", config=config)
```

### Image Extraction

```python title="Python"
# v3 basic image extraction

# v4 advanced image extraction with DPI control
from kreuzberg import ImageExtractionConfig

config = ExtractionConfig(
    images=ImageExtractionConfig(
        extract_images=True,
        target_dpi=300,
        max_image_dimension=4096,
        auto_adjust_dpi=True,
        min_dpi=72,
    ),
)

result = extract_file("document.pdf", config=config)
```

### API Server

```bash title="Terminal"
# v3 API server (not available)

# v4 install REST API server
pip install "kreuzberg[api]"
python -m kreuzberg serve --host 0.0.0.0 --port 8000

# v4 CLI binary server
kreuzberg serve --port 8000

# v4 Docker server
docker run -p 8000:8000 ghcr.io/kreuzberg-dev/kreuzberg:latest
```

### MCP Server

```bash title="Terminal"
# v3 MCP server (not available)

# v4 Model Context Protocol server
python -m kreuzberg mcp

# v4 CLI binary MCP server
kreuzberg mcp
```

## Breaking Changes

### Metadata Field Names: `date` → `created_at`

The legacy `date` field in metadata has been replaced with `created_at` for consistency across all document formats.

#### What Changed

- **Old (deprecated)**: `metadata.date` - Generic date field with ambiguous meaning
- **New (standard)**: `metadata.created_at` - Document creation timestamp (ISO 8601 format)
- **Also available**: `metadata.modified_at` - Last modification timestamp (ISO 8601 format)

The `date` field was inconsistently used across different document formats. The new `created_at` and `modified_at` fields provide clear semantics that match industry standards.

#### Migration Guide

**Rust:**
```rust title="Rust"
// Before (v3/early v4)
if let Some(date) = metadata.date {
    println!("Date: {}", date);
}

// After (v4.0.0+)
if let Some(created_at) = metadata.created_at {
    println!("Created: {}", created_at);
}
if let Some(modified_at) = metadata.modified_at {
    println!("Modified: {}", modified_at);
}
```

**Python:**
```python title="Python"
# Before (v3/early v4)
date = result.metadata.get("date")
if date:
    print(f"Date: {date}")

# After (v4.0.0+)
created_at = result.metadata.get("created_at")
if created_at:
    print(f"Created: {created_at}")

modified_at = result.metadata.get("modified_at")
if modified_at:
    print(f"Modified: {modified_at}")
```

**TypeScript:**
```typescript title="TypeScript"
// Before (v3/early v4)
if (metadata.date) {
    console.log("Date:", metadata.date);
}

// After (v4.0.0+)
if (metadata.createdAt) {
    console.log("Created:", metadata.createdAt);
}
if (metadata.modifiedAt) {
    console.log("Modified:", metadata.modifiedAt);
}
```

**Java:**
```java title="Java"
// Before (v3/early v4)
metadata.date().ifPresent(date ->
    System.out.println("Date: " + date)
);

// After (v4.0.0+)
metadata.createdAt().ifPresent(created ->
    System.out.println("Created: " + created)
);
metadata.modifiedAt().ifPresent(modified ->
    System.out.println("Modified: " + modified)
);
```

**Go:**
```go title="Go"
// Before (v3/early v4)
if metadata.Date != nil {
    fmt.Println("Date:", *metadata.Date)
}

// After (v4.0.0+)
if metadata.CreatedAt != nil {
    fmt.Println("Created:", *metadata.CreatedAt)
}
if metadata.ModifiedAt != nil {
    fmt.Println("Modified:", *metadata.ModifiedAt)
}
```

**Ruby:**
```ruby title="Ruby"
# Before (v3/early v4)
if result.metadata["date"]
  puts "Date: #{result.metadata["date"]}"
end

# After (v4.0.0+)
if result.metadata["created_at"]
  puts "Created: #{result.metadata["created_at"]}"
end
if result.metadata["modified_at"]
  puts "Modified: #{result.metadata["modified_at"]}"
end
```

**C#:**
```csharp title="C#"
// Before (v3/early v4)
if (metadata.Date != null)
{
    Console.WriteLine($"Date: {metadata.Date}");
}

// After (v4.0.0+)
if (metadata.CreatedAt != null)
{
    Console.WriteLine($"Created: {metadata.CreatedAt}");
}
if (metadata.ModifiedAt != null)
{
    Console.WriteLine($"Modified: {metadata.ModifiedAt}");
}
```

#### Format-Specific Metadata

Note that format-specific metadata (like `PdfMetadata`) may have their own date fields with more specific names:

- `PdfMetadata.creation_date` - PDF document creation date (from PDF metadata)
- `PdfMetadata.modification_date` - PDF document modification date (from PDF metadata)
- Top-level `Metadata.created_at` and `Metadata.modified_at` - Normalized across all formats

The format-specific fields preserve the original metadata from the document, while the top-level fields provide a consistent interface across all document types.

### Page Tracking and Byte Offsets

v4 introduces a complete redesign of page tracking and text positioning with several critical breaking changes:

#### Field Renames: Character to Byte Offsets

The most significant change is the shift from character indices to UTF-8 byte positions. This change improves correctness and performance:

- `char_start` → `byte_start`
- `char_end` → `byte_end`

**Why this changed**: Character indices are ambiguous with multi-byte UTF-8 sequences. Modern text processing requires byte-accurate positioning for proper UTF-8 safety. This is essential when working with embeddings, language models, or any text processing that requires precise character location tracking.

#### ChunkMetadata New Fields

`ChunkMetadata` now includes explicit page range tracking:

```python title="Python"
# v4 ChunkMetadata structure
class ChunkMetadata:
    byte_start: int      # Byte offset where chunk starts (UTF-8 valid boundary)
    byte_end: int        # Byte offset where chunk ends (UTF-8 valid boundary)
    byte_length: int     # byte_end - byte_start
    chunk_index: int     # 0-based chunk position
    total_chunks: int    # Total chunks in document
    first_page: int | None   # First page this chunk spans (1-indexed, when tracking enabled)
    last_page: int | None    # Last page this chunk spans (1-indexed, when tracking enabled)
    token_count: int | None  # Token count from embeddings
```

#### New Page Tracking Types

v4 introduces structured page representation:

```python title="Python"
# PageStructure - Overall page metadata
class PageStructure:
    total_count: int           # Total pages/slides/sheets
    unit_type: PageUnitType    # "page", "slide", or "sheet"
    boundaries: list[PageBoundary] | None    # Byte offsets per page
    pages: list[PageInfo] | None             # Per-page metadata

# PageBoundary - Byte offset range for a page
class PageBoundary:
    byte_start: int    # Byte offset where page starts (inclusive)
    byte_end: int      # Byte offset where page ends (exclusive)
    page_number: int   # 1-indexed page number

# PageInfo - Metadata for a single page
class PageInfo:
    number: int                # 1-indexed page number
    title: str | None          # Page/slide title
    dimensions: (float, float) | None  # Width, height
    image_count: int | None    # Images on this page
    table_count: int | None    # Tables on this page
    hidden: bool | None        # Visibility state

# PageContent - Per-page content (when extract_pages=true)
class PageContent:
    page_number: int           # 1-indexed
    content: str               # Text for this page
    tables: list[Table]        # Tables on this page
    images: list[ExtractedImage]  # Images on this page

# PageUnitType - Distinguishes page types
enum PageUnitType:
    Page    # Standard document pages
    Slide   # Presentation slides
    Sheet   # Spreadsheet sheets
```

#### New PageConfig Options

Enable page tracking through the extraction configuration:

```python title="Python"
# v4 PageConfig structure
class PageConfig:
    extract_pages: bool = False          # Extract pages as separate ExtractionResult.pages array
    insert_page_markers: bool = False    # Insert markers in main content string
    marker_format: str = "\n\n<!-- PAGE {page_num} -->\n\n"  # Marker template
```

#### Code Migration Examples

##### Rust

**Before (v3):**
```rust title="Rust"
// v3 - Character indices (no longer available)
// Not directly comparable as v3 had different architecture
```

**After (v4):**
```rust title="Rust"
use kreuzberg::{extract_file_sync, ExtractionConfig, PageConfig};

let config = ExtractionConfig {
    pages: Some(PageConfig {
        extract_pages: true,
        insert_page_markers: false,
        marker_format: "\n\n<!-- PAGE {page_num} -->\n\n".to_string(),
    }),
    ..Default::default()
};

let result = extract_file_sync("document.pdf", None, &config)?;

// Access page tracking in chunks
for chunk in &result.chunks {
    if let (Some(first), Some(last)) = (chunk.metadata.first_page, chunk.metadata.last_page) {
        println!("Chunk spans pages {} to {}", first, last);
    }

    // Byte offsets are UTF-8 safe
    let chunk_text = &result.content[chunk.metadata.byte_start..chunk.metadata.byte_end];
    println!("Chunk content: {}", chunk_text);
}

// Extract per-page content
for page in &result.pages {
    println!("Page {}: {} bytes", page.page_number, page.content.len());
}
```

##### Python

**Before (v3):**
```python title="Python"
# v3 - Used char_start/char_end (now removed)
result = extract_file("document.pdf")
for chunk in result.chunks:
    start = chunk.metadata.get("char_start")  # No longer exists!
    end = chunk.metadata.get("char_end")
```

**After (v4):**
```python title="Python"
from kreuzberg import extract_file, ExtractionConfig, PageConfig

config = ExtractionConfig(
    pages=PageConfig(
        extract_pages=True,
        insert_page_markers=False,
        marker_format="\n\n<!-- PAGE {page_num} -->\n\n",
    ),
)

result = extract_file("document.pdf", config=config)

# Access byte-based offsets and page tracking
for chunk in result.chunks:
    byte_start = chunk.metadata.byte_start    # UTF-8 byte offset
    byte_end = chunk.metadata.byte_end

    # Extract chunk text using byte offsets
    chunk_text = result.content[byte_start:byte_end]

    # Check page range
    if chunk.metadata.first_page is not None:
        first = chunk.metadata.first_page
        last = chunk.metadata.last_page
        print(f"Chunk spans pages {first} to {last}")

# Extract per-page content
for page in result.pages:
    print(f"Page {page.page_number}: {len(page.content)} characters")
    for table in page.tables:
        print(f"  - Table with {len(table.cells)} cells")
```

##### TypeScript

**Before (v3):**
```typescript title="TypeScript"
// v3 - Character indices
const result = await extractFile("document.pdf");
// char_start and char_end no longer available
```

**After (v4):**
```typescript title="TypeScript"
import {
    extractFile,
    ExtractionConfig,
    PageConfig,
} from '@kreuzberg/node';

const config = new ExtractionConfig({
    pages: new PageConfig({
        extractPages: true,
        insertPageMarkers: false,
        markerFormat: "\n\n<!-- PAGE {page_num} -->\n\n",
    }),
});

const result = await extractFile("document.pdf", null, config);

// Access byte offsets and page tracking
for (const chunk of result.chunks) {
    const byteStart = chunk.metadata.byteStart;    // UTF-8 byte offset
    const byteEnd = chunk.metadata.byteEnd;

    // Extract chunk text
    const chunkText = result.content.substring(byteStart, byteEnd);

    // Check page range
    if (chunk.metadata.firstPage !== null) {
        console.log(`Chunk spans pages ${chunk.metadata.firstPage} to ${chunk.metadata.lastPage}`);
    }
}

// Extract per-page content
for (const page of result.pages) {
    console.log(`Page ${page.pageNumber}: ${page.content.length} characters`);
}
```

##### Java

**Before (v3):**
```java title="Java"
// v3 - Character-based tracking
// Not directly comparable as v3 used different architecture
```

**After (v4):**
```java title="Java"
import com.kreuzberg.*;

ExtractionConfig config = new ExtractionConfig.Builder()
    .withPageConfig(new PageConfig.Builder()
        .extractPages(true)
        .insertPageMarkers(false)
        .markerFormat("\n\n<!-- PAGE {page_num} -->\n\n")
        .build())
    .build();

ExtractionResult result = Kreuzberg.extractFile("document.pdf", null, config);

// Access byte offsets and page tracking
for (Chunk chunk : result.getChunks()) {
    int byteStart = chunk.getMetadata().getByteStart();
    int byteEnd = chunk.getMetadata().getByteEnd();

    // Extract chunk text
    String chunkText = result.getContent().substring(byteStart, byteEnd);

    // Check page range
    if (chunk.getMetadata().getFirstPage() != null) {
        int firstPage = chunk.getMetadata().getFirstPage();
        int lastPage = chunk.getMetadata().getLastPage();
        System.out.printf("Chunk spans pages %d to %d%n", firstPage, lastPage);
    }
}

// Extract per-page content
for (PageContent page : result.getPages()) {
    System.out.printf("Page %d: %d characters%n", page.getPageNumber(), page.getContent().length());
}
```

##### Go

**Before (v3):**
```go title="Go"
// v3 - Character indices
// Not directly comparable
```

**After (v4):**
```go title="Go"
package main

import (
    "fmt"
    "log"

    "github.com/kreuzberg/kreuzberg-go/kreuzberg"
)

func main() {
    config := &kreuzberg.ExtractionConfig{
        Pages: &kreuzberg.PageConfig{
            ExtractPages:       true,
            InsertPageMarkers:  false,
            MarkerFormat:       "\n\n<!-- PAGE {page_num} -->\n\n",
        },
    }

    result, err := kreuzberg.ExtractFile("document.pdf", nil, config)
    if err != nil {
        log.Fatal(err)
    }

    // Access byte offsets and page tracking
    for _, chunk := range result.Chunks {
        byteStart := chunk.Metadata.ByteStart
        byteEnd := chunk.Metadata.ByteEnd

        // Extract chunk text
        chunkText := result.Content[byteStart:byteEnd]

        // Check page range
        if chunk.Metadata.FirstPage != nil {
            fmt.Printf("Chunk spans pages %d to %d\n",
                *chunk.Metadata.FirstPage, *chunk.Metadata.LastPage)
        }
    }

    // Extract per-page content
    for _, page := range result.Pages {
        fmt.Printf("Page %d: %d characters\n", page.PageNumber, len(page.Content))
    }
}
```

##### Ruby

**After (v4):**
```ruby title="Ruby"
require 'kreuzberg'

config = Kreuzberg::ExtractionConfig.new(
  pages: Kreuzberg::PageConfig.new(
    extract_pages: true,
    insert_page_markers: false,
    marker_format: "\n\n<!-- PAGE {page_num} -->\n\n"
  )
)

result = Kreuzberg.extract_file("document.pdf", nil, config)

# Access byte offsets and page tracking
result.chunks.each do |chunk|
  byte_start = chunk.metadata.byte_start
  byte_end = chunk.metadata.byte_end

  # Extract chunk text
  chunk_text = result.content[byte_start...byte_end]

  # Check page range
  if chunk.metadata.first_page
    puts "Chunk spans pages #{chunk.metadata.first_page} to #{chunk.metadata.last_page}"
  end
end

# Extract per-page content
result.pages.each do |page|
  puts "Page #{page.page_number}: #{page.content.length} characters"
end
```

##### C#

**After (v4):**
```csharp title="C#"
using Kreuzberg;

var config = new ExtractionConfig
{
    Pages = new PageConfig
    {
        ExtractPages = true,
        InsertPageMarkers = false,
        MarkerFormat = "\n\n<!-- PAGE {page_num} -->\n\n",
    },
};

var result = Kreuzberg.ExtractFile("document.pdf", null, config);

// Access byte offsets and page tracking
foreach (var chunk in result.Chunks)
{
    int byteStart = chunk.Metadata.ByteStart;
    int byteEnd = chunk.Metadata.ByteEnd;

    // Extract chunk text
    string chunkText = result.Content.Substring(byteStart, byteEnd - byteStart);

    // Check page range
    if (chunk.Metadata.FirstPage.HasValue)
    {
        Console.WriteLine($"Chunk spans pages {chunk.Metadata.FirstPage} to {chunk.Metadata.LastPage}");
    }
}

// Extract per-page content
foreach (var page in result.Pages)
{
    Console.WriteLine($"Page {page.PageNumber}: {page.Content.Length} characters");
}
```

#### Impact Summary

| Item | v3 | v4 | Impact |
|------|----|----|--------|
| **Offset Type** | Character indices (ambiguous) | UTF-8 byte positions | Code must use byte offsets; more correct for embeddings |
| **Field Names** | `char_start`, `char_end` | `byte_start`, `byte_end` | Search and replace in code |
| **Page Tracking** | Not available | Always available when boundaries exist | Access `first_page`, `last_page` in metadata |
| **Per-Page Content** | Not available | `ExtractionResult.pages` array | New `PageContent` structures |
| **Page Config** | N/A | New `PageConfig` struct | Optional; enable with extraction config |
| **Boundary Tracking** | N/A | `PageStructure.boundaries` | Maps byte ranges to page numbers |

#### Migration Checklist

- [ ] Replace all `char_start` references with `byte_start`
- [ ] Replace all `char_end` references with `byte_end`
- [ ] Update code that accesses chunk position metadata
- [ ] Test text extraction with multi-byte UTF-8 characters (emoji, CJK, etc.)
- [ ] Enable page tracking if needed via `PageConfig`
- [ ] Update any code that relies on absolute character positions (e.g., for embeddings)
- [ ] Review performance implications (byte offsets are faster)

### Configuration Structure

v3 used flat configuration. v4 uses nested dataclasses:

```python title="Python"
# v3 flat configuration
config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
    ocr_psm=6,
    use_cache=True,
)

# v4 nested dataclasses
config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(psm=6),
    ),
    use_cache=True,
)
```

### Metadata Structure

v3 used dictionaries. v4 uses typed dataclasses:

```python title="Python"
# v3 dictionary-based metadata
pages = result.metadata["pdf"]["page_count"]

# v4 typed dataclass metadata
pages = result.metadata.pdf.page_count
```

### Error Hierarchy

```python title="Python"
# v3 exception hierarchy
KreuzbergException (base)

# v4 exception hierarchy
KreuzbergError (base)
├── ValidationError
├── ParsingError
├── OCRError
├── MissingDependencyError
├── PluginError
└── ConfigurationError
```

### Function Names

| v3 | v4 |
|----|----|
| `batch_extract()` | `batch_extract_files()` |
| `extract_bytes()` | `extract_bytes()` (same) |
| `extract_file()` | `extract_file()` (same) |

### Removed Features

#### GMFT (Give Me Formatted Tables)
v3's vision-based table extraction using TATR models. Replaced with Tesseract OCR table detection:

```python title="Python"
# v4 Tesseract table detection
config = ExtractionConfig(
    ocr=OcrConfig(
        tesseract_config=TesseractConfig(enable_table_detection=True)
    )
)
result = extract_file("doc.pdf", config=config)
```

#### Entity Extraction, Keyword Extraction, Document Classification
Removed. Use external libraries (spaCy, KeyBERT, etc.) with postprocessors if needed.

#### HTMLToMarkdownConfig Replaced with html_options

v3 provided `HTMLToMarkdownConfig` for customizing HTML-to-Markdown conversion.

**v4 replaces this with `html_options`**, which provides comprehensive configuration for HTML-to-Markdown conversion using the `html-to-markdown-rs` library. The new configuration offers more options and better control than v3.

**Available options in `html_options`:**

- `heading_style` - Heading format: "atx" (#), "underlined" (===), or "atx_closed" (# ... #)
- `list_indent_type` - List indentation: "spaces" or "tabs"
- `list_indent_width` - Number of spaces/tabs for list indentation
- `code_block_style` - Code block format: "indented", "backticks", or "tildes"
- `highlight_style` - Mark element style: "double_equal", "html", "bold", or "none"
- `whitespace_mode` - Whitespace handling: "normalized" or "strict"
- `newline_style` - Line break style: "spaces" (two spaces) or "backslash"
- `preprocessing` - HTML preprocessing options (remove navigation, forms, etc.)
- `extract_metadata` - Whether to extract document metadata
- `autolinks` - Enable automatic linking for bare URLs
- `wrap` - Enable text wrapping
- `wrap_width` - Line width for text wrapping (default: 80)
- And many more...

**Migration:**
```python title="Python"
# v3 with HTMLToMarkdownConfig
from kreuzberg import ExtractionConfig, HTMLToMarkdownConfig

config = ExtractionConfig(
    html_to_markdown=HTMLToMarkdownConfig(
        extract_metadata=True,
        preprocess_html=True,
    )
)

# v4 with html_options
from kreuzberg import ExtractionConfig, HtmlOptions, PreprocessingOptions

config = ExtractionConfig(
    html_options=HtmlOptions(
        extract_metadata=True,
        heading_style="atx",
        code_block_style="backticks",
        preprocessing=PreprocessingOptions(
            enabled=True,
            remove_navigation=True,
            remove_forms=True,
        ),
    )
)

# v4 - defaults work well for most cases
config = ExtractionConfig()  # Uses sensible defaults
```

**Rust example:**
```rust title="Rust"
use kreuzberg::{ExtractionConfig, extract_file_sync};
use html_to_markdown_rs::{ConversionOptions, HeadingStyle, CodeBlockStyle};

let config = ExtractionConfig {
    html_options: Some(ConversionOptions {
        heading_style: HeadingStyle::Atx,
        code_block_style: CodeBlockStyle::Backticks,
        extract_metadata: true,
        ..Default::default()
    }),
    ..Default::default()
};

let result = extract_file_sync("document.html", None, &config)?;
```

#### Other
- **ExtractorRegistry**: Custom extractors must be Rust plugins
- **JSONExtractionConfig**: Now uses defaults
- **ImageOCRConfig**: Replaced by `ImageExtractionConfig`

## Migration Examples

### Basic Extraction

```python title="Python"
# v3 basic extraction
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result["content"])
print(result["metadata"])

# v4 basic extraction
from kreuzberg import extract_file

result = extract_file("document.pdf")
print(result.content)
print(result.metadata)
```

### OCR Extraction

```python title="Python"
# v3 OCR extraction
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(
    enable_ocr=True,
    ocr_language="eng",
)

result = extract_file("scanned.pdf", config=config)

# v4 OCR extraction
from kreuzberg import extract_file, ExtractionConfig, OcrConfig

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
    ),
)

result = extract_file("scanned.pdf", config=config)
```

### Batch Processing

```python title="Python"
# v3 batch processing
from kreuzberg import batch_extract

results = batch_extract(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result["content"])

# v4 batch processing
from kreuzberg import batch_extract_files

results = batch_extract_files(["doc1.pdf", "doc2.pdf", "doc3.pdf"])
for result in results:
    print(result.content)
```

### Error Handling

```python title="Python"
# v3 error handling
from kreuzberg import extract_file, KreuzbergException

try:
    result = extract_file("doc.pdf")
except KreuzbergException as e:
    print(f"Error: {e}")

# v4 error handling
from kreuzberg import extract_file, KreuzbergError, ParsingError

try:
    result = extract_file("doc.pdf")
except ParsingError as e:
    print(f"Parsing error: {e}")
except KreuzbergError as e:
    print(f"Error: {e}")
```

## Testing Your Migration

### Automated Testing

```python title="Python"
import pytest
from kreuzberg import extract_file, ExtractionConfig

def test_basic_extraction():
    result = extract_file("tests/fixtures/sample.pdf")
    assert result.content
    assert result.mime_type == "application/pdf"

def test_ocr_extraction():
    from kreuzberg import OcrConfig

    config = ExtractionConfig(
        ocr=OcrConfig(backend="tesseract", language="eng"),
    )

    result = extract_file("tests/fixtures/scanned.pdf", config=config)
    assert result.content
    assert result.metadata.ocr

def test_batch_processing():
    from kreuzberg import batch_extract_files

    files = ["tests/fixtures/doc1.pdf", "tests/fixtures/doc2.pdf"]
    results = batch_extract_files(files)

    assert len(results) == 2
    for result in results:
        assert result.content

def test_error_handling():
    from kreuzberg import ParsingError

    with pytest.raises(ParsingError):
        extract_file("tests/fixtures/corrupted.pdf")
```

### Performance Testing

```python title="Python"
import time
from kreuzberg import extract_file, batch_extract_files

start = time.time()
result = extract_file("large_document.pdf")
print(f"Single file: {time.time() - start:.2f}s")

files = [f"document{i}.pdf" for i in range(100)]
start = time.time()
results = batch_extract_files(files)
print(f"Batch (100 files): {time.time() - start:.2f}s")
```

---

## PDF Hierarchy Detection Feature

**Available**: v4.0.0+

PDF Hierarchy Detection is a new feature in v4 that automatically extracts document structure from PDFs using K-means clustering to identify semantic hierarchies of content blocks.

### What's New

The hierarchy detection system provides:

- **Automatic Structure Inference**: No explicit tags or metadata required - detects structure from content characteristics
- **K-means Clustering**: Groups blocks into semantic levels (typically 3-5 levels) representing document hierarchy
- **Confidence Scoring**: Each block assigned a confidence score reflecting hierarchy assignment quality
- **Parent-Child Relationships**: Links blocks in hierarchical relationships for tree-like document representation
- **Block Type Classification**: Labels blocks as title, heading, subheading, paragraph, etc. based on semantic level
- **Per-Page Hierarchies**: Separate hierarchy detected for each page in multi-page documents

### Default Behavior

**PDF Hierarchy Detection is enabled by default when page extraction is enabled.** This means when you enable `pages=PageConfig(extract_pages=True)`, hierarchy detection automatically runs on each page's content.

To explicitly control hierarchy detection:

```python title="Python"
from kreuzberg import ExtractionConfig, PageConfig, PdfConfig, HierarchyDetectionConfig

# Explicitly enable hierarchy detection
config = ExtractionConfig(
    pages=PageConfig(extract_pages=True),
    pdf_options=PdfConfig(
        hierarchy_detection=HierarchyDetectionConfig(enabled=True)
    )
)

# Disable hierarchy detection
config = ExtractionConfig(
    pages=PageConfig(extract_pages=True),
    pdf_options=PdfConfig(
        hierarchy_detection=HierarchyDetectionConfig(enabled=False)
    )
)
```

### How to Access Hierarchy Output

Hierarchy blocks are available through the `page.hierarchy.blocks` structure when page extraction is enabled:

#### Python

```python title="Python"
from kreuzberg import extract_file, ExtractionConfig, PageConfig, PdfConfig, HierarchyDetectionConfig

config = ExtractionConfig(
    pages=PageConfig(extract_pages=True),
    pdf_options=PdfConfig(
        hierarchy_detection=HierarchyDetectionConfig(enabled=True)
    )
)

result = extract_file("document.pdf", config=config)

# Access hierarchy blocks for each page
for page in result.pages:
    print(f"Page {page.page_number}:")

    # Access hierarchy structure
    if page.hierarchy:
        for block in page.hierarchy.blocks:
            indent = "  " * (block.level - 1)
            print(f"{indent}[{block.block_type}] {block.text[:50]}")
            print(f"{indent}  Level: {block.level}, Confidence: {block.confidence:.2f}")

            # Access parent block if exists
            if block.parent_index is not None:
                parent = page.hierarchy.blocks[block.parent_index]
                print(f"{indent}  Parent: {parent.text[:30]}")
```

#### TypeScript

```typescript title="TypeScript"
import { extractFile, ExtractionConfig, PageConfig, PdfConfig, HierarchyDetectionConfig } from '@kreuzberg/node';

const config = new ExtractionConfig({
    pages: new PageConfig({ extractPages: true }),
    pdfOptions: new PdfConfig({
        hierarchyDetection: new HierarchyDetectionConfig({ enabled: true })
    })
});

const result = await extractFile("document.pdf", null, config);

// Access hierarchy blocks for each page
for (const page of result.pages) {
    console.log(`Page ${page.pageNumber}:`);

    if (page.hierarchy) {
        for (const block of page.hierarchy.blocks) {
            const indent = "  ".repeat(block.level - 1);
            console.log(`${indent}[${block.blockType}] ${block.text.substring(0, 50)}`);
            console.log(`${indent}  Level: ${block.level}, Confidence: ${block.confidence.toFixed(2)}`);

            if (block.parentIndex !== null) {
                const parent = page.hierarchy.blocks[block.parentIndex];
                console.log(`${indent}  Parent: ${parent.text.substring(0, 30)}`);
            }
        }
    }
}
```

### Hierarchy Block Structure

Each hierarchy block contains:

```python title="Python"
class HierarchyBlock:
    text: str                    # Block content text
    level: int                   # Semantic level (1-N, where 1 is highest level)
    block_type: str              # "title", "heading", "subheading", "paragraph", etc.
    confidence: float            # 0.0-1.0 confidence score
    byte_start: int              # UTF-8 byte offset in page content
    byte_end: int                # UTF-8 byte offset in page content
    parent_index: int | None     # Index of parent block in hierarchy.blocks, None if top-level
    children_indices: list[int]  # Indices of child blocks
    position: BlockPosition      # Position info (x, y, width, height)
    font_info: FontInfo | None   # Font characteristics if available
```

### How to Disable Hierarchy Detection

Hierarchy detection can be disabled globally or per-extraction:

```python title="Python"
from kreuzberg import ExtractionConfig, PageConfig, PdfConfig, HierarchyDetectionConfig

# Option 1: Disable in configuration
config = ExtractionConfig(
    pages=PageConfig(extract_pages=True),
    pdf_options=PdfConfig(
        hierarchy_detection=HierarchyDetectionConfig(enabled=False)
    )
)

result = extract_file("document.pdf", config=config)

# Option 2: Disable via TOML configuration file
# kreuzberg.toml
# [pdf_options.hierarchy_detection]
# enabled = false

# Option 3: Disable via environment variable
import os
os.environ["KREUZBERG_PDF_HIERARCHY_ENABLED"] = "false"
```

### Configuration Options

Fine-tune hierarchy detection behavior:

```python title="Python"
from kreuzberg import HierarchyConfig

config = HierarchyConfig(
    enabled=True,                          # Enable/disable hierarchy detection
    k_clusters=6,                          # Number of clusters for semantic levels (default: 6)
    include_bbox=True,                     # Include bounding box in output (default: True)
    ocr_coverage_threshold=None             # OCR coverage threshold (default: None for auto)
)
```

### Breaking Changes: None

**PDF Hierarchy Detection is completely backward compatible.** No breaking changes are introduced:

- ✓ Existing code continues to work without modification
- ✓ Hierarchy detection is opt-in via configuration (enabled by default when pages are extracted)
- ✓ Existing `page.content` and `page.tables` output unchanged
- ✓ No changes to metadata structure
- ✓ No changes to chunk output format

### Use Cases

#### 1. Building Hierarchical RAG Systems

```python title="Python"
from kreuzberg import extract_file, ExtractionConfig, PageConfig, PdfConfig, HierarchyDetectionConfig, ChunkingConfig

config = ExtractionConfig(
    pages=PageConfig(extract_pages=True),
    pdf_options=PdfConfig(
        hierarchy_detection=HierarchyDetectionConfig(enabled=True)
    ),
    chunking=ChunkingConfig(max_chars=1000)
)

result = extract_file("document.pdf", config=config)

# Build hierarchical knowledge base
knowledge_base = []
for page in result.pages:
    if page.hierarchy:
        for block in page.hierarchy.blocks:
            knowledge_base.append({
                "text": block.text,
                "level": block.level,
                "type": block.block_type,
                "page": page.page_number,
                "confidence": block.confidence,
                "context": get_parent_context(block, page.hierarchy)
            })
```

#### 2. Automatic Table of Contents Generation

```python title="Python"
def generate_toc(result):
    """Generate table of contents from hierarchy."""
    toc = []
    for page in result.pages:
        if page.hierarchy:
            for block in page.hierarchy.blocks:
                if block.block_type in ["heading", "title"]:
                    indent = "  " * (block.level - 1)
                    toc.append(f"{indent}• {block.text} (page {page.page_number})")
    return "\n".join(toc)

toc = generate_toc(result)
print(toc)
```

#### 3. Context-Aware Semantic Chunking

```python title="Python"
def enrich_chunks_with_hierarchy(result):
    """Add hierarchy context to chunks."""
    enriched_chunks = []

    for page in result.pages:
        for chunk in result.chunks:
            # Find which hierarchy block contains this chunk
            if page.hierarchy:
                for block in page.hierarchy.blocks:
                    if block.byte_start <= chunk.metadata.byte_start < block.byte_end:
                        enriched_chunks.append({
                            "content": chunk.text,
                            "hierarchy_context": {
                                "section": block.text,
                                "level": block.level,
                                "type": block.block_type
                            },
                            "metadata": chunk.metadata
                        })

    return enriched_chunks
```

### Performance Characteristics

- **Time Complexity**: O(n·k·i) where n = blocks, k = clusters (typically 3-5), i = iterations
- **Typical Runtime**: 50-200ms for 20-100 block documents, scales linearly
- **Memory Usage**: O(n) - linear with number of blocks
- **GPU Acceleration**: Optional CUDA support for large documents (100+ pages)
- **Caching**: Results cached based on PDF content hash - subsequent extractions are instant

### Frequently Asked Questions

**Q: Does hierarchy detection work with all PDFs?**
A: Yes, it analyzes content structure automatically. Quality improves with well-formatted documents that have consistent styling conventions.

**Q: Can I customize the hierarchy detection algorithm?**
A: Currently, clustering parameters are configurable (min_cluster_size, confidence_threshold). Custom algorithms can be added via the plugin system.

**Q: What if a PDF has inconsistent formatting?**
A: The algorithm is robust to formatting variations. The confidence scores will be lower, but blocks are still assigned to semantic levels.

**Q: How does hierarchy detection interact with OCR?**
A: Hierarchy detection works on extracted blocks. For scanned PDFs with OCR enabled, structure is inferred from OCR results.

**Q: Can I disable hierarchy detection globally?**
A: Yes, set `hierarchy_detection=HierarchyDetectionConfig(enabled=False)` in PdfConfig or use the environment variable `KREUZBERG_PDF_HIERARCHY_ENABLED=false`.

---

## Getting Help

- **Documentation**: [https://docs.kreuzberg.dev](https://docs.kreuzberg.dev)
- **Examples**: See [Python API Reference](../reference/api-python.md), [TypeScript API Reference](../reference/api-typescript.md), [Rust API Reference](../reference/api-rust.md)
- **Issues**: [GitHub Issues](https://github.com/kreuzberg-dev/kreuzberg/issues)
- **Changelog**: [CHANGELOG.md](../CHANGELOG.md)

## Deprecation Timeline

- **v3.x**: Maintenance mode (bug fixes only)
- **v4.0**: Current stable release
- **v3 EOL**: June 2025 (no further updates)

Users should migrate to v4 as soon as possible to benefit from performance improvements and new features.
