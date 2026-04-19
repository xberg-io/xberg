# API Server <span class="version-badge">v4.0.0</span>

Kreuzberg runs as an HTTP REST API server (`kreuzberg serve`) or as an MCP server (`kreuzberg mcp`) for AI agent integration.

## HTTP REST API

### Start

=== "CLI"

    --8<-- "snippets/api_server/cli.md"

=== "Docker"

    --8<-- "snippets/api_server/docker.md"

=== "Python"

    --8<-- "snippets/api_server/python.md"

=== "Rust"

    --8<-- "snippets/api_server/rust.md"

=== "Go"

    --8<-- "snippets/api_server/go.md"

=== "Java"

    --8<-- "snippets/api_server/java.md"

=== "C#"

    --8<-- "snippets/api_server/csharp.md"

### Endpoints

#### POST /extract

Extract text from uploaded files via multipart form data.

| Field | Required | Description |
|-------|----------|-------------|
| `files` | Yes (repeatable) | Files to extract |
| `config` | No | JSON config overrides |
| `output_format` | No | `plain` (default), `markdown`, `djot`, or `html` |

```bash title="Terminal"
# Single file
curl -F "files=@document.pdf" http://localhost:8000/extract

# Multiple files
curl -F "files=@doc1.pdf" -F "files=@doc2.docx" http://localhost:8000/extract

# With config overrides
curl -F "files=@scanned.pdf" \
     -F 'config={"ocr":{"language":"eng"},"force_ocr":true}' \
     http://localhost:8000/extract
```

```json title="Response"
[
  {
    "content": "Extracted text...",
    "mime_type": "application/pdf",
    "metadata": { "page_count": 10, "author": "John Doe" },
    "tables": [],
    "detected_languages": ["eng"],
    "chunks": null,
    "images": null
  }
]
```

#### POST /embed

Generate vector embeddings. Requires the `embeddings` feature.

| Field | Required | Description |
|-------|----------|-------------|
| `texts` | Yes | Array of strings |
| `config` | No | Embedding config overrides |

```bash title="Terminal"
curl -X POST http://localhost:8000/embed \
  -H "Content-Type: application/json" \
  -d '{"texts":["Hello world","Second text"]}'
```

| Preset | Dimensions | Model |
|--------|-----------|-------|
| `fast` | 384 | AllMiniLML6V2Q |
| `balanced` (default) | 768 | BGEBaseENV15 |
| `quality` | 1024 | BGELargeENV15 |
| `multilingual` | 768 | MultilingualE5Base |

#### POST /chunk

Chunk text for RAG pipelines.

| Field | Required | Description |
|-------|----------|-------------|
| `text` | Yes | Text to chunk |
| `chunker_type` | No | `"text"` (default), `"markdown"`, `"yaml"`, or `"semantic"` |
| `config.max_characters` | No | Max chars per chunk (default: 2000) |
| `config.overlap` | No | Overlap between chunks (default: 100) |

```bash title="Terminal"
curl -X POST http://localhost:8000/chunk \
  -H "Content-Type: application/json" \
  -d '{"text":"Long text...","chunker_type":"text","config":{"max_characters":1000,"overlap":50}}'
```

=== "Python"

    --8<-- "snippets/python/api/client_chunk_text.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/client_chunk_text.md"

=== "Rust"

    --8<-- "snippets/rust/api/client_chunk_text.md"

=== "Go"

    --8<-- "snippets/go/api/client_chunk_text.md"

=== "Java"

    --8<-- "snippets/java/api/client_chunk_text.md"

=== "C#"

    --8<-- "snippets/csharp/client_chunk_text.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/client_chunk_text.md"

#### POST /extract-structured <span class="version-badge">v4.8.0</span>

Extract typed JSON from a document by running an LLM against the extracted text with a JSON schema. Requires the server to be built with the `liter-llm` feature; otherwise the endpoint returns `501 Not Implemented`.

The request is `multipart/form-data`.

| Field | Required | Description |
|-------|----------|-------------|
| `file` (or `files`) | Yes | The document to extract from |
| `schema` | Yes | JSON Schema string describing the structured output |
| `model` | Yes | LLM model identifier, for example `openai/gpt-4o` or `anthropic/claude-sonnet-4-20250514` |
| `api_key` | No | LLM provider API key. Falls back to provider env vars (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, ...) |
| `prompt` | No | Custom Jinja2 prompt template overriding the default |
| `schema_name` | No | Schema identifier (default: `extraction`) |
| `strict` | No | `"true"` / `"false"` — enable OpenAI strict mode for exact schema matching |
| `config` | No | Extraction config overrides as a JSON string |

```bash title="Terminal"
curl -X POST http://localhost:8000/extract-structured \
  -F "file=@invoice.pdf" \
  -F 'schema={"type":"object","properties":{"invoice_number":{"type":"string"},"total":{"type":"number"}},"required":["invoice_number","total"]}' \
  -F "model=openai/gpt-4o" \
  -F "api_key=$OPENAI_API_KEY" \
  -F "strict=true"
```

```json title="Response"
{
  "structured_output": {
    "invoice_number": "INV-2026-0142",
    "total": 1284.50
  },
  "content": "Invoice INV-2026-0142...",
  "mime_type": "application/pdf"
}
```

Errors follow the same shape as `/extract`. A `501` body indicates the server was built without the `liter-llm` feature; rebuild with `--features liter-llm` to enable structured extraction.

#### Other Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | `{"status":"healthy","version":"4.6.3"}` |
| `/version` | GET | `{"version":"4.6.3"}` <span class="version-badge">v4.5.2</span> |
| `/detect` | POST | MIME type detection (multipart) <span class="version-badge">v4.5.2</span> |
| `/cache/stats` | GET | Cache statistics |
| `/cache/warm` | POST | Pre-download models <span class="version-badge">v4.5.2</span> |
| `/cache/manifest` | GET | Model manifest with checksums <span class="version-badge">v4.5.2</span> |
| `/cache/clear` | DELETE | Clear all cached files |
| `/info` | GET | `{"version":"...","rust_backend":true}` |
| `/openapi.json` | GET | OpenAPI 3.0 schema |

### Client Examples

=== "Python"

    --8<-- "snippets/python/api/client_extract_single_file.md"

=== "TypeScript"

    --8<-- "snippets/typescript/getting-started/client_extract_single_file.md"

=== "Rust"

    --8<-- "snippets/rust/api/client_extract_single_file.md"

=== "Go"

    --8<-- "snippets/go/api/client_extract_single_file.md"

=== "Java"

    --8<-- "snippets/java/api/client_extract_single_file.md"

=== "C#"

    --8<-- "snippets/csharp/client_extract_single_file.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/client_extract_single_file.md"

### Error Handling

```json title="Error response"
{
  "error_type": "ValidationError",
  "message": "Invalid file format",
  "status_code": 400
}
```

| Status | Error type | Meaning |
|--------|-----------|---------|
| 400 | `ValidationError` | Invalid input |
| 422 | `ParsingError`, `OcrError` | Processing failed |
| 500 | Internal errors | Server errors |

=== "Python"

    --8<-- "snippets/python/utils/error_handling_extract.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/error_handling_extract.md"

=== "Rust"

    --8<-- "snippets/rust/api/error_handling_extract.md"

=== "Go"

    --8<-- "snippets/go/api/error_handling_extract.md"

=== "Java"

    --8<-- "snippets/java/api/error_handling_extract.md"

=== "C#"

    --8<-- "snippets/csharp/error_handling_extract.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/error_handling_extract.md"

### Configuration

The server discovers `kreuzberg.toml` in the current and parent directories. Pass `--config path/to/file` to use a different file.

| Variable | Default | Description |
|----------|---------|-------------|
| `KREUZBERG_MAX_UPLOAD_SIZE_MB` | `100` | Max upload size in MB |
| `KREUZBERG_CORS_ORIGINS` | `*` | Comma-separated allowed origins |

!!! Warning
    Default CORS allows all origins. Set `KREUZBERG_CORS_ORIGINS` explicitly in production.

See [Configuration Guide](configuration.md) for all options.

---

## MCP Server

### Start

```bash title="Terminal"
kreuzberg mcp
kreuzberg mcp --config kreuzberg.toml
```

=== "Python"

    --8<-- "snippets/python/mcp/mcp_server_start.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp/mcp_server_start.md"

=== "Rust"

    --8<-- "snippets/rust/mcp/mcp_server_start.md"

=== "Go"

    --8<-- "snippets/go/mcp/mcp_server_start.md"

=== "Java"

    --8<-- "snippets/java/mcp/mcp_server_start.md"

=== "C#"

    --8<-- "snippets/csharp/mcp_server_start.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp/mcp_server_start.md"

### Tools

| Tool | Key parameters | Description |
|------|----------------|-------------|
| `extract_file` | `path` | Extract from file path |
| `extract_bytes` | `data` (base64) | Extract from encoded bytes |
| `batch_extract_files` | `paths` | Extract multiple files |
| `detect_mime_type` | `path` | Detect file format |
| `list_formats` | — | List supported formats <span class="version-badge">v4.5.2</span> |
| `get_version` | — | Library version <span class="version-badge">v4.5.2</span> |
| `cache_stats` | — | Cache usage |
| `cache_clear` | — | Remove cached files |
| `cache_manifest` | — | Model checksums <span class="version-badge">v4.5.2</span> |
| `cache_warm` | — | Pre-download models <span class="version-badge">v4.5.2</span> |
| `embed_text` | `texts` | Generate embeddings <span class="version-badge">v4.5.2</span> |
| `chunk_text` | `text` | Split text <span class="version-badge">v4.5.2</span> |
| `extract_structured` | `path`, `schema`, `model`; optional `schema_name` (default `"extraction"`), `schema_description`, `prompt`, `api_key`, `strict` (default `false`) | Extract structured JSON via LLM <span class="version-badge">v4.8.0</span> |

All tools accept an optional `config` object. `extract_file` and `extract_bytes` also accept `pdf_password`. `extract_structured` requires the server to be built with the `liter-llm` feature; see the row above for optional fields and defaults.

### AI Agent Integration

=== "Claude Desktop"

    Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

    ```json
    {
      "mcpServers": {
        "kreuzberg": {
          "command": "kreuzberg",
          "args": ["mcp"]
        }
      }
    }
    ```

=== "Python"

    --8<-- "snippets/python/mcp/mcp_custom_client.md"

=== "LangChain"

    --8<-- "snippets/python/mcp/mcp_langchain_integration.md"

=== "TypeScript"

    --8<-- "snippets/typescript/mcp/mcp_custom_client.md"

=== "Rust"

    --8<-- "snippets/rust/mcp/mcp_custom_client.md"

=== "Go"

    --8<-- "snippets/go/mcp/mcp_custom_client.md"

=== "Java"

    --8<-- "snippets/java/mcp/mcp_client.md"

=== "C#"

    --8<-- "snippets/csharp/mcp_custom_client.md"

=== "Ruby"

    --8<-- "snippets/ruby/mcp/mcp_custom_client.md"

---

For Docker and Kubernetes deployment, see [Docker Guide](docker.md) and [Kubernetes Guide](kubernetes.md).
