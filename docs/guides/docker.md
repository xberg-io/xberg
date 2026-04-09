# Docker Deployment <span class="version-badge">v4.0.0</span>

Official Docker images built on the Rust core with Debian 13 (Trixie). Each image supports three execution modes: API server (default), CLI tool, and MCP server.

## Quick Start

### Pull and Run

=== "API Server"

    --8<-- "snippets/docker/api_server_basic.md"

=== "CLI Mode"

    --8<-- "snippets/docker/cli_mode_basic.md"

=== "MCP Server"

    --8<-- "snippets/docker/mcp_basic.md"

### Pull Image

=== "Core"

    --8<-- "snippets/docker/core_pull.md"

=== "Full"

    --8<-- "snippets/docker/full_pull.md"

## Image Variants

| | **Core** | **Full** |
|---|---|---|
| **Image** | `ghcr.io/kreuzberg-dev/kreuzberg:latest` | `ghcr.io/kreuzberg-dev/kreuzberg:latest` |
| **Size** | ~1.0–1.3 GB | ~1.5–2.1 GB |
| **Tesseract OCR** | 12 languages | 12 languages |
| **Modern Office** | DOCX, PPTX, XLSX | DOCX, PPTX, XLSX |
| **Legacy Office** | DOC, PPT, XLS (native OLE/CFB) | DOC, PPT, XLS (native OLE/CFB) |
| **Startup** | ~1s | ~1s |

**Core** is optimized for production deployments where image size matters. **Full** adds legacy format support for complete document intelligence pipelines.

Both images include: Tesseract OCR (eng, spa, fra, deu, ita, por, chi-sim, chi-tra, jpn, ara, rus, hin), pdfium, images, HTML, email, archives.

## Execution Modes

### API Server (Default)

```bash title="Terminal"
docker run -p 8000:8000 ghcr.io/kreuzberg-dev/kreuzberg:latest

# Custom port and CORS
docker run -p 9000:9000 \
  -e KREUZBERG_CORS_ORIGINS="https://myapp.com" \
  ghcr.io/kreuzberg-dev/kreuzberg:latest \
  serve --host 0.0.0.0 --port 9000

# With config file
docker run -p 8000:8000 \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  ghcr.io/kreuzberg-dev/kreuzberg:latest \
  serve --config /config/kreuzberg.toml
```

See [API Server Guide](api-server.md) for endpoint documentation.

### CLI Mode

```bash title="Terminal"
# Extract a file
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/document.pdf

# Extract with OCR
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/scanned.pdf --ocr true

# Batch processing
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  batch /data/*.pdf --format json

# MIME detection
docker run -v $(pwd):/data ghcr.io/kreuzberg-dev/kreuzberg:latest \
  detect /data/unknown-file.bin
```

### MCP Server

```bash title="Terminal"
docker run ghcr.io/kreuzberg-dev/kreuzberg:latest mcp

# With config
docker run \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  ghcr.io/kreuzberg-dev/kreuzberg:latest \
  mcp --config /config/kreuzberg.toml
```

See [API Server Guide - MCP Section](api-server.md#mcp-server) for integration details.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `KREUZBERG_MAX_UPLOAD_SIZE_MB` | `100` | Max upload size in MB |
| `KREUZBERG_CORS_ORIGINS` | `*` | Comma-separated allowed origins |
| `RUST_LOG` | `info` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `KREUZBERG_CACHE_DIR` | `/app/.kreuzberg` | Cache directory (set explicitly in Docker; outside containers defaults to platform global cache) |
| `HF_HOME` | `/app/.kreuzberg/huggingface` | HuggingFace model cache |

Host and port are set via CLI args: `serve --host 0.0.0.0 --port 8000`.

## Volume Mounts

```bash title="Terminal"
# Cache persistence (embedding models, OCR cache)
docker run -p 8000:8000 \
  -v kreuzberg-cache:/app/.kreuzberg \
  ghcr.io/kreuzberg-dev/kreuzberg:latest

# Config file
docker run -p 8000:8000 \
  -v $(pwd)/kreuzberg.toml:/config/kreuzberg.toml \
  ghcr.io/kreuzberg-dev/kreuzberg:latest \
  serve --config /config/kreuzberg.toml

# Documents (read-only)
docker run -v $(pwd)/documents:/data:ro \
  ghcr.io/kreuzberg-dev/kreuzberg:latest \
  extract /data/document.pdf
```

!!! Note "Model Downloads"
    Embedding models download on first use (~90 MB – 1.2 GB depending on preset). Use a persistent volume for `/app/.kreuzberg` in production to avoid re-downloading on container restart. Outside Docker, models are cached in the platform-specific global cache directory (for example, `~/.cache/kreuzberg/` on Linux, `~/Library/Caches/kreuzberg/` on macOS).

## Docker Compose

```yaml title="docker-compose.yaml"
services:
  kreuzberg-api:
    image: ghcr.io/kreuzberg-dev/kreuzberg:latest
    ports:
      - "8000:8000"
    environment:
      - KREUZBERG_CORS_ORIGINS=https://myapp.com
      - KREUZBERG_MAX_UPLOAD_SIZE_MB=500
      - RUST_LOG=info
    volumes:
      - ./config:/config
      - cache-data:/app/.kreuzberg
    command: serve --host 0.0.0.0 --port 8000 --config /config/kreuzberg.toml
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "kreuzberg", "--version"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 5s

volumes:
  cache-data:
```

## Security

Images run as non-root user `kreuzberg` (UID 1000). For hardened deployments:

```bash title="Terminal"
docker run --security-opt no-new-privileges \
  --read-only \
  --tmpfs /tmp \
  -p 8000:8000 \
  ghcr.io/kreuzberg-dev/kreuzberg:latest
```

Ensure mounted volumes have correct permissions:

```bash title="Terminal"
chown -R 1000:1000 /path/to/mounted/directory
```

## Resource Allocation

| Workload | Memory | CPU | Notes |
|----------|--------|-----|-------|
| Light | 512 MB | 0.5 cores | Small documents, low concurrency |
| Medium | 1 GB | 1 core | Typical documents, moderate concurrency |
| Heavy | 2 GB+ | 2+ cores | Large documents, OCR, high concurrency |

```bash title="Terminal"
docker run -p 8000:8000 --memory=1g --cpus=1 \
  ghcr.io/kreuzberg-dev/kreuzberg:latest
```

## Building Custom Images

=== "Core Image"

    --8<-- "snippets/docker/build_core.md"

=== "Full Image"

    --8<-- "snippets/docker/build_full.md"

```dockerfile title="Custom Dockerfile"
FROM ghcr.io/kreuzberg-dev/kreuzberg:latest

USER root
RUN apt-get update && \
    apt-get install -y --no-install-recommends your-package-here && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

USER kreuzberg
COPY kreuzberg.toml /app/kreuzberg.toml
CMD ["serve", "--config", "/app/kreuzberg.toml"]
```

## Other Image Variants

The published Core and Full images cover most use cases. For specialized needs, the `docker/` directory has additional Dockerfiles:

| Dockerfile | What it builds |
|------------|---------------|
| `Dockerfile.cli` | Minimal image with just the `kreuzberg` binary — good for CI pipelines and batch jobs |
| `Dockerfile.musl-build` | Fully static Linux binaries via MUSL — runs on any distro, no dynamic libs |
| `Dockerfile.musl-ffi` | Static C FFI library for language bindings (Go, Ruby, R, PHP, Elixir) |
| `Dockerfile.musl-rustler` | MUSL-based Rustler NIF for Elixir |

### CLI Image

A stripped-down image with only the CLI binary. No server, no API — just extraction:

```bash title="Terminal"
docker build -f docker/Dockerfile.cli -t kreuzberg-cli .

docker run -v $(pwd):/data kreuzberg-cli extract /data/document.pdf
docker run -v $(pwd):/data kreuzberg-cli batch /data/*.pdf --format json
docker run -v $(pwd):/data kreuzberg-cli detect /data/unknown-file.bin
```

### MUSL Static Builds

These produce binaries with zero dynamic library dependencies. A single file that runs on any Linux — Alpine, scratch containers, bare EC2 instances, whatever.

```bash title="Terminal"
docker build -f docker/Dockerfile.musl-build -t kreuzberg-musl-build .
docker build -f docker/Dockerfile.musl-ffi -t kreuzberg-musl-ffi .
```

The FFI variant builds a shared library used by the Go, Ruby, R, PHP, and Elixir bindings for portable cross-platform distribution.

## Troubleshooting

??? Question "Container won't start"

    Check logs with `docker logs <container-id>`. Common causes: port conflict (change `-p` mapping), insufficient memory (increase `--memory`), volume permission errors.

??? Question "Permission errors on mounted volumes"

    Images run as UID 1000. Fix with: `chown -R 1000:1000 /path/to/mounted/directory`

??? Question "Large file processing fails"

    Increase memory limit (`--memory=4g`) and upload size (`-e KREUZBERG_MAX_UPLOAD_SIZE_MB=1000`).

## Next Steps

- [Kubernetes Deployment](kubernetes.md) — production K8s with OCR config and health checks
- [API Server Guide](api-server.md) — endpoint documentation
- [Configuration](configuration.md) — all configuration options
