---
title: "txtai"
---

The `txtai-xberg` package feeds Xberg's document extraction into [txtai](https://github.com/neuml/txtai). `XbergPipeline` extracts text and metadata from 107 formats — running OCR where needed — and flattens the result into documents ready for `txtai.Embeddings.index`. With Xberg's native chunking enabled, each chunk becomes one embedding-sized segment instead of a single blob.

[![PyPI](https://img.shields.io/pypi/v/txtai-xberg)](https://pypi.org/project/txtai-xberg)
[![Python](https://img.shields.io/pypi/pyversions/txtai-xberg)](https://pypi.org/project/txtai-xberg)
[![License](https://img.shields.io/pypi/l/txtai-xberg)](https://github.com/xberg-io/xberg/blob/main/integrations/python/txtai/LICENSE)

## How it works

```mermaid
flowchart LR
    Input[Documents] --> Xberg[Xberg Extraction]
    Xberg --> Chunk[Native Chunking]
    Chunk --> Docs["(id, text, tags)"]
    Docs --> Index[txtai Embeddings.index]
    Index --> Search[Semantic Search]

    style Xberg fill:#87CEEB
    style Chunk fill:#FFD700
    style Search fill:#90EE90
```

1. **Extract** — Xberg parses the source documents and runs OCR where needed.
2. **Chunk** — When the `ExtractionConfig` enables chunking, Xberg splits each document into segments and preserves heading and page context.
3. **Flatten** — `to_documents` emits `(id, text, tags)` tuples: one per chunk, or one per file when chunking is off.
4. **Index** — Hand the tuples straight to `txtai.Embeddings.index` for keyword, vector, or hybrid search.

## Key capabilities

- **Batch extraction** — `extract_batch` fans inputs across Xberg's worker pool in one native call, faster than looping.
- **Native chunking** — `ChunkingConfig` produces embedding-sized segments with `chunk_index`, `total_chunks`, `heading_path`, and page numbers surfaced in each document's tags.
- **Metadata passthrough** — Source, MIME type, title, authors, languages, and page count travel with every document.
- **Extraction control** — Pass any Xberg `ExtractionConfig` to set OCR behavior, output format, and concurrency.
- **Sync or async** — Callable methods (`__call__`, `to_documents`) wrap the async ones (`acall`, `ato_documents`) for use inside an event loop.

## Installation

```bash
pip install txtai-xberg
```

Requires Python 3.11+. Install the `txtai` extra if txtai isn't already present:

```bash
pip install "txtai-xberg[txtai]"
```

## Quick start

Extract and index chunked documents into a txtai embeddings database:

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

Chunk ids are `"<source>#<chunk_index>"`. Without a chunking config, `to_documents` emits one document per file.

## Extraction only

Call the pipeline directly to get extracted content and metadata without indexing. A string returns one document; a list returns documents in input order.

```python
from txtai_xberg import XbergPipeline

pipeline = XbergPipeline()

doc = pipeline("report.pdf")
print(doc["content"])            # extracted markdown
print(doc["metadata"]["title"])  # source, mime_type, title, authors, languages, page_count
```

## Choosing a method

|             | `to_documents` / `ato_documents` | `__call__` / `acall`             |
| ----------- | -------------------------------- | -------------------------------- |
| Returns     | `(id, text, tags)` tuples        | `{content, metadata}` dicts      |
| Granularity | One per chunk (or per file)      | One per file                     |
| Best for    | Indexing into `Embeddings`       | Reading extracted text directly  |

Per-input failures in a batch raise `ExtractionFailedError`, whose `errors` attribute holds Xberg's `ExtractionErrorItem` objects. For the complete extraction API and configuration options, see the [Xberg documentation](https://docs.xberg.io). For general txtai usage, see the [txtai docs](https://neuml.github.io/txtai/).
