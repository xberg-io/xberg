# Kreuzberg Docker Images

[![GitHub](https://img.shields.io/badge/GitHub-Goldziher%2Fkreuzberg-blue)](https://github.com/Goldziher/kreuzberg)
[![PyPI](https://badge.fury.io/py/kreuzberg.svg)](https://badge.fury.io/py/kreuzberg)
[![Documentation](https://img.shields.io/badge/docs-kreuzberg.dev-blue)](https://kreuzberg.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://github.com/Goldziher/kreuzberg/blob/main/LICENSE)

High-performance Python library for text extraction from documents, available as optimized Docker images.

**Source Code**: [github.com/Goldziher/kreuzberg](https://github.com/Goldziher/kreuzberg)

## Quick Start

```bash
docker run -p 8000:8000 goldziher/kreuzberg:latest
```

## Available Images

### Base Image (`latest`)

- **Image**: `goldziher/kreuzberg:latest`
- **Size**: ~550MB compressed
- **Includes**: REST API server, CLI tools, Tesseract OCR with 12 business languages
- **Languages**: English, Spanish, French, German, Italian, Portuguese, Chinese (Simplified & Traditional), Japanese, Arabic, Russian, Hindi
- **Use cases**: Basic document processing, simple API deployments, cost-conscious workflows

### Core Image (`core`)

- **Image**: `goldziher/kreuzberg-core:latest`
- **Size**: ~700MB compressed
- **Includes**: Everything from base plus:
    - Text chunking (semantic-text-splitter)
    - Encrypted PDF support (crypto)
    - Document classification
    - Language detection
    - Email parsing (.eml, .msg)
    - Additional format extensions
- **Use cases**: RAG applications, document intelligence, enterprise workflows, multi-language processing

## Usage

### Extract Files via API

```bash
# Single file with base image
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf"

# With core image - chunking for RAG
docker run -p 8000:8000 goldziher/kreuzberg-core:latest
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf" \
  -F "chunk_content=true" \
  -F "max_chars=1000"

# Language detection
curl -X POST http://localhost:8000/extract \
  -F "data=@document.pdf" \
  -F "auto_detect_language=true"

# Encrypted PDF
curl -X POST http://localhost:8000/extract \
  -F "data=@encrypted.pdf" \
  -F "password=secretpassword"
```

### Docker Compose

```yaml
version: '3.8'

services:
  kreuzberg:
    image: goldziher/kreuzberg-core:latest
    ports:
      - "8000:8000"
    volumes:
      - kreuzberg-cache:/app/.kreuzberg
    environment:
      - PYTHONUNBUFFERED=1
      - KREUZBERG_CACHE_DIR=/app/.kreuzberg
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  kreuzberg-cache:
```

## Custom Images

Create tailored images for your specific needs:

### Example: RAG-Optimized

```dockerfile
FROM goldziher/kreuzberg:latest

USER root

# Add chunking and language detection
RUN pip install --upgrade "kreuzberg[chunking,langdetect]"

USER appuser

# RAG-optimized defaults
ENV KREUZBERG_CHUNK_CONTENT=true
ENV KREUZBERG_MAX_CHARS=1000
ENV KREUZBERG_AUTO_DETECT_LANGUAGE=true
```

### Example: Crypto Support

```dockerfile
FROM goldziher/kreuzberg:latest

USER root

# Add encrypted PDF support
RUN pip install --upgrade "kreuzberg[crypto]"

USER appuser
```

## Configuration

### Environment Variables

- `KREUZBERG_CACHE_DIR`: Cache directory (default: `/app/.kreuzberg`)
- `KREUZBERG_CHUNK_CONTENT`: Enable chunking (`true`/`false`)
- `KREUZBERG_AUTO_DETECT_LANGUAGE`: Enable language detection (`true`/`false`)
- `KREUZBERG_OCR_BACKEND`: OCR backend (`tesseract` or `none`)

### Configuration File

Mount `kreuzberg.toml`:

```toml
chunk_content = true
auto_detect_language = true
max_chars = 1000
ocr_backend = "tesseract"

[tesseract]
language = "eng+spa+fra+deu"
psm = 6
```

```bash
docker run -p 8000:8000 \
  -v "$(pwd)/kreuzberg.toml:/app/kreuzberg.toml:ro" \
  goldziher/kreuzberg-core:latest
```

## Features

- **🚀 High Performance**: Optimized for speed and efficiency
- **📄 Multiple Formats**: PDF, DOCX, images, HTML, and more
- **🔍 OCR Support**: Built-in Tesseract with 12 business languages
- **🔒 Secure**: Runs as non-root user, no external API calls
- **📦 Ready to Use**: Pre-configured API server

## Documentation

- **[GitHub Repository](https://github.com/Goldziher/kreuzberg)** - Source code and issue tracking
- **[Full Documentation](https://kreuzberg.dev/)** - Complete user guide and API reference
- **[API Documentation](https://kreuzberg.dev/user-guide/api-server/)** - REST API endpoints and usage
- **[Docker Guide](https://kreuzberg.dev/user-guide/docker/)** - Detailed Docker usage guide

## Support

- **Issues**: [github.com/Goldziher/kreuzberg/issues](https://github.com/Goldziher/kreuzberg/issues)
- **Discussions**: [github.com/Goldziher/kreuzberg/discussions](https://github.com/Goldziher/kreuzberg/discussions)
- **Discord**: [Join our community](https://discord.gg/pXxagNK2zN)

## Contributing

Contributions are welcome! See our [Contributing Guide](https://github.com/Goldziher/kreuzberg/blob/main/docs/contributing.md).

## License

MIT License - see [LICENSE](https://github.com/Goldziher/kreuzberg/blob/main/LICENSE) for details.

______________________________________________________________________

Made with ❤️ by the [Kreuzberg contributors](https://github.com/Goldziher/kreuzberg/graphs/contributors)
