# CLI Usage

The Kreuzberg CLI provides command-line access to all extraction features. This guide covers installation, basic usage, and advanced features.

## Installation

=== "Install Script (Linux/macOS)"

    --8<-- "snippets/cli/install_script.md"

=== "Homebrew (macOS/Linux)"

    --8<-- "snippets/cli/install_homebrew.md"

=== "Cargo (Cross-platform)"

    --8<-- "snippets/cli/install_cargo.md"

=== "Docker"

    --8<-- "snippets/cli/install_docker.md"

=== "Go (SDK)"

    --8<-- "snippets/cli/install_go_sdk.md"

!!! info "Feature Availability"
**Homebrew Installation:**

    - ✅ Text extraction (PDF, Office, images, 88+ formats)
    - ✅ OCR with Tesseract
    - ✅ HTTP API server (`serve` command)
    - ✅ MCP protocol server (`mcp` command)
    - ✅ Chunking, quality scoring, language detection
    - ❌ **Embeddings** - Not available via CLI flags. Use config file or Docker image.

    **Docker Images:**

    - All features enabled including embeddings (ONNX Runtime included)

## Global Flags <span class="version-badge">v4.5.2</span>

### Log Level <span class="version-badge">v4.5.2</span>

Control the verbosity of log output with the `--log-level` flag. This overrides the `RUST_LOG` environment variable.

```bash title="Terminal"
# Set log level to debug for troubleshooting
kreuzberg --log-level debug extract document.pdf

# Suppress all but error messages
kreuzberg --log-level error batch documents/*.pdf

# Trace-level logging for maximum detail
kreuzberg --log-level trace extract document.pdf
```

Valid levels: `trace`, `debug`, `info` (default), `warn`, `error`.

### Colored Output

Text output is colored by default. To disable colors, set the `NO_COLOR` environment variable:

```bash title="Terminal"
# Disable colored output
NO_COLOR=1 kreuzberg extract document.pdf
```

## Basic Usage

### Extract from Single File

```bash title="Terminal"
# Extract text content to stdout
kreuzberg extract document.pdf

# Specify MIME type (auto-detected if not provided)
kreuzberg extract document.pdf --mime-type application/pdf
```

### Batch Extract Multiple Files

Use the `batch` command to extract from multiple files:

```bash title="Terminal"
# Extract from multiple files
kreuzberg batch doc1.pdf doc2.docx doc3.txt

# Batch extract all PDFs in directory
kreuzberg batch documents/*.pdf

# Batch extract recursively
kreuzberg batch documents/**/*.pdf
```

### Output Formats

```bash title="Terminal"
# Output as plain text (default for extract)
kreuzberg extract document.pdf --format text

# Output as JSON (default for batch)
kreuzberg batch documents/*.pdf --format json

# Extract single file as JSON
kreuzberg extract document.pdf --format json
```

### Content Output Format

Control the formatting of extracted text content:

```bash title="Terminal"
# Extract as plain text (default)
kreuzberg extract document.pdf --output-format plain

# Extract as Markdown
kreuzberg extract document.pdf --output-format markdown

# Extract as Djot markup
kreuzberg extract document.pdf --output-format djot

# Extract as HTML
kreuzberg extract document.pdf --output-format html
```

The `--output-format` flag controls how the extracted text is formatted. This is different from `--format` which controls the output structure (text vs JSON).

## OCR Extraction

### Enable OCR

```bash title="Terminal"
# Enable OCR (overrides config file setting)
kreuzberg extract scanned.pdf --ocr true

# Disable OCR
kreuzberg extract document.pdf --ocr false
```

### Force OCR

Force OCR even for PDFs with text layer:

```bash title="Terminal"
# Force OCR to run regardless of existing text
kreuzberg extract document.pdf --force-ocr true
```

### OCR Language Selection

Set the OCR language using the `--ocr-language` flag. This flag is backend-agnostic and works with all supported OCR backends (Tesseract, PaddleOCR, EasyOCR).

**Language Code Formats:**

- **Tesseract**: Uses ISO 639-3 codes (three-letter codes)
  - Examples: `eng` (English), `fra` (French), `deu` (German), `spa` (Spanish), `jpn` (Japanese)
- **PaddleOCR**: Accepts flexible language codes and full language names
  - Examples: `en`, `ch`, `french`, `korean`, `thai`, `greek`, `cyrillic`, etc.
- **EasyOCR**: Similar flexible format to PaddleOCR

When used with `--ocr true`, the language flag overrides the default language. When used without `--ocr`, it overrides the language specified in your config file.

```bash title="Terminal"
# French OCR with Tesseract (default backend)
kreuzberg extract --ocr true --ocr-language fra document.pdf

# Chinese OCR with PaddleOCR
kreuzberg extract --ocr true --ocr-backend paddle-ocr --ocr-language ch document.pdf

# Thai OCR with PaddleOCR
kreuzberg extract --ocr true --ocr-backend paddle-ocr --ocr-language thai document.pdf

# German OCR with Tesseract
kreuzberg extract --ocr true --ocr-language deu document.pdf

# Override config file language with Spanish
kreuzberg extract document.pdf --config kreuzberg.toml --ocr-language spa
```

### OCR Configuration

OCR options are configured via config file. CLI flags override config settings:

```bash title="Terminal"
# Extract with OCR enabled via config file
kreuzberg extract scanned.pdf --config kreuzberg.toml --ocr true
```

Configure OCR backend, language, and Tesseract options in your config file (see Configuration Files section).

## Configuration Files

### Using Config Files

Kreuzberg automatically discovers a configuration file by searching the current directory and parent directories for **`kreuzberg.toml`** only. If you use YAML or JSON, specify the file explicitly with `--config`.

```bash title="Terminal"
# Extract using discovered configuration (finds kreuzberg.toml)
kreuzberg extract document.pdf
```

### Specify Config File

You can load TOML, YAML (`.yaml` or `.yml`), or JSON via `--config`:

```bash title="Terminal"
kreuzberg extract document.pdf --config my-config.toml
kreuzberg extract document.pdf --config kreuzberg.yaml
kreuzberg extract document.pdf --config my-config.json
```

### Inline JSON Config

Override or supply config without a file using inline JSON (merged after config file, before individual flags):

```bash title="Terminal"
# Inline JSON (applied after config file)
kreuzberg extract document.pdf --config-json '{"ocr":{"backend":"tesseract"},"chunking":{"max_chars":1000}}'

# Base64-encoded JSON (useful in shells where quoting is awkward)
kreuzberg extract document.pdf --config-json-base64 eyJvY3IiOnsiYmFja2VuZCI6InRlc3NlcmFjdCJ9fQ==
```

Both `extract` and `batch` support `--config-json` and `--config-json-base64`.

### Example Config Files

**kreuzberg.toml:**

```toml title="OCR configuration"
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"

[ocr.tesseract_config]
psm = 3

[chunking]
max_characters = 1000
overlap = 100
```

**kreuzberg.yaml:**

```yaml title="kreuzberg.yaml"
use_cache: true
enable_quality_processing: true

ocr:
  backend: tesseract
  language: eng
  tesseract_config:
    psm: 3

chunking:
  max_characters: 1000
  overlap: 100
```

**kreuzberg.json:**

```json title="kreuzberg.json"
{
  "use_cache": true,
  "enable_quality_processing": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng",
    "tesseract_config": {
      "psm": 3
    }
  },
  "chunking": {
    "max_characters": 1000,
    "overlap": 100
  }
}
```

## Batch Processing

Use the `batch` command to process multiple files:

```bash title="Terminal"
# Extract all PDFs in directory
kreuzberg batch documents/*.pdf

# Extract PDFs recursively from subdirectories
kreuzberg batch documents/**/*.pdf

# Extract multiple file types
kreuzberg batch documents/**/*.{pdf,docx,txt}
```

### Batch with Output Formats

```bash title="Terminal"
# Output as JSON (default for batch command)
kreuzberg batch documents/*.pdf --format json

# Output as plain text
kreuzberg batch documents/*.pdf --format text
```

### Batch with OCR

```bash title="Terminal"
# Batch extract with OCR enabled
kreuzberg batch scanned/*.pdf --ocr true

# Batch extract with force OCR
kreuzberg batch documents/*.pdf --force-ocr true

# Batch extract with quality processing
kreuzberg batch documents/*.pdf --quality true
```

### Batch with Content Format

```bash title="Terminal"
# Batch extract with djot formatting
kreuzberg batch documents/*.pdf --output-format djot --format json

# Batch extract as Markdown
kreuzberg batch documents/*.pdf --output-format markdown --format json

# Batch extract as HTML
kreuzberg batch documents/*.pdf --output-format html --format json
```

## Advanced Features

### Language Detection

```bash title="Terminal"
# Extract with automatic language detection
kreuzberg extract document.pdf --detect-language true

# Disable language detection
kreuzberg extract document.pdf --detect-language false
```

### Content Chunking

```bash title="Terminal"
# Split content into chunks for LLM processing
kreuzberg extract document.pdf --chunk true

# Specify chunk size and overlap
kreuzberg extract document.pdf --chunk true --chunk-size 1000 --chunk-overlap 100

# Output chunked content as JSON
kreuzberg extract document.pdf --chunk true --format json
```

### Quality Processing

```bash title="Terminal"
# Apply quality processing for improved formatting
kreuzberg extract document.pdf --quality true

# Disable quality processing
kreuzberg extract document.pdf --quality false

# Batch extraction with quality processing
kreuzberg batch documents/*.pdf --quality true
```

### Caching

```bash title="Terminal"
# Extract with result caching enabled (default)
kreuzberg extract document.pdf

# Extract without caching results
kreuzberg extract document.pdf --no-cache true

# Clear all cached results
kreuzberg cache clear

# View cache statistics
kreuzberg cache stats
```

## Extraction Override Flags <span class="version-badge">v4.5.2</span>

The `extract` and `batch` commands support a comprehensive set of flags to override extraction configuration. These flags take precedence over config file settings.

### OCR Flags

| Flag | Description |
|------|-------------|
| `--ocr <true\|false>` | Enable or disable OCR. Defaults to tesseract backend when enabled. |
| `--ocr-backend <BACKEND>` | OCR backend: `tesseract`, `paddle-ocr`, or `easyocr`. |
| `--ocr-language <LANG>` | OCR language code. Tesseract uses ISO 639-3 (`eng`, `fra`, `deu`). PaddleOCR/EasyOCR use short codes (`en`, `ch`, `korean`). |
| `--force-ocr <true\|false>` | Force OCR even if the document has an existing text layer. |
| `--ocr-auto-rotate <true\|false>` | Automatically rotate images before OCR based on detected orientation. |

```bash title="Terminal"
kreuzberg extract scanned.pdf --ocr true --ocr-backend paddle-ocr --ocr-language ch
kreuzberg extract document.pdf --force-ocr true --ocr-auto-rotate true
```

### Chunking Flags

| Flag | Description |
|------|-------------|
| `--chunk <true\|false>` | Enable or disable text chunking. |
| `--chunk-size <N>` | Maximum chunk size in characters (default: 1000). |
| `--chunk-overlap <N>` | Overlap between consecutive chunks in characters (default: 200). |
| `--chunking-tokenizer <MODEL>` | Tokenizer model for token-based chunk sizing (e.g. `Xenova/gpt-4o`). Implicitly enables chunking. Requires the `chunking-tokenizers` feature. |

```bash title="Terminal"
kreuzberg extract document.pdf --chunk true --chunk-size 512 --chunk-overlap 50
kreuzberg extract document.pdf --chunking-tokenizer "Xenova/gpt-4o"
```

### Output Flags

| Flag | Description |
|------|-------------|
| `--output-format <FORMAT>` | Content output format: `plain`, `markdown`, `djot`, or `html`. Controls how extracted text is formatted. |
| `--include-structure <true\|false>` | Include hierarchical document structure in results. |

```bash title="Terminal"
kreuzberg extract document.pdf --output-format markdown --include-structure true
```

### Layout Detection Flags

| Flag | Description |
|------|-------------|
| `--layout-preset <PRESET>` | Layout detection preset (e.g. `accurate`). Requires the `layout-detection` feature. |
| `--layout-confidence <FLOAT>` | Layout detection confidence threshold (0.0 - 1.0). |

```bash title="Terminal"
kreuzberg extract document.pdf --layout-preset accurate --layout-confidence 0.7
```

### Acceleration Flags

| Flag | Description |
|------|-------------|
| `--acceleration <PROVIDER>` | ONNX Runtime execution provider for model inference: `auto`, `cpu`, `coreml`, `cuda`, or `tensorrt`. |

```bash title="Terminal"
# Use CoreML on macOS for GPU acceleration
kreuzberg extract document.pdf --acceleration coreml

# Use CUDA on Linux with NVIDIA GPU
kreuzberg extract document.pdf --acceleration cuda
```

### Page Flags

| Flag | Description |
|------|-------------|
| `--extract-pages <true\|false>` | Extract pages as a separate array in results. |
| `--page-markers <true\|false>` | Insert page marker comments into the main content string. |

```bash title="Terminal"
kreuzberg extract document.pdf --extract-pages true --page-markers true --format json
```

### Image Flags

| Flag | Description |
|------|-------------|
| `--extract-images <true\|false>` | Enable image extraction from documents. |
| `--target-dpi <N>` | Target DPI for image normalisation (36 - 2400). |

```bash title="Terminal"
kreuzberg extract document.pdf --extract-images true --target-dpi 300
```

### PDF Flags

| Flag | Description |
|------|-------------|
| `--pdf-password <PASSWORD>` | Password for encrypted PDFs. Can be specified multiple times for multiple passwords. |
| `--pdf-extract-images <true\|false>` | Extract images embedded in PDF pages. Requires pdfium feature. |
| `--pdf-extract-metadata <true\|false>` | Extract PDF metadata (title, author, etc.). Requires pdfium feature. |

```bash title="Terminal"
kreuzberg extract encrypted.pdf --pdf-password "secret"
kreuzberg extract document.pdf --pdf-extract-images true --pdf-extract-metadata true
```

### Token Reduction Flags

| Flag | Description |
|------|-------------|
| `--token-reduction <LEVEL>` | Token reduction intensity: `off`, `light`, `moderate`, `aggressive`, or `maximum`. Reduces token count for LLM consumption. |

```bash title="Terminal"
# Aggressive token reduction for cheaper LLM processing
kreuzberg extract document.pdf --token-reduction aggressive

# Maximum compression (lossy)
kreuzberg extract document.pdf --token-reduction maximum
```

### Quality and Detection Flags

| Flag | Description |
|------|-------------|
| `--quality <true\|false>` | Enable quality post-processing for improved formatting. |
| `--detect-language <true\|false>` | Enable automatic language detection on extracted text. |

### Caching Flags

| Flag | Description |
|------|-------------|
| `--no-cache <true\|false>` | Disable extraction result caching. |

### Concurrency Flags

| Flag | Description |
|------|-------------|
| `--max-concurrent <N>` | Limit parallel extractions in batch mode. |
| `--max-threads <N>` | Cap all internal thread pools (Rayon, ONNX intra-op, batch semaphore). Useful for constrained environments. |

```bash title="Terminal"
kreuzberg batch documents/*.pdf --max-concurrent 4 --max-threads 8
```

### Email Flags

| Flag | Description |
|------|-------------|
| `--msg-codepage <N>` | Windows codepage fallback for MSG files without codepage metadata. Common values: 1250 (Central European), 1251 (Cyrillic), 1252 (Western). |

```bash title="Terminal"
kreuzberg extract message.msg --msg-codepage 1251
```

## Output Options

### Standard Output (Text Format)

```bash title="Terminal"
# Extract and print content to stdout
kreuzberg extract document.pdf

# Extract and redirect output to file
kreuzberg extract document.pdf > output.txt

# Batch extract as text
kreuzberg batch documents/*.pdf --format text
```

### JSON Output

```bash title="Terminal"
# Output as JSON
kreuzberg extract document.pdf --format json

# Batch extract as JSON (default format)
kreuzberg batch documents/*.pdf --format json
```

**JSON Output Structure:**

The JSON output includes extracted content and related metadata:

```json title="JSON Response"
{
  "content": "Extracted text content...",
  "metadata": {
    "mime_type": "application/pdf"
  }
}
```

## Error Handling

The CLI returns appropriate exit codes on error. Basic error handling can be done with standard shell commands:

```bash title="Terminal"
# Check for extraction errors
kreuzberg extract document.pdf || echo "Extraction failed"

# Continue processing even if one file fails (bash)
for file in documents/*.pdf; do
  kreuzberg batch "$file" || continue
done
```

## Examples

### Extract Single PDF

```bash title="Extract text from PDF"
kreuzberg extract document.pdf
```

### Batch Extract All PDFs in Directory

```bash title="Extract all PDFs from directory as JSON"
kreuzberg batch documents/*.pdf --format json
```

### OCR Scanned Documents

```bash title="OCR extraction from scanned documents"
kreuzberg batch scans/*.pdf --ocr true --format json
```

### Extract with Quality Processing

```bash title="Extract with quality processing enabled"
kreuzberg extract document.pdf --quality true --format json
```

### Extract with Chunking

```bash title="Extract with chunking for LLM processing"
kreuzberg extract document.pdf --config kreuzberg.toml --chunk true --chunk-size 1000 --chunk-overlap 100 --format json
```

### Batch Extract Multiple File Types

```bash title="Extract multiple file types in batch"
kreuzberg batch documents/**/*.{pdf,docx,txt} --format json
```

### Extract with Config File

```bash title="Extract using configuration file"
kreuzberg extract document.pdf --config /path/to/kreuzberg.toml
```

### Detect MIME Type

```bash title="Detect file MIME type"
kreuzberg detect document.pdf
```

## Docker Usage

Use the CLI image `ghcr.io/kreuzberg-dev/kreuzberg-cli:latest` for command-line usage. The full image `ghcr.io/kreuzberg-dev/kreuzberg:latest` also includes the CLI.

### Basic Docker

```bash title="Terminal"
# Extract document using Docker with mounted directory
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg-cli:latest \
  extract /data/document.pdf

# Extract and save output to host directory using shell redirection
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg-cli:latest \
  extract /data/document.pdf > output.txt
```

### Docker with OCR

```bash title="Terminal"
# Extract with OCR using Docker
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg-cli:latest \
  extract /data/scanned.pdf --ocr true
```

### Docker Compose

**docker-compose.yaml:**

```yaml title="docker-compose.yaml"
version: "3.8"

services:
  kreuzberg:
    image: ghcr.io/kreuzberg-dev/kreuzberg-cli:latest
    volumes:
      - ./documents:/input
    command: extract /input/document.pdf --ocr true
```

Run:

```bash title="Terminal"
docker-compose up
```

## Performance Tips

### Optimize Extraction Speed

```bash title="Terminal"
# Extract without quality processing for faster speed
kreuzberg extract large.pdf --quality false

# Use batch for processing multiple files
kreuzberg batch large_files/*.pdf --format json
```

### Manage Memory Usage

```bash title="Terminal"
# Disable caching to reduce memory footprint
kreuzberg extract large_file.pdf --no-cache true

# Compress output to save disk space
kreuzberg extract document.pdf | gzip > output.txt.gz
```

## Troubleshooting

### Check Installation

```bash title="Terminal"
# Display installed version
kreuzberg --version

# Display help for commands
kreuzberg --help
```

### Common Issues

**Issue: "Tesseract not found"**

When using OCR, Tesseract must be installed:

```bash title="Terminal"
# Install Tesseract OCR engine on macOS
brew install tesseract

# Install Tesseract OCR engine on Ubuntu
sudo apt-get install tesseract-ocr
```

**Issue: "File not found"**

Ensure the file path is correct and accessible:

```bash title="Terminal"
# Check if file exists and is readable
ls -la document.pdf

# Extract with absolute path
kreuzberg extract /absolute/path/to/document.pdf
```

## Server Commands

### Start API Server

The `serve` command starts a RESTful HTTP API server:

```bash title="Terminal"
# Start server on default host (127.0.0.1) and port (8000)
kreuzberg serve

# Start server on specific host and port (-H / -p are short forms)
kreuzberg serve --host 0.0.0.0 --port 8000
kreuzberg serve -H 0.0.0.0 -p 8000

# Start server with custom configuration file
kreuzberg serve --config kreuzberg.toml --host 0.0.0.0 --port 8000
```

### Server Endpoints

The server provides the following endpoints:

- `POST /extract` - Extract text from uploaded files
- `POST /batch` - Batch extract from multiple files
- `GET /detect` - Detect MIME type of file
- `GET /health` - Health check
- `GET /info` - Server information
- `GET /cache/stats` - Cache statistics
- `POST /cache/clear` - Clear cache

See [API Server Guide](../guides/api-server.md) for full API details.

### Start MCP Server

The `mcp` command starts a Model Context Protocol server for AI integration:

```bash title="Terminal"
# Start MCP server with stdio transport (default for Claude Desktop)
kreuzberg mcp

# Start MCP server with HTTP transport
kreuzberg mcp --transport http

# Start MCP server on specific HTTP host and port
kreuzberg mcp --transport http --host 0.0.0.0 --port 8001

# Start MCP server with custom configuration file
kreuzberg mcp --config kreuzberg.toml --transport stdio
```

The MCP server provides tools for AI agents:

- `extract_file` - Extract text from a file path
- `extract_bytes` - Extract text from base64-encoded bytes
- `batch_extract` - Extract from multiple files

See [API Server Guide](../guides/api-server.md) for MCP integration details.

## Embeddings <span class="version-badge">v4.5.2</span>

Generate vector embeddings for text using pre-trained models. Reads from `--text` flags or stdin.

```bash title="Terminal"
# Generate embeddings for a single text
kreuzberg embed --text "hello world" --preset balanced

# Generate embeddings with a specific preset
kreuzberg embed --text "document content" --preset fast

# Batch embed multiple texts
kreuzberg embed --text "first document" --text "second document" --preset quality

# Read from stdin
echo "hello world" | kreuzberg embed --preset balanced

# Output as text instead of JSON
kreuzberg embed --text "hello" --preset balanced --format text
```

Available presets: `fast`, `balanced` (default), `quality`, `multilingual`.

!!! info "Feature Availability"
    The `embed` command requires the `embeddings` feature. It is available in Docker images but not in Homebrew installations.

## Chunking Command <span class="version-badge">v4.5.2</span>

Split text into chunks using configurable size and overlap. Reads from `--text` flag or stdin.

```bash title="Terminal"
# Chunk text with default settings
kreuzberg chunk --text "long text content to be split into chunks..."

# Specify chunk size and overlap
kreuzberg chunk --text "long text..." --chunk-size 512 --chunk-overlap 50

# Use markdown-aware chunking
kreuzberg chunk --text "# Heading\n\nParagraph..." --chunker-type markdown

# Use a tokenizer model for token-based sizing
kreuzberg chunk --text "long text..." --chunking-tokenizer "Xenova/gpt-4o"

# Read from stdin
cat document.txt | kreuzberg chunk --chunk-size 1000

# Output as text instead of JSON
kreuzberg chunk --text "long text..." --format text

# Use a config file for chunking settings
kreuzberg chunk --text "long text..." --config kreuzberg.toml
```

## Shell Completions <span class="version-badge">v4.5.2</span>

Generate shell completion scripts for tab-completion support.

```bash title="Terminal"
# Generate bash completions
kreuzberg completions bash

# Generate zsh completions
kreuzberg completions zsh

# Generate fish completions
kreuzberg completions fish

# Install bash completions
eval "$(kreuzberg completions bash)"

# Install zsh completions (add to .zshrc)
eval "$(kreuzberg completions zsh)"
```

## API Utilities <span class="version-badge">v4.5.2</span>

### Dump OpenAPI Schema <span class="version-badge">v4.5.2</span>

Output the full OpenAPI 3.1 specification for the Kreuzberg REST API. Useful for code generation, documentation, and API client tooling.

```bash title="Terminal"
# Print OpenAPI schema as JSON
kreuzberg api schema

# Save to file
kreuzberg api schema > openapi.json
```

!!! info "Feature Availability"
    The `api` subcommand requires the `api` feature.

## List Supported Formats <span class="version-badge">v4.5.2</span>

List all document formats supported by Kreuzberg, including file extensions and MIME types.

```bash title="Terminal"
# List formats as a table
kreuzberg formats

# List formats as JSON
kreuzberg formats --format json
```

## Cache Management

### View Cache Statistics

```bash title="Terminal"
# Display cache usage statistics
kreuzberg cache stats

# Display statistics for specific cache directory
kreuzberg cache stats --cache-dir /path/to/cache

# Output cache statistics as JSON
kreuzberg cache stats --format json
```

### Clear Cache

```bash title="Terminal"
# Remove all cached extraction results
kreuzberg cache clear

# Clear specific cache directory
kreuzberg cache clear --cache-dir /path/to/cache

# Clear cache and display removal details
kreuzberg cache clear --format json
```

### Warm Model Cache <span class="version-badge">v4.5.0</span>

Pre-download all ML models (PaddleOCR and layout detection) so they are ready
for offline use. This is especially useful for containerized deployments.

```bash title="Terminal"
# Download all OCR and layout models eagerly
kreuzberg cache warm

# Download to a specific cache directory
kreuzberg cache warm --cache-dir /path/to/cache

# Also download all 4 embedding model presets (fast, balanced, quality, multilingual)
kreuzberg cache warm --all-embeddings

# Download a specific embedding model preset
kreuzberg cache warm --embedding-model balanced

# Output download results as JSON
kreuzberg cache warm --format json
```

### Model Manifest <span class="version-badge">v4.5.0</span>

Output a manifest of all expected model files with their SHA256 checksums and sizes.
Useful for verifying cache integrity or scripting model pre-population.

```bash title="Terminal"
# Output manifest as JSON (default)
kreuzberg cache manifest

# Output manifest as human-readable text
kreuzberg cache manifest --format text
```

## Getting Help

### CLI Help

```bash title="Terminal"
# Display general CLI help
kreuzberg --help

# Display command-specific help
kreuzberg extract --help
kreuzberg batch --help
kreuzberg detect --help
kreuzberg formats --help
kreuzberg version --help
kreuzberg embed --help
kreuzberg chunk --help
kreuzberg completions --help
kreuzberg serve --help
kreuzberg mcp --help
kreuzberg cache --help
kreuzberg cache stats --help
kreuzberg cache clear --help
kreuzberg cache warm --help
kreuzberg cache manifest --help
kreuzberg api schema --help
```

### Version Information

```bash title="Terminal"
# Display version number
kreuzberg --version

# Show version with JSON output
kreuzberg version --format json
```

The `version` command displays the Kreuzberg version. Use `--format json` for machine-readable output.

## Next Steps

- [API Server Guide](../guides/api-server.md) - API and MCP server setup
- [Advanced Features](../guides/advanced.md) - Advanced Kreuzberg features
- [Plugin Development](../guides/plugins.md) - Extend Kreuzberg functionality
- [API Reference](../reference/api-python.md) - Programmatic access
