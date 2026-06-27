# Xberg

[![Bindings](https://img.shields.io/badge/Bindings-alef%20%D7%90-007ec6)](https://github.com/xberg-io/alef)

[![Rust](https://img.shields.io/crates/v/xberg?label=Rust&color=007ec6)](https://crates.io/crates/xberg)
[![Python](https://img.shields.io/pypi/v/xberg?label=Python&color=007ec6)](https://pypi.org/project/xberg/)
[![TypeScript](https://img.shields.io/npm/v/@xberg-io/xberg?label=TypeScript&color=007ec6)](https://www.npmjs.com/package/@xberg-io/xberg)
[![WASM](https://img.shields.io/npm/v/@xberg-io/xberg-wasm?label=WASM&color=007ec6)](https://www.npmjs.com/package/@xberg-io/xberg-wasm)
[![Ruby](https://img.shields.io/gem/v/xberg?label=Ruby&color=007ec6)](https://rubygems.org/gems/xberg)
[![Java](https://img.shields.io/maven-central/v/io.xberg/xberg?label=Java&color=007ec6)](https://central.sonatype.com/artifact/io.xberg/xberg)
[![Go](https://img.shields.io/github/v/tag/xberg-io/xberg?label=Go&color=007ec6)](https://github.com/xberg-io/xberg/tree/main/packages/go)
[![C#](https://img.shields.io/nuget/v/Xberg?label=C%23&color=007ec6)](https://www.nuget.org/packages/Xberg/)

[![License: MIT](https://img.shields.io/badge/License-MIT-007ec6)](https://www.opensource.org/licenses/MIT)
[![Documentation](https://img.shields.io/badge/Docs-xberg-007ec6)](https://xberg.io/)
[![Hugging Face](https://img.shields.io/badge/Hugging%20Face-Xberg-007ec6)](https://huggingface.co/xberg-io)
[![Discord](https://img.shields.io/badge/Discord-Chat-007ec6)](https://discord.gg/xt9WY3GnKR)

High-performance document intelligence library for Rust. Extract text, metadata, transcripts, and structured information from PDFs, Office documents, images, audio/video, and 96 formats.

This is the core Rust library that powers the Python, TypeScript, and Ruby bindings.

> **Version 5.0.0-rc.17 Release**
> This is a pre-release version. We invite you to test the library and [report any issues](https://github.com/xberg-io/xberg/issues) you encounter.
>
> **Note**: The Rust crate is not currently published to crates.io for this RC. Use git dependencies or language bindings (Python, TypeScript, Ruby) instead.

## Installation

```toml
[dependencies]
xberg = "5.0.0-rc.17"
tokio = { version = "1", features = ["rt", "macros"] }
```

## PDFium Linking Options

Xberg offers flexible PDFium linking strategies for different deployment scenarios. **Note:** Language bindings (Python, TypeScript, Ruby, Java, Go, C#, PHP, Elixir) automatically bundle PDFium—no configuration needed. This section applies only to the Rust crate.

| Strategy              | Feature Flag  | Description                           | Use Case                                            |
| --------------------- | ------------- | ------------------------------------- | --------------------------------------------------- |
| **Default (Dynamic)** | None          | Links to system PDFium at runtime     | Development, system package users                   |
| **Static**            | `pdf-static`  | Statically links PDFium into binary   | Single binary distribution, no runtime dependencies |
| **Bundled**           | `pdf-bundled` | Downloads and embeds PDFium in binary | CI/CD, hermetic builds, largest binary size         |
| **System**            | `pdf-system`  | Uses system PDFium via pkg-config     | Linux distributions with PDFium package             |

**Example Cargo.toml configurations:**

```toml
# Default (dynamic linking)
[dependencies]
xberg = "5.0.0-rc.17"

# Static linking
[dependencies]
xberg = { version = "5.0.0-rc.17", features = ["pdf-static"] }

# Bundled in binary
[dependencies]
xberg = { version = "5.0.0-rc.17", features = ["pdf-bundled"] }

# System library (requires PDFium installed)
[dependencies]
xberg = { version = "5.0.0-rc.17", features = ["pdf-system"] }
```

For more details on feature flags and configuration options, see the [Xberg documentation](https://docs.xberg.io).

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
use xberg::{extract, ExtractInput, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract(ExtractInput::file("document.pdf"), &config).await?;
    println!("{}", result.content);
    Ok(())
}
```

### Async Extraction

```rust
use xberg::{extract, ExtractInput, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract(ExtractInput::file("document.pdf"), &config).await?;
    println!("{}", result.content);
    Ok(())
}
```

### Batch Processing

```rust
use xberg::{extract_batch, ExtractInput, ExtractionConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig::default();
    let inputs = vec![
        ExtractInput::file("doc1.pdf"),
        ExtractInput::file("doc2.pdf"),
        ExtractInput::file("doc3.pdf"),
    ];
    let results = extract_batch(inputs, &config).await?;

    for result in results {
        println!("{}", result.content);
    }
    Ok(())
}
```

## OCR with Table Extraction

```rust
use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig, TesseractConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
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

    let result = extract(ExtractInput::file("invoice.pdf"), &config).await?;

    for table in &result.tables {
        println!("{}", table.markdown);
    }
    Ok(())
}
```

## Password-Protected PDFs

```rust
use xberg::{extract, ExtractInput, ExtractionConfig, PdfConfig};

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let config = ExtractionConfig {
        pdf_options: Some(PdfConfig {
            passwords: Some(vec!["password1".to_string(), "password2".to_string()]),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = extract(ExtractInput::file("protected.pdf"), &config).await?;
    Ok(())
}
```

## Extract from Bytes

```rust
use xberg::{extract, ExtractInput, ExtractionConfig};
use std::fs;

#[tokio::main]
async fn main() -> xberg::Result<()> {
    let data = fs::read("document.pdf")?;
    let config = ExtractionConfig::default();
    let result = extract(ExtractInput::bytes(data, "application/pdf"), &config).await?;
    println!("{}", result.content);
    Ok(())
}
```

## Code Intelligence

Xberg integrates [tree-sitter-language-pack](https://docs.tree-sitter-language-pack.xberg.io) to parse and analyze source code files across **306 programming languages**. When you extract a source code file, Xberg automatically detects the language and produces structured analysis including functions, classes, imports, exports, symbols, diagnostics, and semantic code chunks.

Code intelligence data is available via the `metadata.format` field as a `FormatMetadata::Code` variant containing a `ProcessResult`.

```rust
use xberg::{extract, ExtractionConfig, TreeSitterConfig, TreeSitterProcessConfig};

fn main() -> xberg::Result<()> {
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

    let result = extract("app.py", None, &config)?;

    // Access code intelligence from format metadata
    if let Some(xberg::types::FormatMetadata::Code(ref code)) = result.metadata.format {
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

Requires the `tree-sitter` feature flag (included in `full`). See the [Xberg docs](https://docs.xberg.io/) for configuration details and examples in all languages.

## Features

The crate uses feature flags for optional functionality:

```toml
[dependencies]
xberg = { version = "5.0.0-rc.17", features = ["pdf", "excel", "ocr"] }
```

### Available Features

| Feature              | Description                | Binary Size |
| -------------------- | -------------------------- | ----------- |
| `pdf`                | PDF extraction (pure Rust) | +2MB        |
| `excel`              | Excel/spreadsheet parsing  | +3MB        |
| `office`             | DOCX, PPTX extraction      | +1MB        |
| `email`              | EML, MSG extraction        | +500KB      |
| `html`               | HTML to markdown           | +1MB        |
| `xml`                | XML streaming parser       | +500KB      |
| `archives`           | ZIP, TAR, 7Z extraction    | +2MB        |
| `ocr`                | OCR with Tesseract         | +5MB        |
| `language-detection` | Language detection         | +100KB      |
| `chunking`           | Text chunking              | +200KB      |
| `quality`            | Text quality processing    | +500KB      |

### Feature Bundles

```toml
xberg = { version = "5.0.0-rc.17", features = ["full"] }
xberg = { version = "5.0.0-rc.17", features = ["server"] }
xberg = { version = "5.0.0-rc.17", features = ["cli"] }
```

## PDF Support

Xberg uses **pdf_oxide** — a pure-Rust PDF library with no system dependencies.
Enable PDF extraction with the `pdf` feature:

```toml
[dependencies]
xberg = { version = "5.0", features = ["pdf"] }
```

No native libraries required. Works on all platforms including musl, Docker, and WASM.

## Documentation

**[API Documentation](https://docs.rs/xberg)** – Complete API reference with examples

**[https://docs.xberg.io](https://docs.xberg.io)** – User guide and tutorials

## License

MIT License (MIT) - see [LICENSE](../../LICENSE) for details.
