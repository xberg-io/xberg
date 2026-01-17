# CLI Usage

The Kreuzberg CLI provides command-line access to all extraction features. This guide covers installation, basic usage, and advanced features.

## Installation

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

    - ✅ Text extraction (PDF, Office, images, 50+ formats)
    - ✅ OCR with Tesseract
    - ✅ HTTP API server (`serve` command)
    - ✅ MCP protocol server (`mcp` command)
    - ✅ Chunking, quality scoring, language detection
    - ❌ **Embeddings** - Not available via CLI flags. Use config file or Docker image.

    **Docker Images:**

    - All features enabled including embeddings (ONNX Runtime included)
    - Use `kreuzberg/kreuzberg:full` for LibreOffice support
    - Use `kreuzberg/kreuzberg:core` for smaller image without LibreOffice

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

### OCR Configuration

OCR options are configured via config file. CLI flags override config settings:

```bash title="Terminal"
# Extract with OCR enabled via config file
kreuzberg extract scanned.pdf --config kreuzberg.toml --ocr true
```

Configure OCR backend, language, and Tesseract options in your config file (see Configuration Files section).

## Configuration Files

### Using Config Files

Kreuzberg automatically discovers configuration files in this order:

1. Current directory: `./kreuzberg.{toml,yaml,yml,json}`
2. User config: `~/.config/kreuzberg/config.{toml,yaml,yml,json}`
3. System config: `/etc/kreuzberg/config.{toml,yaml,yml,json}`

```bash title="Terminal"
# Extract using discovered configuration
kreuzberg extract document.pdf
```

### Specify Config File

```bash title="Terminal"
kreuzberg extract document.pdf --config my-config.toml
```

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
max_chunk_size = 1000
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
  max_chunk_size: 1000
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
    "max_chunk_size": 1000,
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

### Basic Docker

```bash title="Terminal"
# Extract document using Docker with mounted directory
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/document.pdf

# Extract and save output to host directory using shell redirection
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/document.pdf > output.txt
```

### Docker with OCR

```bash title="Terminal"
# Extract with OCR using Docker
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/scanned.pdf --ocr true
```

### Docker Compose

**docker-compose.yaml:**

```yaml title="docker-compose.yaml"
version: '3.8'

services:
  kreuzberg:
    image: ghcr.io/kreuzberg-dev/kreuzberg:latest
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

# Start server on specific host and port
kreuzberg serve --host 0.0.0.0 --port 8000

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

## Getting Help

### CLI Help

```bash title="Terminal"
# Display general CLI help
kreuzberg --help

# Display command-specific help
kreuzberg extract --help
kreuzberg batch --help
kreuzberg detect --help
kreuzberg serve --help
kreuzberg mcp --help
kreuzberg cache --help
```

### Version Information

```bash title="Terminal"
# Display version number
kreuzberg --version
```

## Next Steps

- [API Server Guide](../guides/api-server.md) - API and MCP server setup
- [Advanced Features](../guides/advanced.md) - Advanced Kreuzberg features
- [Plugin Development](../guides/plugins.md) - Extend Kreuzberg functionality
- [API Reference](../reference/api-python.md) - Programmatic access
