# OCR (Optical Character Recognition)

Extract text from images and scanned PDFs. Xberg automatically determines when OCR is needed — images always require it, scanned PDFs trigger it per-page, and hybrid PDFs only OCR the pages that lack a text layer. Set `force_ocr=True` to OCR all pages regardless.

See the [OcrConfig reference](../reference/configuration.md#ocrconfig) for all configuration options.

## Backend Comparison

Eight OCR backends — pick based on platform, accuracy needs, and language coverage.

|                  | **Tesseract**        | **PaddleOCR**                       | **Candle GLM-OCR** | **Candle TrOCR** | **Candle Hunyuan-OCR** | **Candle DeepSeek-OCR** | **Candle PaddleOCR-VL** | **VLM**                  |
| ---------------- | -------------------- | ----------------------------------- | ------------------ | ---------------- | ---------------------- | ----------------------- | ----------------------- | ------------------------ |
| **Speed**        | Fast                 | Very fast                           | Moderate           | Moderate         | Moderate               | Moderate                | Moderate                | Slow (API latency)       |
| **Accuracy**     | Good                 | Excellent                           | Excellent          | Good             | Excellent              | Excellent               | Excellent               | Highest                  |
| **Languages**    | 100+                 | 80+ (11 script families)            | All                | 100+             | 20+ (CJK + Latin)      | 20+ (CJK + Latin)       | 20+ (CJK + Latin)       | All (provider-dependent) |
| **Installation** | System package       | Built-in (native) or Python package | Cargo feature      | Cargo feature    | Cargo feature          | Cargo feature           | Cargo feature           | API key only             |
| **Model size**   | ~10 MB               | Mobile ~8 MB, Server ~120 MB        | ~3 GB              | ~250 MB          | ~3.5 GB                | ~4 GB                   | ~2.5 GB                 | None (cloud-hosted)      |
| **GPU support**  | No                   | Yes                                 | Yes (Metal/CUDA)   | Yes (Metal/CUDA) | Yes (Metal/CUDA)       | Yes (Metal/CUDA)        | Yes (Metal/CUDA)        | N/A (server-side)        |
| **Platform**     | All (including Wasm) | All except Wasm                     | Native only        | Native only      | Native only            | Native only             | Native only             | All                      |
| **Cost**         | Free                 | Free                                | Free               | Free             | Free                   | Free                    | Free                    | Per-token API cost       |

**When to use which:**

- **Tesseract** — Default choice. Works everywhere, low overhead, broadest platform support.
- **PaddleOCR** — Best speed-to-accuracy ratio. Preferred for CJK languages. Mobile tier is fast; server tier maximizes accuracy with GPU.
- **Candle GLM-OCR** — Excellent accuracy with VLM-level reasoning on 0.9B-param GLM model. Pure Rust, GPU-accelerated (Metal on macOS, CUDA on Linux). Region-aware layout dispatch. First download ~3 GB.
- **Candle TrOCR** — Smaller model footprint (~250 MB) with solid accuracy across languages. Pure Rust, GPU-accelerated. Good balance of speed and quality.
- **Candle Hunyuan-OCR** — Tencent Hunyuan-OCR with comprehensive document parsing and multilingual support including CJK and Latin scripts. Pure Rust, GPU-accelerated. First download ~3.5 GB.
- **Candle DeepSeek-OCR** — Deep learning-based OCR combining SAM + CLIP + Qwen2 + DeepSeek MoE. Multilingual with strong CJK coverage. Pure Rust, GPU-accelerated. First download ~4 GB.
- **Candle PaddleOCR-VL** — SigLIP vision encoder + Ernie-4.5 text decoder. Lightweight multilingual model with CJK and Latin support. Pure Rust, GPU-accelerated. First download ~2.5 GB.
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
    xberg = { version = "5", features = ["paddle-ocr"] }
    ```

=== "Python"

    PaddleOCR is bundled via the native Rust bindings and works out of the box since 4.8.5 — no extra installation is needed. Models are downloaded automatically on first use.

!!! Tip "Tesseract marker extra"
`pip install "xberg[tesseract]"` is available as a metadata-only marker to document a dependency on the Tesseract system package. It installs no Python packages — Tesseract itself must still be installed via your OS package manager (see above).

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

=== "Wasm"

    --8<-- "snippets/wasm/ocr/ocr_extraction.md"

### Multiple Languages

Specify multiple language codes separated by `+` (Tesseract) or as a list (PaddleOCR and VLM backends):

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

=== "Wasm"

    ```typescript
    import { ExtractInputKind, enableOcr, extract, initWasm } from '@xberg-io/xberg-wasm';

    await initWasm();
    await enableOcr();

    const file = fileInput.files?.[0];
    if (file) {
      const output = await extract(
        {
          kind: ExtractInputKind.Bytes,
          bytes: new Uint8Array(await file.arrayBuffer()),
          mimeType: file.type || 'application/octet-stream',
          filename: file.name,
        },
        { ocr: { backend: 'tesseract-wasm', language: 'eng+deu' } },
      );
      const result = output.results[0];
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

### Disable OCR

When `disable_ocr` is set, image files return empty content instead of raising `MissingDependencyError`:

=== "Python"

    ```python title="disable_ocr.py"
    from xberg import ExtractInput, ExtractionConfig, extract

    config = ExtractionConfig(disable_ocr=True)
    output = await extract(ExtractInput(kind="uri", uri="scanned.png"), config=config)
    result = output.results[0]
    # result.content will be empty — OCR was skipped
    ```

=== "TypeScript"

    ```typescript title="disable_ocr.ts"
    import { ExtractInputKind, extract } from '@xberg-io/xberg';

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'scanned.png' },
      { disableOcr: true },
    );
    const result = output.results[0];
    // result.content will be empty — OCR was skipped
    ```

=== "Rust"

    ```rust title="disable_ocr.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig};

    let config = ExtractionConfig {
        disable_ocr: true,
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("scanned.png"), &config).await?;
    let result = &output.results[0];
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

### Candle GLM-OCR

=== "Native bindings (Rust, Go, TypeScript, Node.js, Java, C#, Ruby, PHP, Elixir)"

    Built in via the `candle-glm-ocr` feature flag. The GLM-OCR model downloads automatically on first use (~3 GB) and is cached at `~/.cache/huggingface/`.

    ```toml title="Cargo.toml (Rust example)"
    [dependencies]
    xberg = { version = "5", features = ["candle-glm-ocr"] }
    ```

    **GPU support:**

    - **Metal** (macOS) — Default, F32 dtype (BF16 matmul unavailable in candle 0.10)
    - **CUDA** (Linux/Windows with NVIDIA GPU) — Auto-detected
    - **CPU fallback** — Slowest, but always available

### Using Candle GLM-OCR

Candle GLM-OCR dispatches by detected layout region using PP-DocLayout-V3. Each region runs through the appropriate task prompt (ocr/table/formula/chart/caption) and outputs are merged into reading-order markdown.

=== "Python"

    ```python title="candle_glm_ocr.py"
    from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

    # Paired mode: per-region dispatch (default)
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="candle-glm-ocr",
            language="en",
            backend_options={"layout_mode": "paired"},
        ),
    )
    output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config=config)
    result = output.results[0]
    print(result.content)

    # Whole-page mode: single OCR pass over entire page
    config_whole = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="candle-glm-ocr",
            language="en",
            backend_options={"layout_mode": "whole_page"},
        ),
    )
    whole_page_output = await extract(
        ExtractInput(kind="uri", uri="document.pdf"),
        config=config_whole,
    )
    result_whole_page = whole_page_output.results[0]
    ```

=== "TypeScript"

    ```typescript title="candle-glm-ocr.ts"
    import { ExtractInputKind, extract } from '@xberg-io/xberg';

    // Paired mode: per-region dispatch (default)
    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'document.pdf' },
      {
        forceOcr: true,
        ocr: {
          backend: 'candle-glm-ocr',
          language: 'en',
          backendOptions: { layout_mode: 'paired' },
        },
      },
    );
    const result = output.results[0];
    console.log(result.content);

    // Whole-page mode
    const wholePageOutput = await extract(
      { kind: ExtractInputKind.Uri, uri: 'document.pdf' },
      {
        forceOcr: true,
        ocr: {
          backend: 'candle-glm-ocr',
          language: 'en',
          backendOptions: { layout_mode: 'whole_page' },
        },
      },
    );
    const resultWholePage = wholePageOutput.results[0];
    ```

=== "Rust"

    ```rust title="candle_glm_ocr.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig};
    use serde_json::json;

    // Paired mode: per-region dispatch (default)
    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "candle-glm-ocr".into(),
            language: "en".into(),
            backend_options: Some(json!({"layout_mode": "paired"})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
    let result = &output.results[0];
    println!("{}", result.content);

    // Whole-page mode
    let config_whole = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "candle-glm-ocr".into(),
            language: "en".into(),
            backend_options: Some(json!({"layout_mode": "whole_page"})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let whole_page_output = extract(ExtractInput::from_uri("document.pdf"), &config_whole).await?;
    let _result_whole_page = &whole_page_output.results[0];
    ```

**Backend options:**

| Option | Values | Description |
| --- | --- | --- |
| `layout_mode` | `"paired"` (default), `"whole_page"` | Paired: dispatch per-region via PP-DocLayout-V3. Whole-page: single OCR pass on entire page. |
| `task` | `"ocr"` (default), `"table"`, `"formula"`, `"chart"`, `"caption"` | Task prompt for whole-page mode only; ignored in paired mode where the region type determines the prompt. |
| `device` | `"auto"` (default), `"cpu"`, `"metal"`, `"cuda"` | Device selection. Auto detects Metal on macOS, CUDA on Linux, CPU fallback. |

### Candle Hunyuan-OCR

Tencent Hunyuan-OCR — vision-language model for comprehensive document parsing with markdown output and multilingual support.

=== "Native bindings (Rust, Go, TypeScript, Node.js, Java, C#, Ruby, PHP, Elixir)"

    Built in via the `candle-hunyuan-ocr` feature flag or the `candle-vlm-ocr` umbrella feature. The model downloads automatically on first use (~3.5 GB) and is cached at `~/.cache/huggingface/`.

    ```toml title="Cargo.toml (Rust example)"
    [dependencies]
    xberg = { version = "5", features = ["candle-hunyuan-ocr"] }
    ```

    **GPU support:**

    - **Metal** (macOS) — Default, F32 dtype
    - **CUDA** (Linux/Windows with NVIDIA GPU) — Auto-detected
    - **CPU fallback** — Slowest, but always available

### Using Candle Hunyuan-OCR

=== "Python"

    ```python title="candle_hunyuan_ocr.py"
    from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="candle-hunyuan-ocr",
            language="en",
            backend_options={"device": "auto", "model_path": "~/.cache/huggingface/"},
        ),
    )
    output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config=config)
    result = output.results[0]
    print(result.content)
    ```

=== "TypeScript"

    ```typescript title="candle-hunyuan-ocr.ts"
    import { ExtractInputKind, extract } from '@xberg-io/xberg';

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'document.pdf' },
      {
        forceOcr: true,
        ocr: {
          backend: 'candle-hunyuan-ocr',
          language: 'en',
          backendOptions: { device: 'auto', model_path: '~/.cache/huggingface/' },
        },
      },
    );
    const result = output.results[0];
    console.log(result.content);
    ```

=== "Rust"

    ```rust title="candle_hunyuan_ocr.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig};
    use serde_json::json;

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "candle-hunyuan-ocr".into(),
            language: "en".into(),
            backend_options: Some(json!({"device": "auto", "model_path": "~/.cache/huggingface/"})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
    let result = &output.results[0];
    println!("{}", result.content);
    ```

=== "CLI"

    ```bash title="Terminal"
    xberg extract document.pdf --force-ocr true --ocr-backend candle-hunyuan-ocr --ocr-backend-options '{"device":"auto","model_path":"~/.cache/huggingface/"}'
    ```

**Supported languages:** English, Chinese, Japanese, Korean, French, German, Spanish, Italian, Portuguese, Russian, Arabic, Hindi, Thai, Vietnamese, and others.

**Model source:** Download from [Hugging Face Hub](https://huggingface.co/models?search=hunyuan-ocr).

### Candle DeepSeek-OCR

DeepSeek-OCR — combination of SAM + CLIP encoder fused with Qwen2 decoder and DeepSeek V2 MoE for comprehensive multilingual document understanding. Markdown output.

=== "Native bindings (Rust, Go, TypeScript, Node.js, Java, C#, Ruby, PHP, Elixir)"

    Built in via the `candle-deepseek-ocr` feature flag or the `candle-vlm-ocr` umbrella feature. The model downloads automatically on first use (~4 GB) and is cached at `~/.cache/huggingface/`.

    ```toml title="Cargo.toml (Rust example)"
    [dependencies]
    xberg = { version = "5", features = ["candle-deepseek-ocr"] }
    ```

    **GPU support:**

    - **Metal** (macOS) — Default, F32 dtype
    - **CUDA** (Linux/Windows with NVIDIA GPU) — Auto-detected
    - **CPU fallback** — Slowest, but always available

### Using Candle DeepSeek-OCR

=== "Python"

    ```python title="candle_deepseek_ocr.py"
    from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="candle-deepseek-ocr",
            language="en",
            backend_options={"device": "auto", "model_path": "~/.cache/huggingface/"},
        ),
    )
    output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config=config)
    result = output.results[0]
    print(result.content)
    ```

=== "TypeScript"

    ```typescript title="candle-deepseek-ocr.ts"
    import { ExtractInputKind, extract } from '@xberg-io/xberg';

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'document.pdf' },
      {
        forceOcr: true,
        ocr: {
          backend: 'candle-deepseek-ocr',
          language: 'en',
          backendOptions: { device: 'auto', model_path: '~/.cache/huggingface/' },
        },
      },
    );
    const result = output.results[0];
    console.log(result.content);
    ```

=== "Rust"

    ```rust title="candle_deepseek_ocr.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig};
    use serde_json::json;

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "candle-deepseek-ocr".into(),
            language: "en".into(),
            backend_options: Some(json!({"device": "auto", "model_path": "~/.cache/huggingface/"})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
    let result = &output.results[0];
    println!("{}", result.content);
    ```

=== "CLI"

    ```bash title="Terminal"
    xberg extract document.pdf --force-ocr true --ocr-backend candle-deepseek-ocr --ocr-backend-options '{"device":"auto","model_path":"~/.cache/huggingface/"}'
    ```

**Supported languages:** English, Chinese, Japanese, Korean, French, German, Spanish, Italian, Portuguese, Russian, Arabic, Hindi, Thai, Vietnamese, and others.

**Model source:** Download from [Hugging Face Hub](https://huggingface.co/models?search=deepseek-ocr).

### Candle PaddleOCR-VL

PaddleOCR-VL 1.5 — SigLIP vision encoder + Ernie-4.5 text decoder for lightweight multilingual document understanding. Markdown output.

=== "Native bindings (Rust, Go, TypeScript, Node.js, Java, C#, Ruby, PHP, Elixir)"

    Built in via the `candle-paddleocr-vl-15` feature flag or the `candle-vlm-ocr` umbrella feature. The model downloads automatically on first use (~2.5 GB) and is cached at `~/.cache/huggingface/`.

    ```toml title="Cargo.toml (Rust example)"
    [dependencies]
    xberg = { version = "5", features = ["candle-paddleocr-vl-15"] }
    ```

    **GPU support:**

    - **Metal** (macOS) — Default, F32 dtype
    - **CUDA** (Linux/Windows with NVIDIA GPU) — Auto-detected
    - **CPU fallback** — Slowest, but always available

### Using Candle PaddleOCR-VL

=== "Python"

    ```python title="candle_paddleocr_vl.py"
    from xberg import ExtractInput, ExtractionConfig, OcrConfig, extract

    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="candle-paddleocr-vl-15",
            language="en",
            backend_options={"device": "auto", "model_path": "~/.cache/huggingface/"},
        ),
    )
    output = await extract(ExtractInput(kind="uri", uri="document.pdf"), config=config)
    result = output.results[0]
    print(result.content)
    ```

=== "TypeScript"

    ```typescript title="candle-paddleocr-vl.ts"
    import { ExtractInputKind, extract } from '@xberg-io/xberg';

    const output = await extract(
      { kind: ExtractInputKind.Uri, uri: 'document.pdf' },
      {
        forceOcr: true,
        ocr: {
          backend: 'candle-paddleocr-vl-15',
          language: 'en',
          backendOptions: { device: 'auto', model_path: '~/.cache/huggingface/' },
        },
      },
    );
    const result = output.results[0];
    console.log(result.content);
    ```

=== "Rust"

    ```rust title="candle_paddleocr_vl.rs"
    use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig};
    use serde_json::json;

    let config = ExtractionConfig {
        force_ocr: true,
        ocr: Some(OcrConfig {
            backend: "candle-paddleocr-vl-15".into(),
            language: "en".into(),
            backend_options: Some(json!({"device": "auto", "model_path": "~/.cache/huggingface/"})),
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = extract(ExtractInput::from_uri("document.pdf"), &config).await?;
    let result = &output.results[0];
    println!("{}", result.content);
    ```

=== "CLI"

    ```bash title="Terminal"
    xberg extract document.pdf --force-ocr true --ocr-backend candle-paddleocr-vl-15 --ocr-backend-options '{"device":"auto","model_path":"~/.cache/huggingface/"}'
    ```

**Supported languages:** English, Chinese, Japanese, Korean, French, German, Spanish, Italian, Portuguese, Russian, and others.

**Model source:** Download from [PaddlePaddle Hub](https://github.com/PaddlePaddle/PaddleOCR).

### Using VLM OCR

Use a vision-language model (e.g. GPT-4o, Claude) as the OCR backend — each page is rendered and sent to the VLM. Cloud providers need an API key; local engines (Ollama, etc.) use the `ollama/` prefix — see [Local LLM Support](llm-integration.md#local-llm-support).

=== "Python"

    --8<-- "snippets/python/llm/vlm_ocr.md"

=== "TypeScript"

    --8<-- "snippets/typescript/llm/vlm_ocr.md"

=== "Rust"

    ```rust title="Rust"
    use xberg::{extract, ExtractInput, ExtractionConfig, OcrConfig, LlmConfig};

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
    let output = extract(ExtractInput::from_uri("scan.pdf"), &config).await?;
    let result = &output.results[0];
    ```

=== "CLI"

    ```bash title="Terminal"
    xberg extract scan.pdf --force-ocr true --vlm-model openai/gpt-4o-mini
    ```

=== "TOML"

    ```toml title="xberg.toml"
    force_ocr = true

    [ocr]
    backend = "vlm"

    [ocr.vlm_config]
    model = "openai/gpt-4o-mini"
    ```

For more on VLM OCR, including custom prompts, supported providers, and API key configuration, see [LLM Integration](llm-integration.md#vlm-ocr).

!!! Tip "GPU Acceleration" PaddleOCR and Candle OCR backends support GPU acceleration. PaddleOCR's `model_tier="server"` gives the best accuracy with GPU.

## DPI Configuration

Higher DPI improves accuracy but increases processing time and memory.

| DPI               | Trade-off                                  |
| ----------------- | ------------------------------------------ |
| **150**           | Fastest — lower accuracy, less memory      |
| **300** (default) | Balanced — good accuracy, reasonable speed |
| **600**           | Best accuracy — slower, more memory        |

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

80+ languages across 11 script families (PP-OCRv5). Recognition models are downloaded on demand from HuggingFace:

| Family         | Languages                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------- |
| **English**    | English, numbers, punctuation                                                                |
| **Chinese**    | Simplified/Traditional Chinese, Japanese                                                     |
| **Latin**      | French, German, Spanish, Portuguese, Italian, Polish, Dutch, Turkish, Vietnamese, and so on. |
| **Korean**     | Korean (Hangul)                                                                              |
| **Slavic**     | Russian, Ukrainian, Belarusian, Bulgarian, Serbian, and so on.                               |
| **Thai**       | Thai script                                                                                  |
| **Greek**      | Greek script                                                                                 |
| **Arabic**     | Arabic, Persian, Urdu                                                                        |
| **Devanagari** | Hindi, Marathi, Sanskrit, Nepali                                                             |
| **Tamil**      | Tamil script                                                                                 |
| **Telugu**     | Telugu script                                                                                |

Models are cached locally after first download, so subsequent runs start immediately.

## CLI Usage

```bash title="Terminal"
# Basic OCR extraction
xberg extract scanned.pdf --ocr true

# Specific language
xberg extract french_doc.pdf --ocr true --ocr-language fra

# Specific backend
xberg extract chinese_doc.pdf --ocr true --ocr-backend paddle-ocr --ocr-language ch

# Force OCR on all pages
xberg extract document.pdf --force-ocr true

# VLM OCR backend
xberg extract handwritten.pdf --force-ocr true --vlm-model openai/gpt-4o-mini

# Use a config file
xberg extract scanned.pdf --config xberg.toml --ocr true
```

| Flag                      | Description                                                                        |
| ------------------------- | ---------------------------------------------------------------------------------- |
| `--ocr true`              | Enable OCR processing                                                              |
| `--ocr-language <code>`   | Language code (`eng`, `deu`, `fra`, `ch`, `ja`, `ru`, etc.)                        |
| `--ocr-backend <backend>` | Engine: `tesseract`, `paddle-ocr`, a `candle-*` backend, or `vlm`                  |
| `--force-ocr true`        | OCR all pages regardless of text layer                                             |
| `--vlm-model <model>`     | VLM model for OCR (for example, `openai/gpt-4o-mini`). Implies `--ocr-backend vlm` |

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
    - Try a different backend — PaddleOCR often outperforms Tesseract on complex layouts
    - Specify the correct language code for your document
    - Use `force_ocr=True` if a PDF's embedded text layer is low quality
    - For handwritten text or very poor scans, try the VLM backend with a vision-capable model (see [LLM Integration](llm-integration.md#vlm-ocr))

??? Question "Slow processing"

    - Reduce DPI to 150 for faster throughput
    - Use a GPU-capable backend such as PaddleOCR or Candle OCR
    - Use batch extraction to process multiple files concurrently

??? Question "Out of memory on large PDFs"

    - Reduce DPI — lower resolution uses significantly less memory
    - Process pages in smaller batches
    - Use PaddleOCR's mobile tier (`model_tier="mobile"`) for a smaller memory footprint

## Next Steps

- [LLM Integration](llm-integration.md) — VLM OCR, structured extraction, and LLM embeddings
- [Configuration](configuration.md) — all configuration options
- [Extraction Basics](extraction.md) — core extraction API and supported formats
- [Chunking](chunking.md) — split text for RAG
- [Language Detection](language-detection.md) — multilingual document analysis
- [Embeddings](embeddings.md) — semantic vectors for search
