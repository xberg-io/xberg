# Build stage
FROM ghcr.io/astral-sh/uv:python3.13-bookworm AS builder
ARG EXTRAS=""
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV UV_LINK_MODE=copy
ENV UV_CACHE_DIR=/tmp/uv-cache

# Copy dependency files
COPY pyproject.toml uv.lock README.md ./
COPY kreuzberg kreuzberg

# Install dependencies with optimizations
RUN --mount=type=cache,target=/tmp/uv-cache \
    uv sync --extra api${EXTRAS:+ --extra ${EXTRAS}} --no-editable --no-dev --compile-bytecode && \
    rm -rf /app/.venv/lib/python*/site-packages/**/__pycache__ && \
    find /app/.venv -type f -name "*.pyc" -delete && \
    find /app/.venv -type d -name "tests" -exec rm -rf {} + 2>/dev/null || true && \
    find /app/.venv -type d -name "test" -exec rm -rf {} + 2>/dev/null || true

# Runtime stage
FROM ghcr.io/astral-sh/uv:python3.13-bookworm-slim AS runtime
ARG EXTRAS=""
WORKDIR /app
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV PATH="/app/.venv/bin:$PATH"

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pandoc \
    tesseract-ocr \
    tesseract-ocr-eng \
    tesseract-ocr-osd \
    libglib2.0-0 \
    libsm6 \
    libxext6 \
    libxrender-dev \
    libgomp1 \
    libgl1 \
    libglib2.0-0 \
    && apt-get clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Copy virtual environment from builder
COPY --from=builder /app/.venv /app/.venv
COPY --from=builder /app/kreuzberg /app/kreuzberg

# Create non-root user
RUN groupadd -r appuser && useradd -r -g appuser -d /app -s /sbin/nologin appuser && \
    chown -R appuser:appuser /app

USER appuser
CMD ["litestar", "--app", "kreuzberg._api.main:app", "run", "--host", "0.0.0.0"]
