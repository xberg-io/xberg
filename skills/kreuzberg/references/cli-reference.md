# Kreuzberg CLI Reference

Comprehensive command-line interface for the Kreuzberg document intelligence library.

## Installation

Install from crates.io:

```bash
cargo install kreuzberg-cli
```

Or download pre-built binaries from [GitHub Releases](https://github.com/lukasmwirth/kreuzberg/releases).

## Commands

### extract

Extract text and structure from a single document.

```bash
kreuzberg extract <path> [FLAGS]
```

**Positional Arguments**

- `<path>` — Path to the document file

**Flags**

- `-c, --config <path>` — Path to config file (TOML, YAML, or JSON). Auto-discovers `kreuzberg.{toml,yaml,json}` in current and parent directories if omitted.
- `--config-json <json>` — Inline JSON configuration (merged after config file, before CLI flags).
- `--config-json-base64 <base64>` — Base64-encoded JSON configuration.
- `-m, --mime-type <type>` — MIME type hint (auto-detected if not provided).
- `-f, --format <text|json>` — CLI output format (default: `text`). Controls how results display, not extraction content format.
- `--output-format <plain|markdown|djot|html>` — Extraction content format (default: `plain`). Controls format of extracted content.
- `--ocr <bool>` — Enable OCR processing.
- `--force-ocr <bool>` — Force OCR even if text extraction succeeds.
- `--no-cache <bool>` — Disable caching.
- `--chunk <bool>` — Enable text chunking.
- `--chunk-size <n>` — Chunk size in characters.
- `--chunk-overlap <n>` — Chunk overlap in characters.
- `--quality <bool>` — Enable quality processing.
- `--detect-language <bool>` — Enable language detection.

**Examples**

```bash
# Extract with default settings
kreuzberg extract document.pdf

# Extract with OCR enabled
kreuzberg extract scanned.pdf --ocr true

# Extract with specific output format
kreuzberg extract doc.docx --output-format markdown

# Extract with inline JSON config
kreuzberg extract file.pdf --config-json '{"ocr":{"backend":"tesseract"}}'

# Extract with base64-encoded config
kreuzberg extract file.pdf --config-json-base64 eyJvY3IiOnsiYmFja2VuZCI6InRlc3NlcmFjdCJ9fQ==

# Extract and output as JSON
kreuzberg extract doc.pdf --format json

# Extract with chunking
kreuzberg extract large-doc.pdf --chunk true --chunk-size 2000 --chunk-overlap 200
```

### batch

Batch extract from multiple documents in parallel.

```bash
kreuzberg batch <paths...> [FLAGS]
```

**Positional Arguments**

- `<paths...>` — One or more document file paths

**Flags**

- `-c, --config <path>` — Path to config file (TOML, YAML, or JSON). Auto-discovers `kreuzberg.{toml,yaml,json}` in current and parent directories if omitted.
- `--config-json <json>` — Inline JSON configuration (merged after config file, before CLI flags).
- `--config-json-base64 <base64>` — Base64-encoded JSON configuration.
- `-f, --format <text|json>` — CLI output format (default: `json`). Controls how results display, not extraction content format.
- `--output-format <plain|markdown|djot|html>` — Extraction content format (default: `plain`). Controls format of extracted content.
- `--ocr <bool>` — Enable OCR processing.
- `--force-ocr <bool>` — Force OCR even if text extraction succeeds.
- `--no-cache <bool>` — Disable caching.
- `--quality <bool>` — Enable quality processing.

**Notes**

- Batch command defaults to JSON output format (unlike `extract` which defaults to text).
- Does not support chunking parameters (`--chunk`, `--chunk-size`, `--chunk-overlap`).
- Does not support `--mime-type` or `--detect-language` flags.

**Examples**

```bash
# Batch extract multiple PDFs
kreuzberg batch document1.pdf document2.pdf document3.pdf

# Batch extract with glob patterns (shell expansion)
kreuzberg batch *.pdf

# Batch extract with custom output format
kreuzberg batch doc1.pdf doc2.pdf --output-format markdown

# Batch extract with OCR
kreuzberg batch scanned*.pdf --ocr true

# Batch extract with text output format
kreuzberg batch files*.docx --format text
```

### detect

Identify MIME type of a file.

```bash
kreuzberg detect <path> [FLAGS]
```

**Positional Arguments**

- `<path>` — Path to the file

**Flags**

- `-f, --format <text|json>` — Output format (default: `text`)

**Examples**

```bash
# Detect MIME type (text output)
kreuzberg detect unknown-file.bin

# Detect MIME type (JSON output)
kreuzberg detect file.xyz --format json
```

### version

Display version information.

```bash
kreuzberg version [FLAGS]
```

**Flags**

- `-f, --format <text|json>` — Output format (default: `text`)

**Examples**

```bash
# Show version as text
kreuzberg version

# Show version as JSON
kreuzberg version --format json
```

### cache

Manage extraction cache.

#### cache stats

Display cache statistics.

```bash
kreuzberg cache stats [FLAGS]
```

**Flags**

- `--cache-dir <path>` — Cache directory (default: `.kreuzberg` in current directory)
- `-f, --format <text|json>` — Output format (default: `text`)

**Examples**

```bash
# Show cache stats
kreuzberg cache stats

# Show cache stats as JSON
kreuzberg cache stats --format json

# Show stats for specific cache directory
kreuzberg cache stats --cache-dir /tmp/my-cache
```

#### cache clear

Clear all cached extractions.

```bash
kreuzberg cache clear [FLAGS]
```

**Flags**

- `--cache-dir <path>` — Cache directory (default: `.kreuzberg` in current directory)
- `-f, --format <text|json>` — Output format (default: `text`)

**Examples**

```bash
# Clear cache
kreuzberg cache clear

# Clear specific cache directory
kreuzberg cache clear --cache-dir /tmp/my-cache
```

### serve

Start the API server (requires `api` feature).

```bash
kreuzberg serve [FLAGS]
```

**Flags**

- `-H, --host <host>` — Host to bind to (e.g., `127.0.0.1` or `0.0.0.0`). CLI arg overrides config file and environment variables.
- `-p, --port <port>` — Port to bind to. CLI arg overrides config file and environment variables.
- `-c, --config <path>` — Path to config file (TOML, YAML, or JSON). Auto-discovers `kreuzberg.{toml,yaml,json}` in current and parent directories if omitted.

**Configuration Precedence**

1. CLI arguments (`--host`, `--port`)
2. Environment variables (`KREUZBERG_HOST`, `KREUZBERG_PORT`)
3. Config file (`[server]` section)
4. Built-in defaults (`127.0.0.1:8000`)

**Examples**

```bash
# Start server with defaults
kreuzberg serve

# Start server on specific host and port
kreuzberg serve --host 0.0.0.0 --port 3000

# Start server with config file
kreuzberg serve --config kreuzberg.toml

# Start server (environment variables override defaults)
KREUZBERG_HOST=192.168.1.100 KREUZBERG_PORT=8080 kreuzberg serve
```

### mcp

Start the Model Context Protocol (MCP) server (requires `mcp` feature).

```bash
kreuzberg mcp [FLAGS]
```

**Flags**

- `-c, --config <path>` — Path to config file (TOML, YAML, or JSON). Auto-discovers `kreuzberg.{toml,yaml,json}` in current and parent directories if omitted.
- `--transport <stdio|http>` — Transport mode (default: `stdio`)
- `--host <host>` — HTTP host for http transport (default: `127.0.0.1`)
- `--port <port>` — HTTP port for http transport (default: `8001`)

**Examples**

```bash
# Start MCP server with stdio transport
kreuzberg mcp

# Start MCP server with HTTP transport
kreuzberg mcp --transport http

# Start MCP server on custom HTTP host/port
kreuzberg mcp --transport http --host 0.0.0.0 --port 9000

# Start MCP server with config file
kreuzberg mcp --config kreuzberg.toml
```

## Configuration

### File Format

Configuration files support three formats with automatic detection:

- **TOML** — `.toml` extension (recommended)
- **YAML** — `.yaml` or `.yml` extension
- **JSON** — `.json` extension

### Configuration Precedence

Settings are applied in order from highest to lowest priority:

1. **Individual CLI flags** (e.g., `--ocr true`, `--output-format markdown`)
2. **Inline JSON config** (`--config-json` or `--config-json-base64`)
3. **Config file** (explicit `--config path.toml` or auto-discovered)
4. **Default values** (built-in library defaults)

### Auto-Discovery

When no config file is specified, Kreuzberg searches for configuration in this order:

1. `kreuzberg.toml` in current directory
2. `kreuzberg.yaml` in current directory
3. `kreuzberg.json` in current directory
4. Parent directories (same search pattern, up to filesystem root)

### Example Configuration

```toml
# Top-level extraction options
use_cache = true
enable_quality_processing = true
force_ocr = false
output_format = "markdown"

# OCR settings
[ocr]
backend = "tesseract"
language = "eng"

# Chunking settings
[chunking]
max_chars = 2000
max_overlap = 200

# Language detection
[language_detection]
enabled = true

# Server configuration (for serve command)
[server]
host = "127.0.0.1"
port = 8000
```

## Exit Codes

- `0` — Success
- Non-zero — Error (see stderr for details)

## Error Handling

The CLI validates input and provides clear error messages:

- **File not found** — Verify path exists and is readable
- **Invalid MIME type** — Ensure file is accessible and format is supported
- **Invalid JSON** — Check `--config-json` syntax
- **Invalid config file** — Verify TOML/YAML/JSON format
- **Invalid chunk parameters** — Ensure chunk-size > 0 and overlap < chunk-size

## Environment Variables

- `RUST_LOG` — Set logging level (e.g., `RUST_LOG=debug`)
- `KREUZBERG_HOST` — Server bind host (used by `serve` command)
- `KREUZBERG_PORT` — Server bind port (used by `serve` command)

## Common Patterns

### Extract with Custom Configuration

```bash
kreuzberg extract document.pdf \
  --output-format markdown \
  --ocr true \
  --quality true
```

### Batch Process with Config File

```bash
kreuzberg batch *.pdf --config extraction-config.toml
```

### CI/CD Integration

```bash
# Extract to JSON for downstream processing
kreuzberg extract file.pdf --format json | jq '.content'

# Batch process with error handling
kreuzberg batch docs/*.pdf --format json || exit 1
```

### Performance Tuning

```bash
# Disable cache for temporary processing
kreuzberg extract file.pdf --no-cache true

# Enable chunking for large documents
kreuzberg extract large-file.pdf \
  --chunk true \
  --chunk-size 5000 \
  --chunk-overlap 500
```

## Debugging

Enable detailed logging:

```bash
RUST_LOG=debug kreuzberg extract document.pdf
```

Check cache statistics:

```bash
kreuzberg cache stats --format json
```

Detect file MIME type:

```bash
kreuzberg detect unknown-file --format json
```
