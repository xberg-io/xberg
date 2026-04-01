# Environment Variables Reference

Configuration precedence in Kreuzberg follows this order (highest to lowest):

1. **Environment Variables** - Highest priority, overrides all other sources
2. **Configuration Files** - TOML, YAML, or JSON config files
3. **Defaults** - Built-in sensible defaults

This document covers all KREUZBERG\_\* environment variables for version 4.3.8.

## When to Use Environment Variables

Environment variables are ideal for:

- **Container/Cloud Deployments**: Docker, Kubernetes, serverless environments where config files are impractical
- **CI/CD Pipelines**: Override settings per environment (dev, staging, production)
- **Simple Overrides**: Changing one or two settings without managing a config file
- **Secrets Management**: Using secret management systems that inject values as env vars

For complex configurations with many settings, configuration files are recommended:

```toml title="Example Configuration File"
# kreuzberg.toml is cleaner for multiple settings
[ocr]
language = "eng"
backend = "tesseract"

[chunking]
max_chars = 2000
max_overlap = 300
```

## API Server Configuration

These variables control the Kreuzberg server's network behavior and request handling.

### KREUZBERG_HOST

**Type**: `String`
**Default**: `127.0.0.1`
**Valid Values**: Any IPv4 or IPv6 address, or hostname

The server bind address. Use `0.0.0.0` to listen on all interfaces.

```bash title="Server Bind Address Examples"
# Listen only on localhost (default)
export KREUZBERG_HOST=127.0.0.1

# Listen on all interfaces (Docker, cloud deployments)
export KREUZBERG_HOST=0.0.0.0

# Listen on specific interface
export KREUZBERG_HOST=192.168.1.100
```

### KREUZBERG_PORT

**Type**: `u16` (1-65535)
**Default**: `8000`

The server port number.

```bash title="Server Port Examples"
export KREUZBERG_PORT=3000
export KREUZBERG_PORT=8080
```

**Error**: Port must be a valid u16 number:

```text
KREUZBERG_PORT must be a valid u16 number, got 'invalid': invalid digit found in string
```

### KREUZBERG_CORS_ORIGINS

**Type**: `String` (comma-separated list)
**Default**: Empty (allows all origins)

Whitelist of allowed CORS origins. When empty, the server accepts requests from any origin.

```bash title="CORS Origins Configuration"
# Allow all origins (default)
# unset KREUZBERG_CORS_ORIGINS

# Allow specific origins
export KREUZBERG_CORS_ORIGINS="https://api.example.com, https://app.example.com"

# Single origin
export KREUZBERG_CORS_ORIGINS="https://trusted.com"
```

**Security Warning**: Be explicit with CORS origins in production. Allowing all origins (`*`) means any website can call your API on behalf of users. In Kreuzberg, an empty list allows all origins - be intentional about this choice.

```bash title="CORS Security Best Practices"
# Production: Restrict to known origins
export KREUZBERG_CORS_ORIGINS="https://app.mycompany.com, https://admin.mycompany.com"

# Development: Can use wildcard, but understand the security implications
# Don't use wildcard in production unless absolutely necessary
```

### KREUZBERG_MAX_REQUEST_BODY_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum size of HTTP request bodies. Prevents oversized requests from consuming server resources.

```bash title="Max Request Body Size Examples"
# 50 MB
export KREUZBERG_MAX_REQUEST_BODY_BYTES=52428800

# 200 MB
export KREUZBERG_MAX_REQUEST_BODY_BYTES=209715200

# 500 MB
export KREUZBERG_MAX_REQUEST_BODY_BYTES=524288000
```

**Note**: Both `KREUZBERG_MAX_REQUEST_BODY_BYTES` and `KREUZBERG_MAX_MULTIPART_FIELD_BYTES` control upload limits. Adjust both for consistent behavior.

### KREUZBERG_MAX_MULTIPART_FIELD_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum size of individual multipart form fields. Controls the size of file uploads in multipart requests.

```bash title="Max Multipart Field Size Examples"
# 100 MB (default)
export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=104857600

# 500 MB for large document processing
export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=524288000

# 1 GB for extreme cases
export KREUZBERG_MAX_MULTIPART_FIELD_BYTES=1073741824
```

## Extraction Configuration

These variables control document extraction behavior, including OCR, text chunking, and caching.

### KREUZBERG_OCR_LANGUAGE

**Type**: `String` (ISO 639-1 or 639-3 language code)
**Default**: `eng` (English)

OCR language for scanned documents. Must be a valid language code recognized by the OCR backend.

```bash title="OCR Language Configuration"
# English (default)
export KREUZBERG_OCR_LANGUAGE=eng

# German
export KREUZBERG_OCR_LANGUAGE=deu

# French
export KREUZBERG_OCR_LANGUAGE=fra

# Spanish
export KREUZBERG_OCR_LANGUAGE=spa

# Chinese (Simplified)
export KREUZBERG_OCR_LANGUAGE=chi_sim

# Japanese
export KREUZBERG_OCR_LANGUAGE=jpn
```

**Supported Codes**: Language codes are backend-agnostic and automatically mapped to the appropriate format for each backend:

- **Tesseract codes** (ISO 639-3): `eng`, `deu`, `fra`, `spa`, `ita`, `por`, `rus`, `chi_sim`, `chi_tra`, `jpn`, `kor`
- **PaddleOCR codes**: `en`, `ch`, `french`, `german`, `korean`, `thai`, `greek`, `cyrillic`, `latin`, `arabic`, `devanagari`, `tamil`, `telugu`
- **ISO 639-1 codes**: `en`, `de`, `fr`, `es`, `ja`, `ko`, `zh`, `ru`, `ar`, `th`, `el`

All code formats are accepted regardless of backend — Kreuzberg automatically maps between them.

### KREUZBERG_OCR_BACKEND

**Type**: `String`
**Default**: `tesseract`
**Valid Values**: `tesseract`, `easyocr`, `paddleocr`

OCR engine to use for text extraction from images and scanned documents.

```bash title="OCR Backend Selection"
# Tesseract (open source, good for English)
export KREUZBERG_OCR_BACKEND=tesseract

# EasyOCR (better multilingual support, slower)
export KREUZBERG_OCR_BACKEND=easyocr

# PaddleOCR (fast, good accuracy across languages)
export KREUZBERG_OCR_BACKEND=paddleocr
```

**Performance Notes**:

- **tesseract**: Fastest, best for English and Latin scripts
- **easyocr**: Slower, excellent multilingual support
- **paddleocr**: Fast with good accuracy for many languages

### KREUZBERG_CHUNKING_MAX_CHARS

**Type**: `usize` (positive integer)
**Default**: `1000` (characters)

Maximum number of characters per text chunk. Smaller chunks are useful for LLM context windows.

```bash title="Chunk Size Configuration"
# Small chunks for token-constrained LLMs
export KREUZBERG_CHUNKING_MAX_CHARS=512

# Default: balanced for most use cases
export KREUZBERG_CHUNKING_MAX_CHARS=1000

# Larger chunks for fewer splits
export KREUZBERG_CHUNKING_MAX_CHARS=2000

# Very large chunks for comprehensive context
export KREUZBERG_CHUNKING_MAX_CHARS=4000
```

**Validation**: Must be greater than 0. Must be greater than `KREUZBERG_CHUNKING_MAX_OVERLAP`.

### KREUZBERG_CHUNKING_MAX_OVERLAP

**Type**: `usize` (non-negative integer)
**Default**: `200` (characters)

Character overlap between consecutive chunks. Maintains context across chunk boundaries.

```bash title="Chunk Overlap Configuration"
# No overlap (creates discontinuities)
export KREUZBERG_CHUNKING_MAX_OVERLAP=0

# Default: 20% overlap with 1000-char chunks
export KREUZBERG_CHUNKING_MAX_OVERLAP=200

# More overlap: 30% for better context continuity
export KREUZBERG_CHUNKING_MAX_OVERLAP=300

# High overlap for sensitive documents
export KREUZBERG_CHUNKING_MAX_OVERLAP=500
```

**Validation**: Must be less than `KREUZBERG_CHUNKING_MAX_CHARS`.

**Example Error**:

```text
Chunking overlap (500) cannot be greater than or equal to max_chars (1000)
```

### KREUZBERG_CACHE_ENABLED

**Type**: `Boolean` (`true` or `false`, case-insensitive)
**Default**: `true`

Enable or disable extraction result caching. Cache stores results to avoid reprocessing identical documents.

```bash title="Cache Enable/Disable"
# Enable cache (default, recommended for production)
export KREUZBERG_CACHE_ENABLED=true

# Disable cache (development, testing, or when cache is problematic)
export KREUZBERG_CACHE_ENABLED=false

# Case insensitive
export KREUZBERG_CACHE_ENABLED=TRUE
export KREUZBERG_CACHE_ENABLED=False
```

### KREUZBERG_OUTPUT_FORMAT

**Type**: `String`
**Default**: `plain`
**Valid Values**: `plain`, `markdown`, `djot`, `html`

Controls the text content format of extraction results. Determines how extracted text is formatted in the result output.

```bash title="Output Format Options"
# Plain text content only (default)
export KREUZBERG_OUTPUT_FORMAT=plain

# Markdown formatted output
export KREUZBERG_OUTPUT_FORMAT=markdown

# Djot markup format
export KREUZBERG_OUTPUT_FORMAT=djot

# HTML formatted output
export KREUZBERG_OUTPUT_FORMAT=html
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
export KREUZBERG_OUTPUT_FORMAT=markdown
kreuzberg
```

### KREUZBERG_TOKEN_REDUCTION_MODE

**Type**: `String`
**Default**: `off`
**Valid Values**: `off`, `light`, `moderate`, `aggressive`, `maximum`

Token reduction aggressiveness for compressing extracted text while preserving meaning. Useful when working with token-limited LLMs.

```bash title="Token Reduction Mode Options"
# No reduction (keep all text as-is)
export KREUZBERG_TOKEN_REDUCTION_MODE=off

# Light reduction: Remove common stopwords, minimal impact
export KREUZBERG_TOKEN_REDUCTION_MODE=light

# Moderate reduction: Balance between compression and meaning preservation
export KREUZBERG_TOKEN_REDUCTION_MODE=moderate

# Aggressive reduction: Significant compression, some detail loss
export KREUZBERG_TOKEN_REDUCTION_MODE=aggressive

# Maximum reduction: Extreme compression for token-constrained scenarios
export KREUZBERG_TOKEN_REDUCTION_MODE=maximum
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

### KREUZBERG_CACHE_DIR

**Type**: `String` (file system path)
**Default**: `.kreuzberg/` (current directory)

Custom directory for storing extraction cache and intermediate files. Useful for managing disk usage across multiple Kreuzberg instances.

```bash title="Cache Directory Configuration"
# Default: cache in current directory
# unset KREUZBERG_CACHE_DIR  # Uses .kreuzberg/

# Store cache in specific location
export KREUZBERG_CACHE_DIR=/var/cache/kreuzberg

# Docker: Use volume mount
export KREUZBERG_CACHE_DIR=/data/kreuzberg-cache

# Development: Quick local cleanup
export KREUZBERG_CACHE_DIR=/tmp/kreuzberg-cache
```

**Directory Structure**: Kreuzberg creates subdirectories for different cache types:

```text
$KREUZBERG_CACHE_DIR/
  ocr/                    # OCR result cache
  embeddings/             # Chunk embedding cache
  extractions/            # Full extraction cache
```

### KREUZBERG_CI_DEBUG

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (unset)

Enable detailed debug logging for CI environments. Outputs step-by-step timing and parameter information for OCR operations.

```bash title="Enable CI Debug Logging"
# Enable CI debug output
export KREUZBERG_CI_DEBUG=1
export KREUZBERG_CI_DEBUG=true
export KREUZBERG_CI_DEBUG=yes

# Output example:
# [kreuzberg::ocr] perform_ocr:start bytes=1024000 language=eng output=text use_cache=true
# [kreuzberg::ocr] perform_ocr:end duration_ms=2534
```

**Use Cases**:

- Debugging slow OCR operations
- Tracing cache hits/misses
- Performance profiling in CI pipelines
- Understanding extraction pipeline behavior

### KREUZBERG_DEBUG_OCR

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (unset)

Enable OCR-specific debug output. Outputs diagnostic information about OCR decisions, fallbacks, and text coverage metrics.

```bash title="Enable OCR Debug Logging"
# Enable OCR debug logging
export KREUZBERG_DEBUG_OCR=1

# Output example:
# [kreuzberg::pdf::ocr] fallback=true non_whitespace=8543 alnum=7234 meaningful_words=312
# [kreuzberg::pdf::ocr] avg_non_whitespace=45.2 avg_alnum=38.1 alnum_ratio=0.847
```

**Diagnostic Information**:

- Whether OCR fallback was triggered
- Character counts (whitespace, alphanumeric)
- Word counts and coverage ratios
- Coverage thresholds and decisions

## Memory & Performance

Configure caching for string encoding operations to optimize performance.

### KREUZBERG_ENCODING_CACHE_MAX_ENTRIES

**Type**: `usize` (positive integer)
**Default**: `10000`

Maximum number of strings cached in the encoding cache. Each entry consumes memory proportional to string length.

```bash title="Encoding Cache Entry Limit"
# Default: reasonable for most applications
export KREUZBERG_ENCODING_CACHE_MAX_ENTRIES=10000

# Higher for very large batches
export KREUZBERG_ENCODING_CACHE_MAX_ENTRIES=50000

# Lower to reduce memory usage
export KREUZBERG_ENCODING_CACHE_MAX_ENTRIES=1000
```

### KREUZBERG_ENCODING_CACHE_MAX_BYTES

**Type**: `usize` (bytes)
**Default**: `104857600` (100 MB)

Maximum total size of cached strings in bytes. Once exceeded, least-used entries are evicted.

```bash title="Encoding Cache Size Limit"
# Default: 100 MB
export KREUZBERG_ENCODING_CACHE_MAX_BYTES=104857600

# Larger cache for high-throughput scenarios
export KREUZBERG_ENCODING_CACHE_MAX_BYTES=524288000  # 500 MB

# Smaller cache for memory-constrained environments
export KREUZBERG_ENCODING_CACHE_MAX_BYTES=10485760   # 10 MB
```

## Testing Variables

Variables for development, testing, and quality assurance.

### KREUZBERG_RUN_FULL_OCR

**Type**: `Boolean` (presence check: set to any value to enable)
**Default**: Disabled (skips expensive tests)
**Status**: Testing only

Enable expensive OCR quality tests. These tests perform full OCR on large documents and are slow (can take minutes).

```bash title="Enable Full OCR Tests"
# Skip expensive OCR tests (default, fast test runs)
# unset KREUZBERG_RUN_FULL_OCR

# Run full OCR quality tests
export KREUZBERG_RUN_FULL_OCR=1

# In test output:
# test test_ocr_quality_multi_page_consistency ... SKIPPED
# Skipping test_ocr_quality_multi_page_consistency: set KREUZBERG_RUN_FULL_OCR=1 to enable
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
  kreuzberg:
    image: kreuzberg:latest
    ports:
      - "3000:3000"
    environment:
      KREUZBERG_HOST: "0.0.0.0"
      KREUZBERG_PORT: "3000"
      KREUZBERG_OCR_LANGUAGE: "eng"
      KREUZBERG_CACHE_ENABLED: "true"
```

### Production Configuration

```yaml title="Docker Compose - Production Setup"
version: "3.8"
services:
  kreuzberg:
    image: kreuzberg:latest
    ports:
      - "8000:8000"
    volumes:
      - kreuzberg_cache:/data/cache
    environment:
      KREUZBERG_HOST: "0.0.0.0"
      KREUZBERG_PORT: "8000"
      KREUZBERG_CORS_ORIGINS: "https://app.example.com, https://admin.example.com"
      KREUZBERG_MAX_REQUEST_BODY_BYTES: "209715200" # 200 MB
      KREUZBERG_MAX_MULTIPART_FIELD_BYTES: "209715200"
      KREUZBERG_CACHE_DIR: "/data/cache"
      KREUZBERG_OCR_LANGUAGE: "eng"
      KREUZBERG_OCR_BACKEND: "tesseract"
      KREUZBERG_CHUNKING_MAX_CHARS: "2000"
      KREUZBERG_CHUNKING_MAX_OVERLAP: "300"
      KREUZBERG_TOKEN_REDUCTION_MODE: "moderate"

volumes:
  kreuzberg_cache:
    driver: local
```

### Multilingual Configuration

```yaml title="Docker Compose - Multilingual Setup"
version: "3.8"
services:
  kreuzberg:
    image: kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      KREUZBERG_HOST: "0.0.0.0"
      KREUZBERG_PORT: "8000"
      KREUZBERG_OCR_BACKEND: "easyocr" # Better multilingual support
      KREUZBERG_OCR_LANGUAGE: "fra" # French
      KREUZBERG_CACHE_ENABLED: "true"
```

### Development Configuration

```yaml title="Docker Compose - Development Setup"
version: "3.8"
services:
  kreuzberg:
    image: kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      KREUZBERG_HOST: "127.0.0.1"
      KREUZBERG_PORT: "8000"
      KREUZBERG_CACHE_ENABLED: "false" # Disable for fresh testing
      KREUZBERG_CI_DEBUG: "1" # Enable debug output
      KREUZBERG_DEBUG_OCR: "1"
      KREUZBERG_CACHE_DIR: "/tmp/kreuzberg"
```

## Environment Variable Loading Order

Kreuzberg applies environment variables in this order:

1. Load configuration file (TOML/YAML/JSON) if specified
2. Parse environment variables using `apply_env_overrides()`
3. Validate all settings

This ensures environment variables always win over file configuration:

```rust title="Rust - Applying Environment Overrides"
let mut config = ExtractionConfig::from_file("kreuzberg.toml")?;
config.apply_env_overrides()?;  // Overrides file values
```

## Common Patterns

### Using with Config Files

Combine files with environment overrides for flexibility:

```bash title="Combining Config Files with Env Overrides"
# Load base config from file
# Override specific values for this deployment
export KREUZBERG_OCR_LANGUAGE=deu
export KREUZBERG_CACHE_DIR=/mnt/cache
kreuzberg --config kreuzberg.toml
```

### Shell Script Initialization

```bash title="Environment-Based Shell Script"
#!/bin/bash
# Load deployment-specific settings

if [ "$ENVIRONMENT" = "production" ]; then
  export KREUZBERG_HOST="0.0.0.0"
  export KREUZBERG_CORS_ORIGINS="https://app.example.com"
  export KREUZBERG_CACHE_ENABLED="true"
  export KREUZBERG_MAX_REQUEST_BODY_BYTES=$((200 * 1048576))
elif [ "$ENVIRONMENT" = "development" ]; then
  export KREUZBERG_HOST="127.0.0.1"
  export KREUZBERG_CACHE_ENABLED="false"
  export KREUZBERG_CI_DEBUG="1"
fi

kreuzberg
```

### Kubernetes ConfigMap

```yaml title="Kubernetes ConfigMap and Pod Configuration"
apiVersion: v1
kind: ConfigMap
metadata:
  name: kreuzberg-config
data:
  KREUZBERG_HOST: "0.0.0.0"
  KREUZBERG_PORT: "8000"
  KREUZBERG_CORS_ORIGINS: "https://api.example.com"
  KREUZBERG_CACHE_DIR: "/data/cache"
  KREUZBERG_OCR_BACKEND: "tesseract"
  KREUZBERG_TOKEN_REDUCTION_MODE: "moderate"
---
apiVersion: v1
kind: Pod
metadata:
  name: kreuzberg-server
spec:
  containers:
    - name: kreuzberg
      image: kreuzberg:latest
      ports:
        - containerPort: 8000
      envFrom:
        - configMapRef:
            name: kreuzberg-config
      volumeMounts:
        - name: cache
          mountPath: /data/cache
  volumes:
    - name: cache
      persistentVolumeClaim:
        claimName: kreuzberg-cache-pvc
```

## See Also

- [Configuration Guide](./configuration.md) - Detailed configuration file format and options
- [File Size Limits](./file-size-limits.md) - Upload and processing limits
- [Types Reference](./types.md) - API type definitions and structures
