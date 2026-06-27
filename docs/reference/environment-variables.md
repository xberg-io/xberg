# Environment Variables Reference

Configuration precedence in Xberg follows this order (highest to lowest):

1. **Environment Variables** - Highest priority, overrides all other sources
2. **Configuration Files** - TOML, YAML, or JSON config files
3. **Defaults** - Built-in sensible defaults

This document covers all XBERG\_\* environment variables for version 4.3.8.

## When to Use Environment Variables

Environment variables are ideal for:

- **Container/Cloud Deployments**: Docker, serverless, and orchestrated environments where config files are impractical
- **CI/CD Pipelines**: Override settings per environment (dev, staging, production)
- **Simple Overrides**: Changing one or two settings without managing a config file
- **Secrets Management**: Using secret management systems that inject values as env vars

For complex configurations with many settings, configuration files are recommended:

```toml title="Example Configuration File"
# xberg.toml is cleaner for multiple settings
[ocr]
language = "eng"
backend = "tesseract"

[chunking]
max_chars = 2000
max_overlap = 300
```

## API Server Configuration

These variables control the Xberg server's network behavior and request handling.

### XBERG_HOST

**Type**: `String`
**Default**: `127.0.0.1`
**Valid Values**: Any IPv4 or IPv6 address, or hostname

The server bind address. Use `0.0.0.0` to listen on all interfaces.

```bash title="Server Bind Address Examples"
# Listen only on localhost (default)
export XBERG_HOST=127.0.0.1

# Listen on all interfaces (Docker, cloud deployments)
export XBERG_HOST=0.0.0.0

# Listen on specific interface
export XBERG_HOST=192.168.1.100
```

### XBERG_PORT

**Type**: `u16` (1-65535)
**Default**: `8000`

The server port number.

```bash title="Server Port Examples"
export XBERG_PORT=3000
export XBERG_PORT=8080
```

**Error**: Port must be a valid u16 number:

```text
XBERG_PORT must be a valid u16 number, got 'invalid': invalid digit found in string
```

### XBERG_CORS_ORIGINS

**Type**: `String` (comma-separated list)
**Default**: Empty (allows all origins)

Whitelist of allowed CORS origins. When empty, the server accepts requests from any origin.

```bash title="CORS Origins Configuration"
# Allow all origins (default)
# unset XBERG_CORS_ORIGINS

# Allow specific origins
export XBERG_CORS_ORIGINS="https://api.example.com, https://app.example.com"

# Single origin
export XBERG_CORS_ORIGINS="https://trusted.com"
```

**Security Warning**: Be explicit with CORS origins in production. Allowing all origins (`*`) means any website can call your API on behalf of users. In Xberg, an empty list allows all origins - be intentional about this choice.

```bash title="CORS Security Best Practices"
# Production: Restrict to known origins
export XBERG_CORS_ORIGINS="https://app.mycompany.com, https://admin.mycompany.com"

# Development: Can use wildcard, but understand the security implications
# Don't use wildcard in production unless absolutely necessary
```

### XBERG_MAX_REQUEST_BODY_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum size of HTTP request bodies. Prevents oversized requests from consuming server resources.

```bash title="Max Request Body Size Examples"
# 50 MB
export XBERG_MAX_REQUEST_BODY_BYTES=52428800

# 200 MB
export XBERG_MAX_REQUEST_BODY_BYTES=209715200

# 500 MB
export XBERG_MAX_REQUEST_BODY_BYTES=524288000
```

**Note**: Both `XBERG_MAX_REQUEST_BODY_BYTES` and `XBERG_MAX_MULTIPART_FIELD_BYTES` control upload limits. Adjust both for consistent behavior.

### XBERG_MAX_MULTIPART_FIELD_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum size of individual multipart form fields. Controls the size of file uploads in multipart requests.

```bash title="Max Multipart Field Size Examples"
# 100 MB (default)
export XBERG_MAX_MULTIPART_FIELD_BYTES=104857600

# 500 MB for large document processing
export XBERG_MAX_MULTIPART_FIELD_BYTES=524288000

# 1 GB for extreme cases
export XBERG_MAX_MULTIPART_FIELD_BYTES=1073741824
```

## Extraction Configuration

These variables control document extraction behavior, including OCR, text chunking, and caching.

### XBERG_OCR_LANGUAGE

**Type**: `String` (ISO 639-1 or 639-3 language code)
**Default**: `eng` (English)

OCR language for scanned documents. Must be a valid language code recognized by the OCR backend.

```bash title="OCR Language Configuration"
# English (default)
export XBERG_OCR_LANGUAGE=eng

# German
export XBERG_OCR_LANGUAGE=deu

# French
export XBERG_OCR_LANGUAGE=fra

# Spanish
export XBERG_OCR_LANGUAGE=spa

# Chinese (Simplified)
export XBERG_OCR_LANGUAGE=chi_sim

# Japanese
export XBERG_OCR_LANGUAGE=jpn
```

**Supported Codes**: Language codes are backend-agnostic and automatically mapped to the appropriate format for each backend:

- **Tesseract codes** (ISO 639-3): `eng`, `deu`, `fra`, `spa`, `ita`, `por`, `rus`, `chi_sim`, `chi_tra`, `jpn`, `kor`
- **PaddleOCR codes**: `en`, `ch`, `french`, `german`, `korean`, `thai`, `greek`, `cyrillic`, `latin`, `arabic`, `devanagari`, `tamil`, `telugu`
- **ISO 639-1 codes**: `en`, `de`, `fr`, `es`, `ja`, `ko`, `zh`, `ru`, `ar`, `th`, `el`

All code formats are accepted regardless of backend — Xberg automatically maps between them.

### XBERG_OCR_BACKEND

**Type**: `String`
**Default**: `tesseract`
**Valid Values**: `tesseract`, `easyocr`, `paddleocr`

OCR engine to use for text extraction from images and scanned documents.

```bash title="OCR Backend Selection"
# Tesseract (open source, good for English)
export XBERG_OCR_BACKEND=tesseract

# EasyOCR (better multilingual support, slower)
export XBERG_OCR_BACKEND=easyocr

# PaddleOCR (fast, good accuracy across languages)
export XBERG_OCR_BACKEND=paddleocr
```

**Performance Notes**:

- **tesseract**: Fastest, best for English and Latin scripts
- **easyocr**: Slower, excellent multilingual support
- **paddleocr**: Fast with good accuracy for many languages

### XBERG_CHUNKING_MAX_CHARS

**Type**: `usize` (positive integer)
**Default**: `1000` (characters)

Maximum number of characters per text chunk. Smaller chunks are useful for LLM context windows.

```bash title="Chunk Size Configuration"
# Small chunks for token-constrained LLMs
export XBERG_CHUNKING_MAX_CHARS=512

# Default: balanced for most use cases
export XBERG_CHUNKING_MAX_CHARS=1000

# Larger chunks for fewer splits
export XBERG_CHUNKING_MAX_CHARS=2000

# Very large chunks for comprehensive context
export XBERG_CHUNKING_MAX_CHARS=4000
```

**Validation**: Must be greater than 0. Must be greater than `XBERG_CHUNKING_MAX_OVERLAP`.

### XBERG_CHUNKING_MAX_OVERLAP

**Type**: `usize` (non-negative integer)
**Default**: `200` (characters)

Character overlap between consecutive chunks. Maintains context across chunk boundaries.

```bash title="Chunk Overlap Configuration"
# No overlap (creates discontinuities)
export XBERG_CHUNKING_MAX_OVERLAP=0

# Default: 20% overlap with 1000-char chunks
export XBERG_CHUNKING_MAX_OVERLAP=200

# More overlap: 30% for better context continuity
export XBERG_CHUNKING_MAX_OVERLAP=300

# High overlap for sensitive documents
export XBERG_CHUNKING_MAX_OVERLAP=500
```

**Validation**: Must be less than `XBERG_CHUNKING_MAX_CHARS`.

**Example Error**:

```text
Chunking overlap (500) cannot be greater than or equal to max_chars (1000)
```

### XBERG_CACHE_ENABLED

**Type**: `Boolean` (`true` or `false`, case-insensitive)
**Default**: `true`

Enable or disable extraction result caching. Cache stores results to avoid reprocessing identical documents.

```bash title="Cache Enable/Disable"
# Enable cache (default, recommended for production)
export XBERG_CACHE_ENABLED=true

# Disable cache (development, testing, or when cache is problematic)
export XBERG_CACHE_ENABLED=false

# Case insensitive
export XBERG_CACHE_ENABLED=TRUE
export XBERG_CACHE_ENABLED=False
```

### XBERG_OUTPUT_FORMAT

**Type**: `String`
**Default**: `plain`
**Valid Values**: `plain`, `markdown`, `djot`, `html`

Controls the text content format of extraction results. Determines how extracted text is formatted in the result output.

```bash title="Output Format Options"
# Plain text content only (default)
export XBERG_OUTPUT_FORMAT=plain

# Markdown formatted output
export XBERG_OUTPUT_FORMAT=markdown

# Djot markup format
export XBERG_OUTPUT_FORMAT=djot

# HTML formatted output
export XBERG_OUTPUT_FORMAT=html
```

**Use Cases**:

| Format     | Use Case                                                        |
| ---------- | --------------------------------------------------------------- |
| `plain`    | Raw extracted text without formatting                           |
| `markdown` | Structured text with headings, lists, emphasis (RAG, LLM input) |
| `djot`     | Lightweight markup, alternative to Markdown                     |
| `html`     | Rich formatted output for web display                           |

**Example:**

```bash title="Extract with markdown formatting"
export XBERG_OUTPUT_FORMAT=markdown
xberg
```

### XBERG_TOKEN_REDUCTION_MODE

**Type**: `String`
**Default**: `off`
**Valid Values**: `off`, `light`, `moderate`, `aggressive`, `maximum`

Token reduction aggressiveness for compressing extracted text while preserving meaning. Useful when working with token-limited LLMs.

```bash title="Token Reduction Mode Options"
# No reduction (keep all text as-is)
export XBERG_TOKEN_REDUCTION_MODE=off

# Light reduction: Remove common stopwords, minimal impact
export XBERG_TOKEN_REDUCTION_MODE=light

# Moderate reduction: Balance between compression and meaning preservation
export XBERG_TOKEN_REDUCTION_MODE=moderate

# Aggressive reduction: Significant compression, some detail loss
export XBERG_TOKEN_REDUCTION_MODE=aggressive

# Maximum reduction: Extreme compression for token-constrained scenarios
export XBERG_TOKEN_REDUCTION_MODE=maximum
```

**Impact on Tokens**:

| Mode         | Typical Reduction | Use Case                                    |
| ------------ | ----------------- | ------------------------------------------- |
| `off`        | 0%                | Full preservation, no compression           |
| `light`      | 10-15%            | Minimal impact, clean up obvious redundancy |
| `moderate`   | 25-35%            | Balanced approach for most scenarios        |
| `aggressive` | 40-50%            | Significant compression, still readable     |
| `maximum`    | 50-70%            | Extreme compression, lose some detail       |

## Runtime Configuration

Control cache location, debug output, and runtime behavior.

### XBERG_CACHE_DIR

**Type**: `String` (file system path)
**Default**: Platform-specific global cache directory

Override the default cache directory for storing extraction cache, models, and intermediate files. When unset, Xberg uses a platform-appropriate global cache:

- **Linux**: `~/.cache/xberg/` (or `$XDG_CACHE_HOME/xberg/`)
- **macOS**: `~/Library/Caches/xberg/`
- **Windows**: `%LOCALAPPDATA%/xberg/`

If the platform cache directory cannot be determined, Xberg falls back to `~/.cache/xberg/`, then `.xberg/` in the current working directory as a last resort.

```bash title="Cache Directory Configuration"
# Default: uses platform-specific global cache (recommended)
# unset XBERG_CACHE_DIR

# Store cache in specific location
export XBERG_CACHE_DIR=/var/cache/xberg

# Docker: Use volume mount
export XBERG_CACHE_DIR=/data/xberg-cache

# Development: Quick local cleanup
export XBERG_CACHE_DIR=/tmp/xberg-cache
```

**Directory Structure**: Xberg creates subdirectories for different cache types:

```text
$XBERG_CACHE_DIR/
  ocr/                    # OCR result cache
  embeddings/             # Chunk embedding cache
  extractions/            # Full extraction cache
```

### XBERG_CI_DEBUG

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (unset)

Enable detailed debug logging for CI environments. Outputs step-by-step timing and parameter information for OCR operations.

```bash title="Enable CI Debug Logging"
# Enable CI debug output
export XBERG_CI_DEBUG=1
export XBERG_CI_DEBUG=true
export XBERG_CI_DEBUG=yes

# Output example:
# [xberg::ocr] perform_ocr:start bytes=1024000 language=eng output=text use_cache=true
# [xberg::ocr] perform_ocr:end duration_ms=2534
```

**Use Cases**:

- Debugging slow OCR operations
- Tracing cache hits/misses
- Performance profiling in CI pipelines
- Understanding extraction pipeline behavior

### XBERG_DEBUG_OCR

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (unset)

Enable OCR-specific debug output. Outputs diagnostic information about OCR decisions, fallbacks, and text coverage metrics.

```bash title="Enable OCR Debug Logging"
# Enable OCR debug logging
export XBERG_DEBUG_OCR=1

# Output example:
# [xberg::pdf::ocr] fallback=true non_whitespace=8543 alnum=7234 meaningful_words=312
# [xberg::pdf::ocr] avg_non_whitespace=45.2 avg_alnum=38.1 alnum_ratio=0.847
```

**Diagnostic Information**:

- Whether OCR fallback was triggered
- Character counts (whitespace, alphanumeric)
- Word counts and coverage ratios
- Coverage thresholds and decisions

## Memory & Performance

Configure caching for string encoding operations to optimize performance.

### XBERG_ENCODING_CACHE_MAX_ENTRIES

**Type**: `usize` (positive integer)
**Default**: `10000`

Maximum number of strings cached in the encoding cache. Each entry consumes memory proportional to string length.

```bash title="Encoding Cache Entry Limit"
# Default: reasonable for most applications
export XBERG_ENCODING_CACHE_MAX_ENTRIES=10000

# Higher for very large batches
export XBERG_ENCODING_CACHE_MAX_ENTRIES=50000

# Lower to reduce memory usage
export XBERG_ENCODING_CACHE_MAX_ENTRIES=1000
```

### XBERG_ENCODING_CACHE_MAX_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum total size of cached strings in bytes. Once exceeded, least-used entries are evicted.

```bash title="Encoding Cache Size Limit"
# Default: 100 MB
export XBERG_ENCODING_CACHE_MAX_BYTES=104857600

# Larger cache for high-throughput scenarios
export XBERG_ENCODING_CACHE_MAX_BYTES=524288000  # 500 MB

# Smaller cache for memory-constrained environments
export XBERG_ENCODING_CACHE_MAX_BYTES=10485760   # 10 MB
```

## LLM Integration

Configure LLM-powered features such as structured extraction, vision-based OCR, and provider-hosted embeddings.

### XBERG_LLM_MODEL

**Type**: `String`
**Default**: None (must be set explicitly or via config)

Default LLM model for structured extraction. Uses [liter-llm](https://github.com/xberg-io/liter-llm) model format (`provider/model-name`).

```bash title="LLM Model Configuration"
# OpenAI
export XBERG_LLM_MODEL=openai/gpt-4o-mini

# Anthropic
export XBERG_LLM_MODEL=anthropic/claude-sonnet-4-20250514

# Local provider
export XBERG_LLM_MODEL=ollama/llama3
```

### XBERG_LLM_API_KEY

**Type**: `String`
**Default**: unset

Xberg-wide API key fallback for LLM-backed features. When set, serves as a fallback for any LLM-backed pipeline feature that doesn't have an explicit `api_key` in its config.

**Used by**: VLM OCR, structured extraction, embeddings, NER (LLM backend), redaction (NER), summarisation (abstractive), translation, page classification, and VLM image captions.

**Precedence** (highest to lowest):

1. Explicit `api_key` field in the relevant config (`LlmConfig.api_key`, `OcrConfig.vlm_config.api_key`, etc.)
2. Config file's `api_key` (loaded before CLI processing)
3. CLI flag `--api-key`
4. `XBERG_LLM_API_KEY` env var (this entry — Xberg-wide fallback for any LLM feature)
5. Per-provider env var (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`, …) — resolved inside liter-llm

**Local providers** (Ollama, LM Studio, vLLM, llama.cpp, LocalAI, llamafile) skip every API-key lookup.

```bash title="LLM API Key Configuration"
# Set Xberg-wide fallback (used for any LLM feature without explicit api_key)
export XBERG_LLM_API_KEY=sk-...

# Or use provider-standard env vars (higher precedence within liter-llm fallback chain)
export OPENAI_API_KEY=sk-...
export ANTHROPIC_API_KEY=sk-ant-...
```

**Security Warning**: Prefer using provider-standard environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc.) or a secrets manager over `XBERG_LLM_API_KEY`. This variable is provided for cases where explicit key routing is needed and provider-standard vars are not suitable.

### XBERG_LLM_BASE_URL

**Type**: `String`
**Default**: None (uses provider default)

Custom base URL for the structured extraction LLM provider. Useful for self-hosted models, proxies, or alternative API-compatible endpoints.

```bash title="LLM Base URL Configuration"
# Custom OpenAI-compatible endpoint
export XBERG_LLM_BASE_URL=https://api.example.com

# Local Ollama instance
export XBERG_LLM_BASE_URL=http://localhost:11434
```

### XBERG_VLM_OCR_MODEL

**Type**: `String`
**Default**: None (must be set explicitly or via config)

VLM (Vision Language Model) model for vision-based OCR. When configured, Xberg can use a vision model as an OCR backend, sending document images directly to the VLM for text extraction.

```bash title="VLM OCR Model Configuration"
# OpenAI GPT-4o for vision OCR
export XBERG_VLM_OCR_MODEL=openai/gpt-4o

# Anthropic Claude for vision OCR
export XBERG_VLM_OCR_MODEL=anthropic/claude-sonnet-4-20250514
```

### XBERG_VLM_EMBEDDING_MODEL

**Type**: `String`
**Default**: None (must be set explicitly or via config)

LLM model for provider-hosted embeddings. Instead of running local ONNX embedding models, Xberg can delegate embedding generation to a cloud provider's embedding API.

```bash title="VLM Embedding Model Configuration"
# OpenAI embeddings
export XBERG_VLM_EMBEDDING_MODEL=openai/text-embedding-3-small

# Cohere embeddings
export XBERG_VLM_EMBEDDING_MODEL=cohere/embed-english-v3.0
```

**Note**: When `api_key` is not set in config, liter-llm falls back to provider-standard environment variables (for example, `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`).

| Variable                        | Description                                        | Example                         |
| ------------------------------- | -------------------------------------------------- | ------------------------------- |
| `XBERG_LLM_MODEL`           | Default LLM model for structured extraction        | `openai/gpt-4o-mini`            |
| `XBERG_LLM_API_KEY`         | API key for structured extraction LLM provider     | `sk-...`                        |
| `XBERG_LLM_BASE_URL`        | Custom base URL for structured extraction provider | `https://api.example.com`       |
| `XBERG_VLM_OCR_MODEL`       | VLM model for vision-based OCR                     | `openai/gpt-4o`                 |
| `XBERG_VLM_EMBEDDING_MODEL` | LLM model for provider-hosted embeddings           | `openai/text-embedding-3-small` |

---

## Testing Variables

Variables for development, testing, and quality assurance.

### XBERG_RUN_FULL_OCR

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (skips expensive tests)
**Status**: Testing only

Enable expensive OCR quality tests. These tests perform full OCR on large documents and are slow (can take minutes).

```bash title="Enable Full OCR Tests"
# Skip expensive OCR tests (default, fast test runs)
# unset XBERG_RUN_FULL_OCR

# Run full OCR quality tests
export XBERG_RUN_FULL_OCR=1

# In test output:
# test test_ocr_quality_multi_page_consistency ... SKIPPED
# Skipping test_ocr_quality_multi_page_consistency: set XBERG_RUN_FULL_OCR=1 to enable
```

**Warning**:

- These tests can take 10+ minutes
- Require OCR backends to be installed and working
- Produce large temporary files
- Use only in CI/CD for comprehensive validation

## Docker Compose Examples

### Basic Configuration

```yaml title="Docker Compose - Basic Setup"
version: "3.8"
services:
  xberg:
    image: xberg:latest
    ports:
      - "3000:3000"
    environment:
      XBERG_HOST: "0.0.0.0"
      XBERG_PORT: "3000"
      XBERG_OCR_LANGUAGE: "eng"
      XBERG_CACHE_ENABLED: "true"
```

### Production Configuration

```yaml title="Docker Compose - Production Setup"
version: "3.8"
services:
  xberg:
    image: xberg:latest
    ports:
      - "8000:8000"
    volumes:
      - xberg_cache:/data/cache
    environment:
      XBERG_HOST: "0.0.0.0"
      XBERG_PORT: "8000"
      XBERG_CORS_ORIGINS: "https://app.example.com, https://admin.example.com"
      XBERG_MAX_REQUEST_BODY_BYTES: "209715200" # 200 MB
      XBERG_MAX_MULTIPART_FIELD_BYTES: "209715200"
      XBERG_CACHE_DIR: "/data/cache"
      XBERG_OCR_LANGUAGE: "eng"
      XBERG_OCR_BACKEND: "tesseract"
      XBERG_CHUNKING_MAX_CHARS: "2000"
      XBERG_CHUNKING_MAX_OVERLAP: "300"
      XBERG_TOKEN_REDUCTION_MODE: "moderate"

volumes:
  xberg_cache:
    driver: local
```

### Multilingual Configuration

```yaml title="Docker Compose - Multilingual Setup"
version: "3.8"
services:
  xberg:
    image: xberg:latest
    ports:
      - "8000:8000"
    environment:
      XBERG_HOST: "0.0.0.0"
      XBERG_PORT: "8000"
      XBERG_OCR_BACKEND: "easyocr" # Better multilingual support
      XBERG_OCR_LANGUAGE: "fra" # French
      XBERG_CACHE_ENABLED: "true"
```

### Development Configuration

```yaml title="Docker Compose - Development Setup"
version: "3.8"
services:
  xberg:
    image: xberg:latest
    ports:
      - "8000:8000"
    environment:
      XBERG_HOST: "127.0.0.1"
      XBERG_PORT: "8000"
      XBERG_CACHE_ENABLED: "false" # Disable for fresh testing
      XBERG_CI_DEBUG: "1" # Enable debug output
      XBERG_DEBUG_OCR: "1"
      XBERG_CACHE_DIR: "/tmp/xberg"
```

## Environment Variable Loading Order

Xberg applies environment variables in this order:

1. Load configuration file (TOML/YAML/JSON) if specified
2. Parse environment variables using `apply_env_overrides()`
3. Validate all settings

This ensures environment variables always win over file configuration:

```rust title="Rust - Applying Environment Overrides"
let mut config = ExtractionConfig::from_file("xberg.toml")?;
config.apply_env_overrides()?;  // Overrides file values
```

## Common Patterns

### Using with Config Files

Combine files with environment overrides for flexibility:

```bash title="Combining Config Files with Env Overrides"
# Load base config from file
# Override specific values for this deployment
export XBERG_OCR_LANGUAGE=deu
export XBERG_CACHE_DIR=/mnt/cache
xberg --config xberg.toml
```

### Shell Script Initialization

```bash title="Environment-Based Shell Script"
#!/bin/bash
# Load deployment-specific settings

if [ "$ENVIRONMENT" = "production" ]; then
  export XBERG_HOST="0.0.0.0"
  export XBERG_CORS_ORIGINS="https://app.example.com"
  export XBERG_CACHE_ENABLED="true"
  export XBERG_MAX_REQUEST_BODY_BYTES=$((200 * 1048576))
elif [ "$ENVIRONMENT" = "development" ]; then
  export XBERG_HOST="127.0.0.1"
  export XBERG_CACHE_ENABLED="false"
  export XBERG_CI_DEBUG="1"
fi

xberg
```

### Container Environment Block

```yaml title="container-env.yaml"
environment:
  XBERG_HOST: "0.0.0.0"
  XBERG_PORT: "8000"
  XBERG_CORS_ORIGINS: "https://api.example.com"
  XBERG_CACHE_DIR: "/data/cache"
  XBERG_OCR_BACKEND: "tesseract"
  XBERG_TOKEN_REDUCTION_MODE: "moderate"
volumes:
  - source: xberg-cache
    target: /data/cache
```

## ONNX Runtime Configuration

### ORT_DYLIB_PATH

**Type**: `String`
**Default**: Not set (bundled CPU ONNX Runtime is used)

Path to a custom ONNX Runtime shared library. Set this to use a GPU-enabled ONNX Runtime instead of the bundled CPU-only version.

Required for GPU acceleration (`cuda`, `tensorrt`) with PaddleOCR, layout detection, embeddings, and document orientation detection.

```bash title="GPU Acceleration Setup"
# Linux — using ONNX Runtime GPU release
export ORT_DYLIB_PATH=/usr/local/lib/libonnxruntime.so

# Linux — using pip-installed onnxruntime-gpu
export ORT_DYLIB_PATH=$(python -c "import onnxruntime; print(onnxruntime.__path__[0])")/capi/libonnxruntime.so

# macOS — using Homebrew
export ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib

# Windows
set ORT_DYLIB_PATH=C:\path\to\onnxruntime.dll
```

When not set, Xberg auto-discovers system-installed ONNX Runtime on common paths. If no system library is found, the bundled CPU-only version is used.

## See Also

- [Configuration Guide](./configuration.md) - Detailed configuration file format and options
- [File Size Limits](./file-size-limits.md) - Upload and processing limits
- [Types Reference](./types.md) - API type definitions and structures
