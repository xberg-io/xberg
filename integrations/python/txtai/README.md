<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# txtai-xberg

Feed [Xberg](https://github.com/xberg-io/xberg) document extraction into [txtai](https://github.com/neuml/txtai). `XbergPipeline` is a plain callable that extracts text and metadata from 107 formats — running OCR where needed — and flattens the result into documents ready for `txtai.Embeddings.index`. When you enable Xberg's native chunking, each chunk becomes one embedding-sized segment instead of a single blob.

## Install

```bash
pip install txtai-xberg
```

Requires Python 3.11+. Install the `txtai` extra (`pip install "txtai-xberg[txtai]"`) if txtai isn't already in your environment.

## Extract

Call the pipeline with a path or a list of paths. A string returns one document; a list returns documents in input order. Batches run through Xberg's `extract_batch` in a single native call.

```python
from txtai_xberg import XbergPipeline

pipeline = XbergPipeline()

doc = pipeline("report.pdf")
print(doc["content"])              # extracted markdown
print(doc["metadata"]["title"])    # source, mime_type, title, authors, languages, page_count

docs = pipeline(["report.pdf", "notes.docx"])
```

## Index into txtai

Use `to_documents` to get `(id, text, tags)` tuples and hand them straight to `Embeddings.index`. Enable Xberg chunking so segments arrive sized for the model, with heading and page context in each document's tags.

```python
from txtai import Embeddings
from txtai_xberg import XbergPipeline
from xberg import ChunkingConfig, ExtractionConfig

pipeline = XbergPipeline(
    config=ExtractionConfig(chunking=ChunkingConfig(max_characters=1000, overlap=200)),
)
documents = pipeline.to_documents(["report.pdf", "notes.docx"])

embeddings = Embeddings(path="sentence-transformers/all-MiniLM-L6-v2", content=True)
embeddings.index(documents)

for result in embeddings.search("quarterly revenue", 3):
    print(result["id"], result["text"])
```

Without a chunking config, `to_documents` emits one document per file. Chunk ids are `"<source>#<chunk_index>"`.

## Configure extraction

Pass any Xberg [`ExtractionConfig`](https://github.com/xberg-io/xberg) to control OCR, output format, chunking, and concurrency.

```python
from xberg import ExtractionConfig, OcrConfig

pipeline = XbergPipeline(
    config=ExtractionConfig(
        output_format="markdown",
        ocr=OcrConfig(language="eng"),
        force_ocr=True,
        max_concurrent_extractions=8,
    ),
)
```

## Async

Already inside an event loop? Use the async methods — `await pipeline.acall(paths)` and `await pipeline.ato_documents(paths)`. The synchronous `__call__` and `to_documents` wrap these with `asyncio.run` and must not run inside a running loop.

## Errors

Per-input failures in a batch surface as an `ExtractionFailedError`, whose `errors` attribute holds Xberg's `ExtractionErrorItem` objects (each with an `index`, `source`, `code`, and `message`).

For the full API, see the [Xberg documentation](https://docs.xberg.io).
