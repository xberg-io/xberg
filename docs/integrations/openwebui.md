# Open WebUI

![Kreuzberg](https://img.shields.io/badge/kreuzberg-v4.7.0+-blue)

Open WebUI supports pluggable content extraction backends. Kreuzberg implements two of those backend APIs — the **docling-serve** endpoint and the **external document loader** endpoint, so it works as a drop-in replacement without patching Open WebUI.

## How it works

1. A user uploads a document (PDF, DOCX, image, etc.) in Open WebUI.
2. Open WebUI sends the file to Kreuzberg's API endpoint.
3. Kreuzberg extracts the content — running OCR where needed and returns markdown.
4. Open WebUI stores the markdown in its vector database for retrieval-augmented generation.

Kreuzberg supports [90+ file formats](../reference/formats.md) and requires no GPU.

## Prerequisites

- Docker and Docker Compose (v2)
- Open WebUI running or ready to deploy
- No GPU required — Kreuzberg runs entirely on CPU

## Setup with Docker Compose

This is the fastest way to get both services running together.

```yaml title="docker-compose.yaml"
services:
  kreuzberg:
    image: ghcr.io/kreuzberg-dev/kreuzberg:latest-core
    ports:
      - "8000:8000"
    command: ["serve", "--host", "0.0.0.0", "--port", "8000"]
    volumes:
      - kreuzberg-cache:/app/.kreuzberg
    healthcheck:
      test: ["CMD", "kreuzberg", "version"]
      interval: 10s
      timeout: 5s
      retries: 5

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    ports:
      - "3000:8080"
    environment:
      CONTENT_EXTRACTION_ENGINE: "docling"
      DOCLING_SERVER_URL: "http://kreuzberg:8000"
    depends_on:
      kreuzberg:
        condition: service_healthy

volumes:
  kreuzberg-cache:
```

Start both services in detached mode:

```bash
docker compose up -d
```

Open `http://localhost:3000`, create an account, and upload a document. The extracted text will appear in the chat context.

!!! note "Cache volume"
    The `kreuzberg-cache` volume persists OCR models and embedding weights across restarts. Without it, models re-download on every container restart (~90 MB–1.2 GB depending on configuration).

!!! info "Already running Open WebUI?"
    Start Kreuzberg separately, then point Open WebUI to that Kreuzberg URL.

=== "Docker"

    ```bash
    docker run -d \
      --name kreuzberg \
      -p 8000:8000 \
      -v kreuzberg-cache:/app/.kreuzberg \
      ghcr.io/kreuzberg-dev/kreuzberg:latest-core \
      serve --host 0.0.0.0 --port 8000
    ```

=== "CLI (Homebrew / Cargo)"

    ```bash
    kreuzberg serve --host 0.0.0.0 --port 8000
    ```

Then configure Open WebUI using one of the two engine modes below.

## Choosing an engine mode

Kreuzberg exposes two Open WebUI–compatible APIs. Both return the same extracted content. So pick whichever fits your setup.

| | **Docling** (recommended) | **External** |
|---|---|---|
| **Endpoint** | `POST /v1/convert/file` | `PUT /process` |
| **Engine setting** | `docling` | `external` |
| **URL variable** | `DOCLING_SERVER_URL` | `EXTERNAL_DOCUMENT_LOADER_URL` |

=== "Docling (recommended)"

    Set these environment variables on the Open WebUI container:

    ```yaml
    environment:
      CONTENT_EXTRACTION_ENGINE: "docling"
      DOCLING_SERVER_URL: "http://kreuzberg:8000"
    ```

    Or via the Admin UI: **Settings → Documents → Content Extraction Engine** → select **Docling** → set server URL to `http://kreuzberg:8000`.

=== "External"

    Set these environment variables on the Open WebUI container:

    ```yaml
    environment:
      CONTENT_EXTRACTION_ENGINE: "external"
      EXTERNAL_DOCUMENT_LOADER_URL: "http://kreuzberg:8000"
    ```

    Or via the Admin UI: **Settings → Documents → Content Extraction Engine** → select **External** → set URL to `http://kreuzberg:8000`.

!!! tip
    If Kreuzberg runs on a different host or port, replace `http://kreuzberg:8000` with the actual address. Inside Docker Compose, use the service name (`kreuzberg`). Outside Docker, use the host IP or `localhost`.

## Verify it works

Test the endpoints directly before debugging through Open WebUI.

=== "Docling endpoint"

    ```bash
    curl -s -F "files=@invoice.pdf" http://localhost:8000/v1/convert/file | jq .
    ```

    ```json title="Expected response"
    {
      "document": {
        "md_content": "# Invoice\n\nDate: 2026-01-15\n..."
      },
      "status": "success"
    }
    ```

=== "External endpoint"

    ```bash
    curl -s -X PUT \
      -H "Content-Type: application/pdf" \
      -H "X-Filename: invoice.pdf" \
      --data-binary @invoice.pdf \
      http://localhost:8000/process | jq .
    ```

    ```json title="Expected response"
    {
      "page_content": "# Invoice\n\nDate: 2026-01-15\n...",
      "metadata": {
        "source": "invoice.pdf"
      }
    }
    ```

If the endpoint returns extracted text, the integration is working. Upload a document through Open WebUI to confirm end-to-end.

## Next steps

- [Docker deployment guide](../guides/docker.md) — image variants, volumes, security hardening
- [API server reference](../guides/api-server.md) — all endpoints and configuration options
- [OCR guide](../guides/ocr.md) — language packs, engine selection, tuning
- [Format support](../reference/formats.md) — full list of supported file types
