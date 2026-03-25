# Architecture

Kreuzberg is a document extraction library with a Rust core and native bindings for Python, TypeScript, Ruby, and more. The core handles all the expensive work (PDF parsing, OCR, text processing) and exposes it through thin language-specific wrappers. Your code calls directly into compiled Rust. No subprocesses, no serialization, no IPC overhead.

---

## Design Principles

Three ideas shape how Kreuzberg is built:

1. **Rust does the heavy lifting.** Every performance-critical operation runs as native Rust code - compiled, optimized, and fast.
2. **Plugins cross language boundaries.** A Python OCR backend can register itself with the Rust core and participate in the extraction pipeline as a first-class citizen.
3. **Minimize data copying.** Data passes across FFI boundaries using zero-copy techniques wherever possible. When a Python plugin receives file bytes, it gets a buffer protocol view into Rust-owned memory, not a copy.

---

## System Layers

```mermaid
flowchart TB
    subgraph your_code ["Your Code"]
        Python["Python"]
        Node["TypeScript\nNode.js"]
        Wasm["TypeScript\nWASM"]
        Ruby["Ruby"]
    end

    subgraph bridges ["FFI Bridges"]
        PyO3["PyO3"]
        NAPI["NAPI-RS"]
        WB["wasm-bindgen"]
        Magnus["Magnus"]
    end

    subgraph engine ["Rust Core"]
        Core["kreuzberg\ncrate"]
    end

    Python --> PyO3
    Node --> NAPI
    Wasm --> WB
    Ruby --> Magnus

    PyO3 --> Core
    NAPI --> Core
    WB --> Core
    Magnus --> Core

    style Core fill:#e1f5ff,stroke:#0288d1
    style PyO3 fill:#ffe1e1,stroke:#c62828
    style NAPI fill:#ffe1e1,stroke:#c62828
    style WB fill:#fff3e0,stroke:#ef6c00
    style Magnus fill:#ffe1e1,stroke:#c62828
```

Your code sits at the top. It calls into a bridge layer that translates types between your language and Rust. The bridge forwards the call to the Rust core, which does the actual extraction, OCR, and text processing. Results come back through the same bridge.

### TypeScript: Native vs WASM

There are two TypeScript packages because server and browser environments have fundamentally different constraints:

- **`@kreuzberg/node`** (native) - compiled via NAPI-RS. Maximum performance on Node.js, Bun, and Deno. Requires a platform-specific native binary.
- **`@kreuzberg/wasm`** (WebAssembly) - compiled via wasm-bindgen. Runs in browsers, Cloudflare Workers, Vercel Edge, and any JavaScript runtime. About 60-80% of native speed, but zero native dependencies.

Rule of thumb: use native on servers, WASM in browsers and edge runtimes. See the [Installation Guide](../getting-started/installation.md#typescript) for setup.

---

## Rust Core Structure

The core crate (`crates/kreuzberg`) is organized into modules with clear responsibilities:

```mermaid
flowchart LR
    subgraph crate ["kreuzberg crate"]
        Core["core/\nOrchestration\nPipeline entry points"]
        Plugins["plugins/\nTrait definitions\nRegistries"]
        Extractors["extractors/\nMIME → handler\nmapping"]
        Extraction["extraction/\nPDF · Excel · Email\nHTML · XML · Text"]
        OCR["ocr/\nTesseract\nTable detection"]
        Text["text/\nToken reduction\nQuality scoring"]
        Types["types/\nExtractionResult\nMetadata · Chunk"]
        Error["error/\nKreuzbergError"]
    end

    Core --> Plugins
    Core --> Extractors
    Extractors --> Extraction
    Extractors --> Plugins
    Extraction --> OCR
    Extraction --> Text
    Core --> Types
    Core --> Error

    style Core fill:#bbdefb,stroke:#1565c0
    style Plugins fill:#c8e6c9,stroke:#2e7d32
    style Extraction fill:#fff9c4,stroke:#f9a825
    style Extractors fill:#ffccbc,stroke:#d84315
```

| Module | Responsibility |
|--------|---------------|
| **core/** | Main entry points (`extract_file`, `extract_bytes`), MIME detection, config loading, pipeline orchestration |
| **plugins/** | Plugin trait definitions (`DocumentExtractor`, `OcrBackend`, `PostProcessor`, `Validator`) and the registry system |
| **extractors/** | Maps MIME types to the correct extractor implementation and registers them with the plugin system |
| **extraction/** | Format-specific extraction logic - PDF via pdfium, Excel via calamine, email parsing, etc. |
| **ocr/** | OCR orchestration - Tesseract bindings, HOCR parsing, table detection |
| **text/** | Text processing utilities - token reduction, quality scoring, string manipulation |
| **types/** | Shared data structures: `ExtractionResult`, `Metadata`, `Chunk`, and friends |
| **error/** | Centralized error handling with the `KreuzbergError` enum |

---

## Why Rust?

**Speed.** Rust compiles to native machine code with LLVM optimizations. PDF parsing uses native pdfium bindings with no interpreter overhead. Text processing uses SIMD instructions to handle multiple characters per CPU cycle. Batch extraction runs on all CPU cores through Tokio's async runtime.

**Safety.** Rust's type system and ownership model catch entire categories of bugs at compile time. No null pointer exceptions, no data races, no buffer overflows, no use-after-free. If it compiles, those runtime errors can't happen.

**Real concurrency.** Unlike Python (limited by the GIL), Rust executes on all available cores simultaneously. Tokio's work-stealing scheduler distributes async tasks efficiently. File I/O is non-blocking, so threads never stall waiting on disk.

For detailed performance analysis, see [Performance](performance.md).

---

## Using Kreuzberg from Rust

The Rust core is a standalone library. You don't need Python or Node.js to use it:

```rust title="main.rs"
use kreuzberg::{extract_file_sync, ExtractionConfig};

fn main() -> kreuzberg::Result<()> {
    let config = ExtractionConfig::default();
    let result = extract_file_sync("document.pdf", None, &config)?;
    println!("Extracted: {}", result.content);
    Ok(())
}
```

This makes Kreuzberg a fit for Rust-native applications, CLI tools, high-performance API servers, and embedded systems where Python or Node.js aren't practical.

---

## What to Read Next

- [Extraction Pipeline](extraction-pipeline.md) - how files flow through the system stage by stage
- [Plugin System](plugin-system.md) - extending Kreuzberg with custom extractors, OCR backends, and processors
- [Performance](performance.md) - why Rust matters for extraction performance
- [Creating Plugins](../guides/plugins.md) - step-by-step plugin development guide
