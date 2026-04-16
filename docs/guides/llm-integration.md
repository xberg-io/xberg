# LLM Integration <span class="version-badge">v4.8.0</span>

Kreuzberg integrates with 146 LLM providers (including local inference engines) via [liter-llm](https://github.com/kreuzberg-dev/liter-llm) for three capabilities: VLM OCR, structured extraction, and provider-hosted embeddings.

!!! Note "Feature gate"
    Requires the `liter-llm` Cargo feature. Not included in the default feature set.

## VLM OCR

Use vision-language models as an OCR backend. The document page is rendered as an image and sent to the VLM, which returns the extracted text.

### When to Use

- Low-quality scanned documents where traditional OCR struggles
- Handwritten text recognition
- Arabic, Farsi, and other scripts with poor Tesseract/PaddleOCR support
- Complex layouts where traditional OCR fails (mixed tables, forms, diagrams)
- When you need higher accuracy and can accept higher latency and API costs

### Configuration

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
    kreuzberg extract scan.pdf --force-ocr true \
      --vlm-model openai/gpt-4o-mini
    ```

=== "TOML"

    ```toml title="kreuzberg.toml"
    force_ocr = true

    [ocr]
    backend = "vlm"

    [ocr.vlm_config]
    model = "openai/gpt-4o-mini"
    ```

=== "Environment Variables"

    ```bash title="Terminal"
    export KREUZBERG_VLM_OCR_MODEL=openai/gpt-4o-mini
    export OPENAI_API_KEY=sk-...
    ```

### Custom VLM Prompt

Override the default prompt template for VLM OCR:

```python title="Python"
from kreuzberg import ExtractionConfig, OcrConfig, LlmConfig

config = ExtractionConfig(
    force_ocr=True,
    ocr=OcrConfig(
        backend="vlm",
        vlm_config=LlmConfig(model="openai/gpt-4o-mini"),
        vlm_prompt="Extract all text from this document image. Preserve formatting.",
    ),
)
```

### Supported Providers

Any liter-llm vision-capable provider works as a VLM OCR backend:

| Provider | Example Model |
|----------|--------------|
| OpenAI | `openai/gpt-4o`, `openai/gpt-4o-mini` |
| Anthropic | `anthropic/claude-sonnet-4-20250514` |
| Google | `google/gemini-2.0-flash` |
| Groq | `groq/llama-3.2-90b-vision-preview` |
| Ollama (local) | `ollama/llama3.2-vision` |
| LM Studio (local) | `lmstudio/llava-1.5` |
| vLLM (local) | `vllm/llava-next` |

## Structured Extraction

Extract structured JSON data from documents by providing a JSON schema. The document is first extracted as text, then sent to an LLM with the schema to produce conforming output.

### Basic Usage

=== "Python"

    --8<-- "snippets/python/llm/structured_extraction.md"

=== "TypeScript"

    --8<-- "snippets/typescript/llm/structured_extraction.md"

=== "Rust"

    --8<-- "snippets/rust/llm/structured_extraction.md"

=== "CLI"

    ```bash title="Terminal"
    kreuzberg extract-structured paper.pdf \
      --schema schema.json \
      --model openai/gpt-4o-mini \
      --strict
    ```

=== "TOML"

    ```toml title="kreuzberg.toml"
    [structured_extraction]
    schema_name = "paper_metadata"
    strict = true

    [structured_extraction.schema]
    type = "object"

    [structured_extraction.schema.properties.title]
    type = "string"

    [structured_extraction.schema.properties.date]
    type = "string"

    [structured_extraction.llm]
    model = "openai/gpt-4o-mini"
    ```

### Custom Prompts (Jinja2)

Override the default extraction prompt with a Jinja2 template:

```python title="Python"
from kreuzberg import ExtractionConfig, StructuredExtractionConfig, LlmConfig

config = ExtractionConfig(
    structured_extraction=StructuredExtractionConfig(
        schema={"type": "object", "properties": {"title": {"type": "string"}}},
        llm=LlmConfig(model="openai/gpt-4o-mini"),
        prompt=(
            "Analyze this document and extract key metadata.\n\n"
            "Document:\n{{ content }}\n\n"
            "Schema: {{ schema }}"
        ),
    ),
)
```

Available template variables:

| Variable | Description |
|----------|-------------|
| `{{ content }}` | The extracted document text |
| `{{ schema }}` | The JSON schema as a formatted string |
| `{{ schema_name }}` | The schema name (default: `"extraction"`) |
| `{{ schema_description }}` | The schema description (may be empty) |

### Cross-Provider Compatibility

Structured extraction handles provider differences automatically:

- **OpenAI**: Full strict mode with `additionalProperties` enforcement
- **Anthropic/Gemini**: `additionalProperties` automatically stripped (not supported by these providers)
- **All providers**: Markdown code fence wrapping in responses is automatically handled

### Strict Mode

When `strict=True`, the LLM is instructed to produce output that exactly matches the schema. This enables OpenAI's structured output mode and adds validation on the response.

## VLM Embeddings

Use provider-hosted embedding models instead of local ONNX models. Useful when you want to match the embedding model used by your vector database or when local ONNX models are not available.

### Configuration

=== "Python"

    --8<-- "snippets/python/llm/vlm_embeddings.md"

=== "TypeScript"

    ```typescript title="TypeScript"
    import { embedSync } from '@kreuzberg/node';

    const embeddings = embedSync(['Hello world'], {
      model: {
        modelType: 'llm',
        value: 'openai/text-embedding-3-small',
      },
      normalize: true,
    });
    console.log(embeddings[0].length); // 1536
    ```

=== "Rust"

    ```rust title="Rust"
    use kreuzberg::{embed_texts, EmbeddingConfig, EmbeddingModelType, LlmConfig};

    let config = EmbeddingConfig {
        model: EmbeddingModelType::Llm {
            llm: LlmConfig {
                model: "openai/text-embedding-3-small".to_string(),
                ..Default::default()
            },
        },
        normalize: true,
        ..Default::default()
    };
    let embeddings = embed_texts(&["Hello world"], &config)?;
    ```

=== "CLI"

    ```bash title="Terminal"
    kreuzberg embed \
      --provider llm \
      --model openai/text-embedding-3-small \
      --text "Hello world"
    ```

### Available Models

| Model | Dimensions | Provider |
|-------|-----------|----------|
| `openai/text-embedding-3-small` | 1536 | OpenAI |
| `openai/text-embedding-3-large` | 3072 | OpenAI |
| `mistral/mistral-embed` | 1024 | Mistral |
| Any liter-llm embedding-capable provider | Varies | Various |

## Local LLM Support

<span class="version-badge">v4.8.0</span>

Kreuzberg supports local LLM inference engines via [liter-llm](https://github.com/kreuzberg-dev/liter-llm)'s built-in provider routing. No API key required — just point to your local server.

### Supported Local Engines

| Engine | Prefix | Default URL | Install |
|--------|--------|-------------|---------|
| [Ollama](https://ollama.com) | `ollama/` | `http://localhost:11434/v1` | `brew install ollama` |
| [LM Studio](https://lmstudio.ai) | `lmstudio/` | `http://localhost:1234/v1` | Desktop app |
| [vLLM](https://vllm.ai) | `vllm/` | `http://localhost:8000/v1` | `pip install vllm` |
| [llama.cpp](https://github.com/ggerganov/llama.cpp) | `llamacpp/` | `http://localhost:8080/v1` | Build from source |
| [LocalAI](https://localai.io) | `localai/` | `http://localhost:8080/v1` | Docker |
| [llamafile](https://github.com/Mozilla-Ocho/llamafile) | `llamafile/` | `http://localhost:8080/v1` | Single binary |

### Example: Ollama

=== "CLI"
    ```bash

    # Start Ollama and pull a model

    ollama pull llama3.2-vision

    # Use it for VLM OCR (no API key needed)
    kreuzberg extract scan.pdf --force-ocr true \
      --vlm-model ollama/llama3.2-vision

    # Use it for structured extraction
    kreuzberg extract-structured doc.pdf \
      --schema schema.json \
      --model ollama/llama3.2

    # Use it for embeddings
    kreuzberg embed --provider llm \
      --model ollama/all-minilm \
      --text "Hello world"
    ```

=== "Python"
    ```python
    from Kreuzberg import extract_file, ExtractionConfig, StructuredExtractionConfig, LlmConfig

    config = ExtractionConfig(
        structured_extraction=StructuredExtractionConfig(
            schema={"type": "object", "properties": {"title": {"type": "string"}}},
            llm=LlmConfig(model="ollama/llama3.2"),  # No api_key needed
        ),
    )
    result = await extract_file("doc.pdf", config=config)
    ```

=== "TOML Config"
    ```toml
    [structured_extraction.llm]
    model = "ollama/llama3.2"

    # No api_key needed for local providers
    ```

!!! Tip "Custom Base URL"
    If your local server runs on a non-default port, use `base_url`:
    ```python
    LlmConfig(model="ollama/llama3.2", base_url="http://localhost:11435/v1")```

## LLM Usage Tracking

Every LLM call made during extraction is tracked in the `llm_usage` field of `ExtractionResult`. Each entry records the model used, token counts, estimated cost, and why the model stopped generating.

=== "Python"

    ```python
    result = await extract_file("document.pdf", config)
    if result.get("llm_usage"):
        for usage in result["llm_usage"]:
            print(f"{usage['source']}: {usage['input_tokens']} in, {usage['output_tokens']} out, ${usage['estimated_cost']:.4f}")
    ```

=== "TypeScript"

    ```typescript
    const result = await extractFile("document.pdf", config);
    for (const usage of result.llmUsage ?? []) {
        console.log(`${usage.source}: ${usage.inputTokens} in, ${usage.outputTokens} out, $${usage.estimatedCost?.toFixed(4)}`);
    }
    ```

=== "Rust"

    ```rust
    let result = extract_file("document.pdf", &config).await?;
    if let Some(usages) = &result.llm_usage {
        for usage in usages {
            println!("{}: {} in, {} out", usage.source, usage.input_tokens.unwrap_or(0), usage.output_tokens.unwrap_or(0));
        }
    }
    ```

The `source` field indicates which pipeline stage triggered the call: `"vlm_ocr"`, `"structured_extraction"`, or `"embeddings"`.

## API Key Configuration

API keys can be set via (in order of precedence):

1. `api_key` field in `LlmConfig` — highest priority, per-request
2. Provider standard env vars (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, etc.)
3. Kreuzberg-specific env var (`KREUZBERG_LLM_API_KEY`) — used as fallback for any provider

!!! Note "Local providers skip API key lookup"
    Local inference engines (Ollama, LM Studio, vLLM, llama.cpp, LocalAI, llamafile) do not require an API key. If you use a local provider prefix (for example, `ollama/`), the API key fields are ignored.

```python title="Python"
from kreuzberg import LlmConfig

# Explicit API key
config = LlmConfig(model="openai/gpt-4o", api_key="sk-...")

# Custom base URL (e.g., Azure OpenAI, local proxy)
config = LlmConfig(
    model="openai/gpt-4o",
    base_url="https://my-proxy.example.com/v1",
)
```

## LlmConfig Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model` | `str` | *required* | Provider/model in liter-llm format (for example, `"openai/gpt-4o"`) |
| `api_key` | `str \| None` | `None` | API key (falls back to env vars) |
| `base_url` | `str \| None` | `None` | Custom endpoint URL |
| `timeout_secs` | `int \| None` | `60` | Request timeout in seconds |
| `max_retries` | `int \| None` | `3` | Maximum retry attempts |
| `temperature` | `float \| None` | `None` | Sampling temperature |
| `max_tokens` | `int \| None` | `None` | Maximum tokens to generate |

## REST API

### Structured Extraction

`POST /extract-structured` — multipart form with file + schema + model configuration.

```bash title="Terminal"
curl -X POST http://localhost:4000/extract-structured \
  -F "file=@invoice.pdf" \
  -F 'schema={"type":"object","properties":{"vendor":{"type":"string"},"total":{"type":"number"}}}' \
  -F "model=openai/gpt-4o-mini" \
  -F "strict=true"
```

## MCP Tools

When running Kreuzberg as an MCP server, LLM features are available as tools:

- `extract_structured` — extract structured data from a document using a JSON schema
- `embed_text` — extended with `model` parameter for LLM-hosted embeddings

## Related

- [OCR](ocr.md) — OCR backends including VLM OCR
- [Configuration Reference](configuration.md) — full field reference for all config types
- [Advanced Features](advanced.md) — chunking, language detection, local embeddings
- [API Server](api-server.md) — REST API endpoints
