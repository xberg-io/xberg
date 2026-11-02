<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# langchain-xberg

A [LangChain](https://www.langchain.com/) document loader backed by [Xberg](https://github.com/xberg-io/xberg). `XbergLoader` extracts text and metadata from 107 formats — running OCR where needed — and returns LangChain `Document` objects. Extraction is async at the core; multiple sources go through Xberg's `extract_batch` in a single native call, so concurrency happens Rust-side.

## Install

```bash
pip install langchain-xberg
```

Requires Python 3.11+.

## Load

Pass a path, a list of paths, a directory, or raw bytes. One source becomes one `Document`.

```python
from langchain_xberg import XbergLoader

# Single file
docs = XbergLoader(file_path="report.pdf").load()
print(docs[0].page_content)           # extracted markdown
print(docs[0].metadata["title"])      # source, mime_type, title, authors, detected_languages, page_count, ...

# Multiple files — one batched extraction
docs = XbergLoader(file_path=["report.pdf", "notes.docx"]).load()

# A directory with a glob
docs = XbergLoader(file_path="./corpus/", glob="**/*.pdf").load()

# Raw bytes (mime_type required)
docs = XbergLoader(data=raw_bytes, mime_type="application/pdf").load()
```

## Chunk for retrieval

Enable Xberg's native chunking to emit one `Document` per chunk, sized for embedding. Each chunk carries `chunk_index`, `total_chunks`, `heading_path`, `page`, and `token_count` in its metadata.

```python
from langchain_xberg import XbergLoader
from xberg import ChunkingConfig, ExtractionConfig

config = ExtractionConfig(chunking=ChunkingConfig(max_characters=1000, overlap=200))
docs = XbergLoader(file_path="report.pdf", config=config).load()  # one Document per chunk
```

To split by page instead, pass `pages=PageConfig(extract_pages=True)`; each `Document` then gets a 0-indexed `page`.

## Configure extraction

Pass any Xberg [`ExtractionConfig`](https://docs.xberg.io) to control OCR, output format, and batch concurrency.

```python
from xberg import ExtractionConfig, OcrConfig

config = ExtractionConfig(
    output_format="markdown",
    ocr=OcrConfig(backend="tesseract"),
    force_ocr=True,
    max_concurrent_extractions=8,
)
docs = XbergLoader(file_path="./corpus/", config=config).load()
```

## Async

Inside an event loop, use the async API — `await loader.aload()` or `async for doc in loader.alazy_load()`. These use Xberg's native async extraction end to end. The synchronous `load` / `lazy_load` bridge to it and must not run inside a running loop.

```python
loader = XbergLoader(file_path="report.pdf")
docs = await loader.aload()
```

## Errors

A per-input failure raises `xberg.XbergError` with the offending source and message. In batch mode the failure comes from `ExtractionResult.errors`; a single load surfaces the raised exception directly.

For the full API, see the [Xberg documentation](https://docs.xberg.io).
</content>
</invoke>
