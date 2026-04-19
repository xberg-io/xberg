# Kreuzberg

[![Rust](https://img.shields.io/crates/v/kreuzberg?label=Rust)](https://crates.io/crates/kreuzberg)
[![Python](https://img.shields.io/pypi/v/kreuzberg?label=Python)](https://pypi.org/project/kreuzberg/)
[![TypeScript](https://img.shields.io/npm/v/@kreuzberg/node?label=TypeScript)](https://www.npmjs.com/package/@kreuzberg/node)
[![WASM](https://img.shields.io/npm/v/@kreuzberg/wasm?label=WASM)](https://www.npmjs.com/package/@kreuzberg/wasm)
[![Ruby](https://img.shields.io/gem/v/kreuzberg?label=Ruby)](https://rubygems.org/gems/kreuzberg)
[![Java](https://img.shields.io/maven-central/v/dev.kreuzberg/kreuzberg?label=Java)](https://central.sonatype.com/artifact/dev.kreuzberg/kreuzberg)
[![Go](https://img.shields.io/github/v/tag/kreuzberg-dev/kreuzberg?label=Go)](https://pkg.go.dev/github.com/kreuzberg-dev/kreuzberg)
[![C#](https://img.shields.io/nuget/v/Kreuzberg?label=C%23)](https://www.nuget.org/packages/Kreuzberg/)

[![License: Elastic-2.0](https://img.shields.io/badge/License-Elastic--2.0-blue.svg)](https://www.elastic.co/licensing/elastic-license)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![Hugging Face](https://img.shields.io/badge/%F0%9F%A4%97%20Hugging%20Face-Models-yellow)](https://huggingface.co/Kreuzberg)
[![Discord](https://img.shields.io/badge/Discord-Join%20our%20community-7289da)](https://discord.gg/xt9WY3GnKR)

High-performance document intelligence library for Rust. Extract text, metadata, and structured information from PDFs, Office documents, images, and 75 formats.

This is the core Rust library that powers the Python, TypeScript, and Ruby bindings.

> **🚀 Version 4.9.1 Release**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/kreuzberg-dev/kreuzberg/issues) you encounter.
>
> **Note**: The Rust crate is not currently published to crates.io for this RC. Use git dependencies or language bindings (Python, TypeScript, Ruby) instead.

## Installation

```toml
[dependencies]
kreuzberg = "4.0"
tokio = { version = "1", features = ["rt", "macros"] }
```

## PDFium Linking Options

Kreuzberg offers flexible PDFium linking strategies for different deployment scenarios. **Note:** Language bindings (Python, TypeScript, Ruby, Java, Go, C#, PHP, Elixir) automatically bundle PDFium—no configuration needed. This section applies only to the Rust crate.

| Strategy | Feature Flag | Description | Use Case |
|----------|-------------|-------------|----------|
| **Default (Dynamic)** | None | Links to system PDFium at runtime | Development, system package users |
| **Static** | `pdf-static` | Statically links PDFium into binary | Single binary distribution, no runtime dependencies |
| **Bundled** | `pdf-bundled` | Downloads and embeds PDFium in binary | CI/CD, hermetic builds, largest binary size |
| **System** | `pdf-system` | Uses system PDFium via pkg-config | Linux distributions with PDFium package |

**Example Cargo.toml configurations:**

```toml
# Default (dynamic linking)
[dependencies]
kreuzberg = "4.0"

# Static linking
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf-static"] }

# Bundled in binary
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf-bundled"] }

# System library (requires PDFium installed)
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf-system"] }
```

For more details on feature flags and configuration options, see the [Features documentation](https://docs.kreuzberg.dev/rust/features/).

## System Requirements

### ONNX Runtime (for embeddings)

If using embeddings functionality, ONNX Runtime must be installed:

```bash
# macOS
brew install onnxruntime

# Ubuntu/Debian
sudo apt install libonnxruntime libonnxruntime-dev

# Windows (MSVC)
scoop install onnxruntime
# OR download from https://github.com/microsoft/onnxruntime/releases
```

Without ONNX Runtime, embeddings will raise `MissingDependencyError` with installation instructions.

## Quick Start

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("{}", result.content);
    Ok(())
}
```

### Async Extraction

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

### Batch Processing

```rust
use kreuzberg::{batch_extract_file, ExtractionConfig};

#[tokio::main]
async fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    let results = batch_extract_file(&files, None, &config).await?;

    for result in results {
        println!("{}", result.content);
    }
    Ok(())
}
```

## OCR with Table Extraction

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, OcrConfig, TesseractConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: "eng".to_string(),
            tesseract_config: Some(TesseractConfig {
                enable_table_detection: true,
                ..Default::default()
            }),
        }),
        ..Default::default()
    };

    let result = extract_file_sync("invoice.pdf", None, &config)?;

    for table in &result.tables {
        println!("{}", table.markdown);
    }
    Ok(())
}
```

## Password-Protected PDFs

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, PdfConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("protected.pdf", None, &config)?;
    Ok(())
}
```

## Extract from Bytes

```rust
use kreuzberg::{extract_bytes_sync, ExtractionConfig};
use std::fs;

fn main() -> kreuzberg::Result<()> {
    let data = fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract_bytes_sync(&data, "application/pdf", &config)?;
    println!("{}", result.content);
    Ok(())
}
```

## Code Intelligence

Kreuzberg integrates [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.kreuzberg.dev) to parse and analyze source code files across **248 programming languages**. When you extract a source code file, Kreuzberg automatically detects the language and produces structured analysis including functions, classes, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available via the `metadata.format` field as a `FormatMetadata::Code` variant containing a `ProcessResult`.

```rust
use kreuzberg::{extract_file_sync, ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig {
        tree_sitter: Some(TreeSitterConfig {
            process: TreeSitterProcessConfig {
                structure: true,
                imports: true,
                exports: true,
                comments: true,
                docstrings: true,
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract_file_sync("app.py", None, &config)?;

    // Access code intelligence from format metadata
    if let Some(kreuzberg::types::FormatMetadata::Code(ref code)) = result.metadata.format {
        println!("Language: {}", code.language);
        println!("Functions/classes: {}", code.structure.len());
        println!("Imports: {}", code.imports.len());

        for item in &code.structure {
            println!("  {:?}: {:?} at line {}", item.kind, item.name, item.span.start_line);
        }

        for chunk in &code.chunks {
            println!("Chunk ({} bytes): {}...", chunk.content.len(), &chunk.content[..50.min(chunk.content.len())]);
        }
    }

    Ok(())
}
```

Requires the `tree-sitter` feature flag (included in `full`). See the [Code Intelligence Guide](https://docs.kreuzberg.dev/guides/code-intelligence/) for configuration details and examples in all languages.

## Features

The crate uses feature flags for optional functionality:

```toml
[dependencies]
kreuzberg = { version = "4.0", features = ["pdf", "excel", "ocr"] }
```

### Available Features

| Feature | Description | Binary Size |
|---------|-------------|-------------|
| `pdf` | PDF extraction via pdfium | +25MB |
| `excel` | Excel/spreadsheet parsing | +3MB |
| `office` | DOCX, PPTX extraction | +1MB |
| `email` | EML, MSG extraction | +500KB |
| `html` | HTML to markdown | +1MB |
| `xml` | XML streaming parser | +500KB |
| `archives` | ZIP, TAR, 7Z extraction | +2MB |
| `ocr` | OCR with Tesseract | +5MB |
| `language-detection` | Language detection | +100KB |
| `chunking` | Text chunking | +200KB |
| `quality` | Text quality processing | +500KB |

### Feature Bundles

```toml
kreuzberg = { version = "4.0", features = ["full"] }
kreuzberg = { version = "4.0", features = ["server"] }
kreuzberg = { version = "4.0", features = ["cli"] }
```

## PDF Support and Linking Options

Kreuzberg supports three PDFium linking strategies. **Default is `bundled-pdfium`** (best developer experience).

| Strategy | Feature | Use Case | Binary Size | Runtime Deps |
|----------|---------|----------|-------------|--------------|
| **Bundled (default)** | `bundled-pdfium` | Development, production | +8-15MB | None |
| **Static** | `static-pdfium` | Docker, musl, standalone binaries | +200MB | None |
| **System** | `system-pdfium` | Package managers, distros | +2MB | libpdfium.so |

### Quick Start

```toml
# Default - bundled PDFium (recommended)
[dependencies]
kreuzberg = "4.0"

# Static linking (Docker, musl)
[dependencies]
kreuzberg = { version = "4.0", features = ["static-pdfium"] }

# System PDFium (package managers)
[dependencies]
kreuzberg = { version = "4.0", features = ["system-pdfium"] }
```

For detailed information, see the PDFium Linking Guide in the project documentation.

**Note:** Language bindings (Python, TypeScript, Ruby, Java, Go) automatically bundle PDFium. No configuration needed.

## Documentation

**[API Documentation](https://docs.rs/kreuzberg)** – Complete API reference with examples

**[https://docs.kreuzberg.dev](https://docs.kreuzberg.dev)** – User guide and tutorials

## License

Elastic License 2.0 (ELv2) - see [LICENSE](../../LICENSE) for details.
