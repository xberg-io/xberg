---
name: kreuzberg
description: >-
  Extract text, tables, metadata, and images from 62+ document formats
  (PDF, Office, images, HTML, email, archives, academic) using Kreuzberg.
  Use when writing code that calls Kreuzberg APIs in Python, Node.js/TypeScript,
  Rust, or CLI. Covers installation, extraction (sync/async), configuration
  (OCR, chunking, output format), batch processing, error handling, and plugins.
license: MIT
metadata:
  author: kreuzberg-dev
  version: "1.0"
  repository: https://github.com/kreuzberg-dev/kreuzberg
---

# Kreuzberg Document Extraction

Kreuzberg is a high-performance document intelligence library with a Rust core and native bindings for Python, Node.js/TypeScript, Ruby, Go, Java, C#, PHP, and Elixir. It extracts text, tables, metadata, and images from 62+ file formats including PDF, Office documents, images (with OCR), HTML, email, archives, and academic formats.

Use this skill when writing code that:
- Extracts text or metadata from documents
- Performs OCR on scanned documents or images
- Batch-processes multiple files
- Configures extraction options (output format, chunking, OCR, language detection)
- Implements custom plugins (post-processors, validators, OCR backends)

## Installation

### Python
```bash
pip install kreuzberg
# Optional OCR backends:
pip install kreuzberg[easyocr]    # EasyOCR
pip install kreuzberg[paddleocr]  # PaddleOCR
```

### Node.js
```bash
npm install @kreuzberg/node
```

### Rust
```toml
# Cargo.toml
[dependencies]
kreuzberg = { version = "4", features = ["tokio-runtime"] }
# features: tokio-runtime (required for sync + batch), pdf, ocr, chunking,
#           embeddings, language-detection, keywords-yake, keywords-rake
```

### CLI
```bash
# Download from GitHub releases, or:
cargo install kreuzberg-cli
```

## Quick Start

### Python (Async)
```python
from kreuzberg import extract_file

result = await extract_file("document.pdf")
print(result.content)       # extracted text
print(result.metadata)      # document metadata
print(result.tables)        # extracted tables
```

### Python (Sync)
```python
from kreuzberg import extract_file_sync

result = extract_file_sync("document.pdf")
print(result.content)
```

### Node.js
```typescript
import { extractFile } from '@kreuzberg/node';

const result = await extractFile('document.pdf');
console.log(result.content);
console.log(result.metadata);
console.log(result.tables);
```

### Node.js (Sync)
```typescript
import { extractFileSync } from '@kreuzberg/node';

const result = extractFileSync('document.pdf');
```

### Rust (Async)
```rust
use kreuzberg::{extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file("document.pdf", None, &config).await?;
    println!("{}", result.content);
    Ok(())
}
```

### Rust (Sync) — requires `tokio-runtime` feature
```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```

### CLI
```bash
kreuzberg extract document.pdf
kreuzberg extract document.pdf --format json
kreuzberg extract document.pdf --output-format markdown
```

## Configuration

All languages use the same configuration structure with language-appropriate naming conventions.

### Python (snake_case)
```python
from kreuzberg import (
    ExtractionConfig, OcrConfig, TesseractConfig,
    PdfConfig, ChunkingConfig,
)

config = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(psm=6, enable_table_detection=True),
    ),
    pdf_options=PdfConfig(passwords=["secret123"]),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
    output_format="markdown",
)

result = await extract_file("document.pdf", config=config)
```

### Node.js (camelCase)
```typescript
import { extractFile, type ExtractionConfig } from '@kreuzberg/node';

const config: ExtractionConfig = {
    ocr: { backend: 'tesseract', language: 'eng' },
    pdfOptions: { passwords: ['secret123'] },
    chunking: { maxChars: 1000, maxOverlap: 200 },
    outputFormat: 'markdown',
};

const result = await extractFile('document.pdf', null, config);
```

### Rust (snake_case)
```rust
use kreuzberg::{ExtractionConfig, OcrConfig, ChunkingConfig, OutputFormat};

let config = ExtractionConfig {
    ocr: Some(OcrConfig {
        backend: "tesseract".into(),
        language: "eng".into(),
        ..Default::default()
    }),
    chunking: Some(ChunkingConfig {
        max_characters: 1000,
        overlap: 200,
        ..Default::default()
    }),
    output_format: OutputFormat::Markdown,
    ..Default::default()
};

let result = extract_file("document.pdf", None, &config).await?;
```

### Config File (TOML)
```toml
output_format = "markdown"

[ocr]
backend = "tesseract"
language = "eng"

[chunking]
max_chars = 1000
max_overlap = 200

[pdf_options]
passwords = ["secret123"]
```

```bash
# CLI: auto-discovers kreuzberg.toml in current/parent directories
kreuzberg extract doc.pdf
# or explicit:
kreuzberg extract doc.pdf --config kreuzberg.toml
kreuzberg extract doc.pdf --config-json '{"ocr":{"backend":"tesseract","language":"deu"}}'
```

## Batch Processing

### Python
```python
from kreuzberg import batch_extract_files, batch_extract_files_sync

# Async
results = await batch_extract_files(["doc1.pdf", "doc2.docx", "doc3.xlsx"])

# Sync
results = batch_extract_files_sync(["doc1.pdf", "doc2.docx"])

for result in results:
    print(f"{len(result.content)} chars extracted")
```

### Node.js
```typescript
import { batchExtractFiles } from '@kreuzberg/node';

const results = await batchExtractFiles(['doc1.pdf', 'doc2.docx']);
```

### Rust — requires `tokio-runtime` feature
```rust
use kreuzberg::{batch_extract_file, ExtractionConfig};

let config = ExtractionConfig::default();
let paths = vec!["doc1.pdf", "doc2.docx"];
let results = batch_extract_file(paths, &config).await?;
```

### CLI
```bash
kreuzberg batch *.pdf --format json
kreuzberg batch docs/*.docx --output-format markdown
```

## OCR

OCR runs automatically for images and scanned PDFs. Tesseract is the default backend (native binding, no external install required).

### Backends
- **Tesseract** (default): Built-in native binding. All Tesseract languages supported.
- **EasyOCR** (Python only): `pip install kreuzberg[easyocr]`. Pass `easyocr_kwargs={"gpu": True}`.
- **PaddleOCR** (Python only): `pip install kreuzberg[paddleocr]`. Pass `paddleocr_kwargs={"use_angle_cls": True}`.
- **Guten** (Node.js only): Built-in OCR backend via `GutenOcrBackend`.

### Language Codes
```python
config = ExtractionConfig(ocr=OcrConfig(language="eng"))       # English
config = ExtractionConfig(ocr=OcrConfig(language="eng+deu"))   # Multiple
config = ExtractionConfig(ocr=OcrConfig(language="all"))       # All installed
```

### Force OCR
```python
config = ExtractionConfig(force_ocr=True)  # OCR even if text is extractable
```

## ExtractionResult Fields

| Field | Python | Node.js | Rust | Description |
|-------|--------|---------|------|-------------|
| Text content | `result.content` | `result.content` | `result.content` | Extracted text (str/String) |
| MIME type | `result.mime_type` | `result.mimeType` | `result.mime_type` | Input document MIME type |
| Metadata | `result.metadata` | `result.metadata` | `result.metadata` | Document metadata (dict/object/HashMap) |
| Tables | `result.tables` | `result.tables` | `result.tables` | Extracted tables with cells + markdown |
| Languages | `result.detected_languages` | `result.detectedLanguages` | `result.detected_languages` | Detected languages (if enabled) |
| Chunks | `result.chunks` | `result.chunks` | `result.chunks` | Text chunks (if chunking enabled) |
| Images | `result.images` | `result.images` | `result.images` | Extracted images (if enabled) |
| Elements | `result.elements` | `result.elements` | `result.elements` | Semantic elements (if element_based format) |
| Pages | `result.pages` | `result.pages` | `result.pages` | Per-page content (if page extraction enabled) |
| Keywords | `result.keywords` | `result.keywords` | `result.keywords` | Extracted keywords (if enabled) |

## Error Handling

### Python
```python
from kreuzberg import (
    extract_file_sync, KreuzbergError, ParsingError,
    OCRError, ValidationError, MissingDependencyError,
)

try:
    result = extract_file_sync("file.pdf")
except ParsingError as e:
    print(f"Failed to parse: {e}")
except OCRError as e:
    print(f"OCR failed: {e}")
except ValidationError as e:
    print(f"Invalid input: {e}")
except MissingDependencyError as e:
    print(f"Missing dependency: {e}")
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
```

### Node.js
```typescript
import {
    extractFile, KreuzbergError, ParsingError,
    OcrError, ValidationError, MissingDependencyError,
} from '@kreuzberg/node';

try {
    const result = await extractFile('file.pdf');
} catch (e) {
    if (e instanceof ParsingError) { /* ... */ }
    else if (e instanceof OcrError) { /* ... */ }
    else if (e instanceof ValidationError) { /* ... */ }
    else if (e instanceof KreuzbergError) { /* ... */ }
}
```

### Rust
```rust
use kreuzberg::{extract_file, ExtractionConfig, KreuzbergError};

let config = ExtractionConfig::default();
match extract_file("file.pdf", None, &config).await {
    Ok(result) => println!("{}", result.content),
    Err(KreuzbergError::Parsing(msg)) => eprintln!("Parse error: {msg}"),
    Err(KreuzbergError::Ocr(msg)) => eprintln!("OCR error: {msg}"),
    Err(e) => eprintln!("Error: {e}"),
}
```

## Common Pitfalls

1. **Python ChunkingConfig fields**: Use `max_chars` and `max_overlap`, NOT `max_characters` or `overlap`.
2. **Rust extract_file signature**: Third argument is `&ExtractionConfig` (a reference), not `Option`. Use `&ExtractionConfig::default()` for defaults.
3. **Rust feature gates**: `extract_file_sync`, `batch_extract_file`, and `batch_extract_file_sync` all require `features = ["tokio-runtime"]` in Cargo.toml.
4. **Rust async context**: `extract_file` is async. Use `#[tokio::main]` or call from an async context.
5. **CLI --format vs --output-format**: `--format` controls CLI output (text/json). `--output-format` controls content format (plain/markdown/djot/html).
6. **Node.js extractFile signature**: `extractFile(path, mimeType?, config?)` — mimeType is the second arg (pass `null` to skip).
7. **Python detect_mime_type**: The function for detecting from bytes is `detect_mime_type(data)`. For paths use `detect_mime_type_from_path(path)`.
8. **Config file field names**: Use snake_case in TOML/YAML/JSON config files (e.g., `max_chars`, `max_overlap`, `pdf_options`).

## Supported Formats (Summary)

| Category | Extensions |
|----------|-----------|
| **PDF** | `.pdf` |
| **Word** | `.docx`, `.odt` |
| **Spreadsheets** | `.xlsx`, `.xlsm`, `.xlsb`, `.xls`, `.xla`, `.xlam`, `.xltm`, `.ods` |
| **Presentations** | `.pptx`, `.ppt`, `.ppsx` |
| **eBooks** | `.epub`, `.fb2` |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.bmp`, `.tiff`, `.tif`, `.jp2`, `.jpx`, `.jpm`, `.mj2`, `.jbig2`, `.jb2`, `.pnm`, `.pbm`, `.pgm`, `.ppm`, `.svg` |
| **Markup** | `.html`, `.htm`, `.xhtml`, `.xml` |
| **Data** | `.json`, `.yaml`, `.yml`, `.toml`, `.csv`, `.tsv` |
| **Text** | `.txt`, `.md`, `.markdown`, `.djot`, `.rst`, `.org`, `.rtf` |
| **Email** | `.eml`, `.msg` |
| **Archives** | `.zip`, `.tar`, `.tgz`, `.gz`, `.7z` |
| **Academic** | `.bib`, `.biblatex`, `.ris`, `.nbib`, `.enw`, `.csl`, `.tex`, `.latex`, `.typ`, `.jats`, `.ipynb`, `.docbook`, `.opml`, `.pod`, `.mdoc`, `.troff` |

See [references/supported-formats.md](references/supported-formats.md) for the complete format reference with MIME types.

## Additional Resources

Detailed reference files for specific topics:

- **[Python API Reference](references/python-api.md)** — All functions, config classes, plugin protocols, exact signatures
- **[Node.js API Reference](references/nodejs-api.md)** — All functions, TypeScript interfaces, worker pool APIs
- **[Rust API Reference](references/rust-api.md)** — All functions with feature gates, structs, Cargo.toml examples
- **[CLI Reference](references/cli-reference.md)** — All commands, flags, config precedence, exit codes
- **[Configuration Reference](references/configuration.md)** — TOML/YAML/JSON formats, auto-discovery, env vars, full schema
- **[Supported Formats](references/supported-formats.md)** — All 62+ formats with file extensions and MIME types
- **[Advanced Features](references/advanced-features.md)** — Plugins, embeddings, MCP server, API server, security limits
- **[Other Language Bindings](references/other-bindings.md)** — Go, Ruby, Java, C#, PHP, Elixir, WASM, Docker

Full documentation: https://docs.kreuzberg.dev
GitHub: https://github.com/kreuzberg-dev/kreuzberg
