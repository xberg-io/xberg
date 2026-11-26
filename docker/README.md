# Xberg Docker Images

This directory contains Dockerfile variants for building Xberg Docker images with different feature sets.

## Base Image

Both variants use **Debian 13 (Trixie) slim** - the latest stable Debian release for optimal package availability and security updates.

## Image Variants

### 1. Core Image (`Dockerfile.core`)

**Size:** ~239 MB compressed
**Base:** debian:trixie-slim
**Features:** PDF, DOCX, PPTX, images, HTML, XML, text, Excel, email, academic formats (LaTeX, EPUB, etc.)
**OCR:** Tesseract (12 languages)
**Legacy Office:** Native OLE/CFB parsing support

**When to use:**

- Production deployments where image size matters
- Cloud environments with size/bandwidth constraints
- Kubernetes deployments with frequent pod scaling
- All use cases (both images have equivalent legacy Office support)

**Build command:**

```bash
docker build -f docker/Dockerfile.core -t xberg:core .
```

### 2. Full Image (`Dockerfile.full`)

**Size:** ~1179 MB compressed (includes the pre-downloaded PaddleOCR and layout models)
**Base:** debian:trixie-slim
**Features:** All core features with native legacy Office format support
**OCR:** Tesseract (12 languages)
**Legacy Office:** Native OLE/CFB parsing for .doc, .ppt, .xls

**When to use:**

- Complete document intelligence pipeline with all optional dependencies
- Development and testing environments
- When you want maximum feature completeness

**Build command:**

```bash
docker build -f docker/Dockerfile.full -t xberg:full .
```

### 3. tessdata_best twins (`--target best`)

Both Dockerfiles have a `best` target: the same image plus the `tessdata_best` Tesseract models for
its OCR languages (the Debian set, `jpn_vert`, `kor`, and `osd`/`equ` for orientation and equation
detection) in the read-only directory `/usr/share/xberg/tessdata/best`, with
`XBERG_OCR_TESSDATA_MODEL=best`. The models are downloaded at the revision pinned in
`crates/xberg/src/ocr/tessdata_download/pins.rs` and checked against
`scripts/ci/tessdata/tessdata_best.sha256` (regenerate with
`python3 scripts/ci/tessdata/generate_best_manifest.py`). `TESSDATA_PREFIX` is left unset: it is
used as given for every model set and would replace the best models. Best OCR is more accurate,
several times slower per page and needs more memory per request. The default target (`fast`) is
the image without them.

```bash
docker build -f docker/Dockerfile.core --target best -t xberg:core-best .
docker build -f docker/Dockerfile.full --target best -t xberg:full-best .
```

## Size Comparison

Compressed (registry) sizes. The best figures add the ~124 MB of compressed `tessdata_best`
models to the measured fast images and are estimates until the first published build.

| Image     | Tag                             | Compressed size |
| --------- | ------------------------------- | --------------- |
| Core      | `VERSION-core`, `core`          | ~239 MB         |
| Full      | `VERSION`, `latest`             | ~1179 MB        |
| Core best | `VERSION-core-best`, `core-best` | ~363 MB (est.)  |
| Full best | `VERSION-best`, `latest-best`   | ~1303 MB (est.) |

## Default Image

The root `Dockerfile` is a symlink to `Dockerfile.full` for backward compatibility and complete feature support by default.

## GPU Support

All published images (Core, Full, CLI) run ONNX Runtime on CPU only — there is no published GPU/CUDA image. GPU acceleration exists in Xberg (the `cuda` Cargo feature) but requires building from source against a CUDA-enabled ONNX Runtime; the prebuilt `download-binaries` ORT these Dockerfiles use has no CUDA support. See the [GPU Acceleration docs](https://docs.xberg.io/getting-started/installation/#gpu-acceleration).

## Multi-Architecture Support

Both images support:

- `linux/amd64` (x86_64)
- `linux/arm64` (aarch64)

Both architectures use the same pure-Rust PDF library — no architecture-specific binaries needed.

## Usage Modes

All images support three execution modes via ENTRYPOINT:

### 1. API Server (default)

```bash
docker run -p 8000:8000 -e XBERG_API_TOKEN="$XBERG_API_TOKEN" xberg:core
# or override host/port:
docker run -p 8000:8000 -e XBERG_API_TOKEN="$XBERG_API_TOKEN" xberg:core serve --host 0.0.0.0 --port 8000
```

The images bind `0.0.0.0`, so the server exits at startup unless `XBERG_API_TOKEN` is set (clients send
`Authorization: Bearer <token>`; `GET /health` stays open). Set `XBERG_ALLOW_UNAUTHENTICATED=true`
instead only on a trusted network, for example behind a reverse proxy. The token must be ASCII.

### 2. CLI Mode

```bash
docker run -v $(pwd):/data xberg:core extract /data/document.pdf
docker run -v $(pwd):/data xberg:core detect /data/file.bin
docker run -v $(pwd):/data xberg:core batch /data/*.pdf
```

### 3. MCP Server Mode

```bash
docker run xberg:core mcp
```

## Testing

Test scripts are provided to verify both image variants:

```bash
# Test core image
IMAGE_NAME=xberg:core ./scripts/test_docker.sh

# Test full image
IMAGE_NAME=xberg:full ./scripts/test_docker.sh
```

## GitHub Actions

The `.github/workflows/publish-docker.yaml` workflow builds and publishes both variants to GitHub Container Registry:

- `ghcr.io/xberg-io/xberg:VERSION-core` - Core image (minimal runtime)
- `ghcr.io/xberg-io/xberg:core` - Latest core image
- `ghcr.io/xberg-io/xberg:VERSION` - Full image (all optional dependencies)
- `ghcr.io/xberg-io/xberg:latest` - Latest full image
- `ghcr.io/xberg-io/xberg:VERSION-core-best` / `core-best` - Core image with `tessdata_best`
- `ghcr.io/xberg-io/xberg:VERSION-best` / `latest-best` - Full image with `tessdata_best`

For local development, use the local tags shown in the build commands above.

## Recommendations

**Choose Core if:**

- ✅ Minimal runtime setup
- ✅ Standard document processing needs
- ✅ Cloud deployments with cost constraints
- ✅ Kubernetes or container orchestration

**Choose Full if:**

- ✅ Want maximum optional dependencies preinstalled
- ✅ Development and testing environments
- ✅ "Batteries included" experience preferred
