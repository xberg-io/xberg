# Docker Deployment

Official Docker images built on the Rust core with Debian 13 (Trixie). Each image supports three execution modes: API server (default), command-line tool, and MCP server.

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

|                   | **Core**                               | **Full**                                 |
| ----------------- | -------------------------------------- | ---------------------------------------- |
| **Image**         | `ghcr.io/xberg-io/xberg:core` | `ghcr.io/xberg-io/xberg:latest` |
| **Size**          | ~1.0–1.3 GB                            | ~1.5–2.1 GB                              |
| **Tesseract OCR** | 12 languages                           | 12 languages                             |
| **Modern Office** | DOCX, PPTX, XLSX                       | DOCX, PPTX, XLSX                         |
| **Legacy Office** | DOC, PPT, XLS (native OLE/CFB)         | DOC, PPT, XLS (native OLE/CFB)           |
| **Startup**       | ~1s                                    | ~1s                                      |

**Core** is optimized for production deployments where image size matters. Both images support all major formats — choose based on deployment constraints.

All images include: Tesseract OCR (eng, spa, fra, deu, ita, por, chi-sim, chi-tra, jpn, ara, rus, hin), PDF (pdf_oxide), images, HTML, email, and archives.

## Execution Modes

### API Server (Default)

```bash title="Terminal"
docker run -p 8000:8000 ghcr.io/xberg-io/xberg:latest

# Custom port and CORS
docker run -p 9000:9000 \
  -e XBERG_CORS_ORIGINS="https://myapp.com" \
  ghcr.io/xberg-io/xberg:latest \
  serve --host 0.0.0.0 --port 9000

# With config file
docker run -p 8000:8000 \
  -v $(pwd)/xberg.toml:/config/xberg.toml \
  ghcr.io/xberg-io/xberg:latest \
  serve --config /config/xberg.toml
```

See [API Server Guide](api-server.md) for endpoint documentation.

### CLI Mode

```bash title="Terminal"
# Extract a file
docker run -v $(pwd):/data ghcr.io/xberg-io/xberg:latest \
  extract /data/document.pdf

# Extract with OCR
docker run -v $(pwd):/data ghcr.io/xberg-io/xberg:latest \
  extract /data/scanned.pdf --ocr true

# Batch processing
docker run -v $(pwd):/data ghcr.io/xberg-io/xberg:latest \
  batch /data/*.pdf --format json

# MIME detection
docker run -v $(pwd):/data ghcr.io/xberg-io/xberg:latest \
  detect /data/unknown-file.bin
```

### MCP Server

```bash title="Terminal"
docker run ghcr.io/xberg-io/xberg:latest mcp

# With config
docker run \
  -v $(pwd)/xberg.toml:/config/xberg.toml \
  ghcr.io/xberg-io/xberg:latest \
  mcp --config /config/xberg.toml
```

See [API Server Guide - MCP Section](api-server.md#mcp-server) for integration details.

## Environment Variables

| Variable                       | Default                       | Description                                                                                      |
| ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------ |
| `XBERG_MAX_UPLOAD_SIZE_MB` | `100`                         | Max upload size in MB                                                                            |
| `XBERG_CORS_ORIGINS`       | `*`                           | Comma-separated allowed origins                                                                  |
| `RUST_LOG`                     | `info`                        | Log level: `error`, `warn`, `info`, `debug`, `trace`                                             |
| `XBERG_CACHE_DIR`          | `/app/.xberg`             | Cache directory (set explicitly in Docker; outside containers defaults to platform global cache) |
| `HF_HOME`                      | `/app/.xberg/huggingface` | HuggingFace model cache                                                                          |

Host and port are set via CLI args: `serve --host 0.0.0.0 --port 8000`.

## Volume Mounts

```bash title="Terminal"
# Cache persistence (embedding models, OCR cache)
docker run -p 8000:8000 \
  -v xberg-cache:/app/.xberg \
  ghcr.io/xberg-io/xberg:latest

# Config file
docker run -p 8000:8000 \
  -v $(pwd)/xberg.toml:/config/xberg.toml \
  ghcr.io/xberg-io/xberg:latest \
  serve --config /config/xberg.toml

# Documents (read-only)
docker run -v $(pwd)/documents:/data:ro \
  ghcr.io/xberg-io/xberg:latest \
  extract /data/document.pdf
```

!!! Note "Model Downloads" Embedding models download on first use (~90 MB – 1.2 GB depending on preset). Use a persistent volume for `/app/.xberg` in production to avoid re-downloading on container restart. Outside Docker, models are cached in the platform-specific global cache directory (for example, `~/.cache/xberg/` on Linux, `~/Library/Caches/xberg/` on macOS).

## Docker Compose

```yaml title="docker-compose.yaml"
services:
  xberg-api:
    image: ghcr.io/xberg-io/xberg:latest
    ports:
      - "8000:8000"
    environment:
      - XBERG_CORS_ORIGINS=https://myapp.com
      - XBERG_MAX_UPLOAD_SIZE_MB=500
      - RUST_LOG=info
    volumes:
      - ./config:/config
      - cache-data:/app/.xberg
    command: serve --host 0.0.0.0 --port 8000 --config /config/xberg.toml
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "xberg", "--version"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 5s

volumes:
  cache-data:
```

## Security

Images run as non-root user `xberg` (UID 1000). For hardened deployments:

```bash title="Terminal"
docker run --security-opt no-new-privileges \
  --read-only \
  --tmpfs /tmp \
  -p 8000:8000 \
  ghcr.io/xberg-io/xberg:latest
```

Ensure mounted volumes have correct permissions:

```bash title="Terminal"
chown -R 1000:1000 /path/to/mounted/directory
```

## Resource Allocation

| Workload | Memory | CPU       | Notes                                   |
| -------- | ------ | --------- | --------------------------------------- |
| Light    | 512 MB | 0.5 cores | Small documents, low concurrency        |
| Medium   | 1 GB   | 1 core    | Typical documents, moderate concurrency |
| Heavy    | 2 GB+  | 2+ cores  | Large documents, OCR, high concurrency  |

```bash title="Terminal"
docker run -p 8000:8000 --memory=1g --cpus=1 \
  ghcr.io/xberg-io/xberg:latest
```

## Building Custom Images

=== "Core Image"

    --8<-- "snippets/docker/build_core.md"

=== "Full Image"

    --8<-- "snippets/docker/build_full.md"

```dockerfile title="Custom Dockerfile"
FROM ghcr.io/xberg-io/xberg:latest

USER root
RUN apt-get update && \
    apt-get install -y --no-install-recommends your-package-here && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

USER xberg
COPY xberg.toml /app/xberg.toml
CMD ["serve", "--config", "/app/xberg.toml"]
```

## Other Image Variants

The published Core and Full images cover most use cases. For specialized needs, the `docker/` directory has additional Dockerfiles:

| Dockerfile                | What it builds                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------- |
| `Dockerfile.cli`          | Minimal image with just the `xberg` binary — good for CI pipelines and batch jobs |
| `Dockerfile.musl-build`   | Fully static Linux binaries via MUSL — runs on any distro, no dynamic libs            |
| `Dockerfile.musl-ffi`     | Static C FFI library for language bindings (Go, Ruby, R, PHP, Elixir)                 |
| `Dockerfile.musl-rustler` | MUSL-based Rustler NIF for Elixir                                                     |

### CLI Image

A stripped-down image with only the CLI binary. No server, no API — just extraction:

```bash title="Terminal"
docker build -f docker/Dockerfile.cli -t xberg-cli .

docker run -v $(pwd):/data xberg-cli extract /data/document.pdf
docker run -v $(pwd):/data xberg-cli batch /data/*.pdf --format json
docker run -v $(pwd):/data xberg-cli detect /data/unknown-file.bin
```

### MUSL Static Builds

These produce binaries with zero dynamic library dependencies. A single file that runs on any Linux — Alpine, scratch containers, bare EC2 instances, whatever.

```bash title="Terminal"
docker build -f docker/Dockerfile.musl-build -t xberg-musl-build .
docker build -f docker/Dockerfile.musl-ffi -t xberg-musl-ffi .
```

The FFI variant builds a shared library used by the Go, Ruby, R, PHP, and Elixir bindings for portable cross-platform distribution.

## Troubleshooting

??? Question "Container won't start"

    Check logs with `docker logs <container-id>`. Common causes: port conflict (change `-p` mapping), insufficient memory (increase `--memory`), volume permission errors.

??? Question "Permission errors on mounted volumes"

    Images run as UID 1000. Fix with: `chown -R 1000:1000 /path/to/mounted/directory`

??? Question "Large file processing fails"

    Increase memory limit (`--memory=4g`) and upload size (`-e XBERG_MAX_UPLOAD_SIZE_MB=1000`).

## Next Steps

- [API Server Guide](api-server.md) — endpoint documentation
- [Configuration](configuration.md) — all configuration options
