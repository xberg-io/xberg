# OCR (Optical Character Recognition)

Extract text from images and scanned PDFs. Kreuzberg automatically determines when OCR is needed — images always require it, scanned PDFs trigger it per-page, and hybrid PDFs only OCR the pages that lack a text layer. Set `force_ocr=True` to OCR all pages regardless.

## Backend Comparison

Kreuzberg supports four OCR backends. Pick based on your platform, accuracy needs, and language coverage.

| | **Tesseract** | **PaddleOCR** | **EasyOCR** | **VLM** |
|---|---|---|---|---|
| **Speed** | Fast | Very fast | Moderate | Slow (API latency) |
| **Accuracy** | Good | Excellent | Excellent | Highest |
| **Languages** | 100+ | 80+ (11 script families) | 80+ | All (provider-dependent) |
| **Installation** | System package | Built-in (native) or Python package | Python package only | API key only |
| **Model size** | ~10 MB | Mobile ~8 MB, Server ~120 MB | ~100 MB | None (cloud-hosted) |
| **GPU support** | No | Yes | Yes | N/A (server-side) |
| **Platform** | All (including WASM) | All except WASM | Python only | All |
| **Cost** | Free | Free | Free | Per-token API cost |

**When to use which:**

- **Tesseract** — Default choice. Works everywhere, low overhead, broadest platform support.
- **PaddleOCR** — Best speed-to-accuracy ratio. Preferred for CJK languages. Mobile tier is fast; server tier maximizes accuracy with GPU.
- **EasyOCR** — Highest accuracy with deep learning models. Python-only, heavier dependency.
- **VLM** — Best for handwritten text, poor scans, Arabic/Farsi, and complex layouts. Requires an API key and incurs per-token costs. See [LLM Integration](llm-integration.md) for full details.

## Installation

### Tesseract

=== "macOS"

    ```bash title="Terminal"
    brew install tesseract
    ```

=== "Ubuntu / Debian"

    ```bash title="Terminal"
    sudo apt-get install tesseract-ocr
    ```

=== "RHEL / Fedora"

    ```bash title="Terminal"
    sudo dnf install tesseract
    ```

=== "Windows"

    Download from [GitHub releases](https://github.com/UB-Mannheim/tesseract/wiki).

**Additional language packs:**

```bash title="Terminal"
# macOS — all languages
brew install tesseract-lang

# Ubuntu/Debian — individual languages
sudo apt-get install tesseract-ocr-deu  # German
sudo apt-get install tesseract-ocr-fra  # French

# Verify installed languages
tesseract --list-langs
```

### PaddleOCR

=== "Native bindings (Rust, Go, TypeScript, Java, C#, Ruby, PHP, Elixir)"

    Built in via the `paddle-ocr` feature flag. Models download automatically on first use — no extra installation needed.

    ```toml title="Cargo.toml (Rust example)"
    [dependencies]
    kreuzberg = { version = "4.0", features = ["paddle-ocr"] }
    ```

=== "Python"

    PaddleOCR is bundled via the native Rust bindings and works out of the box since 4.8.5 — no extra installation is needed. Models are downloaded automatically on first use.

### EasyOCR (Python only)

```bash title="Terminal"
pip install "kreuzberg[easyocr]"
```

!!! Info "Python 3.14"
    EasyOCR 1.7.3+ and PyTorch 2.9.1+ support Python 3.14. Install `kreuzberg[easyocr]` on any supported Python version (3.10–3.14).

!!! Tip "Tesseract marker extra"
    `pip install "kreuzberg[tesseract]"` is available as a metadata-only marker to document a dependency on the Tesseract system package. It installs no Python packages — Tesseract itself must still be installed via your OS package manager (see above).

## Configuration

### Basic OCR

=== "Python"

    --8<-- "snippets/python/ocr/ocr_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_extraction.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_extraction.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_extraction.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_extraction.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_extraction.md"

=== "WASM"

    --8<-- "snippets/wasm/ocr/ocr_extraction.md"

### Multiple Languages

Specify multiple language codes separated by `+` (Tesseract) or as a list (EasyOCR/PaddleOCR):

=== "Python"

    --8<-- "snippets/python/ocr/ocr_multi_language.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_multi_language.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_multi_language.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_multi_language.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_multi_language.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_multi_language.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_multi_language.md"

=== "WASM"

    ```typescript
    import { enableOcr, extractFromFile, initWasm } from '@kreuzberg/wasm';

    await initWasm();
    await enableOcr();

    const file = fileInput.files?.[0];
    if (file) {
      const result = await extractFromFile(file, file.type, {
        ocr: { backend: 'tesseract-wasm', language: 'eng+deu' },
      });
    }
    ```

### Force OCR

Process PDFs with OCR even when they have a text layer:

=== "Python"

    --8<-- "snippets/python/ocr/ocr_force_all_pages.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_force_all_pages.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_force_all_pages.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_force_all_pages.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_force_all_pages.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_force_all_pages.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_force_all_pages.md"

### Using EasyOCR

=== "Python"

    --8<-- "snippets/python/ocr/ocr_easyocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_easyocr.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_easyocr.md"

### Disable OCR

!!! Info "Added in v4.7.0"

Skip OCR entirely, even for image files that would normally require it. When `disable_ocr` is set, image files return empty content instead of raising a `MissingDependencyError`:

=== "Python"

    ```python title="disable_ocr.py"
    from kreuzberg import ExtractionConfig, extract_file_sync

    config = ExtractionConfig(disable_ocr=True)
    result = extract_file_sync("scanned.png", config=config)
    # result.content will be empty — OCR was skipped
    ```

=== "TypeScript"

    ```typescript title="disable_ocr.ts"
    import { extractFileSync } from '@kreuzberg/node';

    const result = extractFileSync('scanned.png', {
      disableOcr: true,
    });
    // result.content will be empty — OCR was skipped
    ```

=== "Rust"

    ```rust title="disable_ocr.rs"
    use kreuzberg::{ExtractionConfig, extract_file};

    let config = ExtractionConfig {
        disable_ocr: true,
        ..Default::default()
    };
    let result = extract_file("scanned.png", &config).await?;
    // result.content will be empty — OCR was skipped
    ```

### Using PaddleOCR

=== "Python"

    --8<-- "snippets/python/ocr/ocr_paddleocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_paddleocr.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_paddleocr.md"

=== "Go"

    --8<-- "snippets/go/ocr/ocr_paddleocr.md"

=== "Java"

    --8<-- "snippets/java/ocr/ocr_paddleocr.md"

=== "Ruby"

    --8<-- "snippets/ruby/ocr/ocr_paddleocr.md"

=== "R"

    --8<-- "snippets/r/ocr/ocr_paddleocr.md"

### Using VLM OCR <span class="version-badge">v4.8.0</span>

Use a vision-language model (for example, GPT-4o, Claude) as the OCR backend. Each page is rendered as an image and sent to the VLM for text extraction. Cloud providers require an API key; local engines like Ollama do not — just start the server and use the `ollama/` prefix (for example, `ollama/llama3.2-vision`). See [Local LLM Support](llm-integration.md#local-llm-support) for setup details.

=== "Python"

    --8<-- "snippets/python/llm/vlm_ocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/llm/vlm_ocr.md"

=== "Rust"

    ```rust title="Rust"
    use kreuzberg::{extract_file, ExtractionConfig, OcrConfig, LlmConfig};

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "vlm".to_string(),
            vlm_config: Some(LlmConfig {
                model: "openai/gpt-4o-mini".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_file("scan.pdf", None, &config).await?;
    ```

=== "CLI"

    ```bash title="Terminal"
    kreuzberg extract scan.pdf --force-ocr true --vlm-model openai/gpt-4o-mini
    ```

=== "TOML"

    ```toml title="kreuzberg.toml"
    force_ocr = true

    [ocr]
    backend = "vlm"

    [ocr.vlm_config]
    model = "openai/gpt-4o-mini"
    ```

For more on VLM OCR, including custom prompts, supported providers, and API key configuration, see [LLM Integration](llm-integration.md#vlm-ocr).

!!! Tip "GPU Acceleration"
    EasyOCR and PaddleOCR support GPU acceleration. Set `use_gpu=True` in your OCR config. PaddleOCR's `model_tier="server"` gives the best accuracy with GPU.

## DPI Configuration

Image resolution affects both accuracy and speed. Higher DPI improves accuracy but increases processing time and memory usage.

| DPI | Trade-off |
|-----|-----------|
| **150** | Fastest — lower accuracy, less memory |
| **300** (default) | Balanced — good accuracy, reasonable speed |
| **600** | Best accuracy — slower, more memory |

=== "Python"

    --8<-- "snippets/python/config/ocr_dpi_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/ocr/ocr_dpi_config.md"

=== "Rust"

    --8<-- "snippets/rust/ocr/ocr_dpi_config.md"

=== "Go"

    --8<-- "snippets/go/config/ocr_dpi_config.md"

=== "Java"

    --8<-- "snippets/java/config/ocr_dpi_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/ocr_dpi_config.md"

=== "R"

    --8<-- "snippets/r/config/ocr_dpi_config.md"

## PaddleOCR Script Families

PaddleOCR supports 80+ languages across 11 script families (PP-OCRv5). Recognition models are downloaded on demand from HuggingFace:

| Family | Languages |
|--------|-----------|
| **English** | English, numbers, punctuation |
| **Chinese** | Simplified/Traditional Chinese, Japanese |
| **Latin** | French, German, Spanish, Portuguese, Italian, Polish, Dutch, Turkish, Vietnamese, and so on. |
| **Korean** | Korean (Hangul) |
| **Slavic** | Russian, Ukrainian, Belarusian, Bulgarian, Serbian, and so on. |
| **Thai** | Thai script |
| **Greek** | Greek script |
| **Arabic** | Arabic, Persian, Urdu |
| **Devanagari** | Hindi, Marathi, Sanskrit, Nepali |
| **Tamil** | Tamil script |
| **Telugu** | Telugu script |

Models are cached locally after first download, so subsequent runs start immediately.

## CLI Usage

```bash title="Terminal"
# Basic OCR extraction
kreuzberg extract scanned.pdf --ocr true

# Specific language
kreuzberg extract french_doc.pdf --ocr true --ocr-language fra

# Specific backend
kreuzberg extract chinese_doc.pdf --ocr true --ocr-backend paddle-ocr --ocr-language ch

# Force OCR on all pages
kreuzberg extract document.pdf --force-ocr true

# VLM OCR backend
kreuzberg extract handwritten.pdf --force-ocr true --vlm-model openai/gpt-4o-mini

# Use a config file
kreuzberg extract scanned.pdf --config kreuzberg.toml --ocr true
```

| Flag | Description |
|------|-------------|
| `--ocr true` | Enable OCR processing |
| `--ocr-language <code>` | Language code (`eng`, `deu`, `fra`, `ch`, `ja`, `ru`, etc.) |
| `--ocr-backend <backend>` | Engine: `tesseract`, `paddle-ocr`, `easyocr`, or `vlm` |
| `--force-ocr true` | OCR all pages regardless of text layer |
| `--vlm-model <model>` | VLM model for OCR (for example, `openai/gpt-4o-mini`). Implies `--ocr-backend vlm` |

## Troubleshooting

??? Question "Tesseract not found"

    Install Tesseract and verify it's on your PATH:

    ```bash title="Terminal"
    # macOS
    brew install tesseract

    # Ubuntu/Debian
    sudo apt-get install tesseract-ocr

    # Verify
    tesseract --version
    ```

??? Question "Language not found"

    Install the language data pack:

    ```bash title="Terminal"
    # macOS — all languages
    brew install tesseract-lang

    # Ubuntu/Debian — individual language
    sudo apt-get install tesseract-ocr-deu

    # Verify
    tesseract --list-langs
    ```

??? Question "Poor accuracy"

    - Increase DPI to 600 for better quality
    - Try a different backend — PaddleOCR and EasyOCR often outperform Tesseract on complex layouts
    - Specify the correct language code for your document
    - Use `force_ocr=True` if a PDF's embedded text layer is low quality
    - For handwritten text or very poor scans, try the VLM backend with a vision-capable model (see [LLM Integration](llm-integration.md#vlm-ocr))

??? Question "Slow processing"

    - Reduce DPI to 150 for faster throughput
    - Enable GPU acceleration with EasyOCR or PaddleOCR (`use_gpu=True`)
    - Use batch extraction to process multiple files concurrently

??? Question "Out of memory on large PDFs"

    - Reduce DPI — lower resolution uses significantly less memory
    - Process pages in smaller batches
    - Use PaddleOCR's mobile tier (`model_tier="mobile"`) for a smaller memory footprint

## Next Steps

- [LLM Integration](llm-integration.md) — VLM OCR, structured extraction, and LLM embeddings
- [Configuration](configuration.md) — all configuration options
- [Extraction Basics](extraction.md) — core extraction API and supported formats
- [Advanced Features](advanced.md) — chunking, language detection, embeddings
