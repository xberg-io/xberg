# LlamaIndex Node Parser Xberg

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://pypi.org/project/llama-index-node-parser-xberg/">
    <img src="https://img.shields.io/pypi/v/llama-index-node-parser-xberg?label=Node%20Parser&color=007ec6" alt="Node Parser">
  </a>
  <a href="https://pypi.org/project/xberg/">
    <img src="https://img.shields.io/pypi/v/xberg?label=Xberg&color=007ec6" alt="Xberg">
  </a>
  <a href="https://github.com/xberg-io/llama-index-xberg/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License">
  </a>
  <a href="https://docs.xberg.io">
    <img src="https://img.shields.io/badge/docs-xberg.io-blue" alt="Documentation">
  </a>
</div>

<img width="3384" height="573" alt="Xberg Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

Element-aware LlamaIndex node parser for xberg-extracted documents.

## Installation

```bash
pip install llama-index-node-parser-xberg
```

Requires `llama-index-core>=0.13.0,<0.15`. This package does not depend on
`xberg` directly — the `xberg` package is a dependency of the reader
(`llama-index-readers-xberg`), which is needed for producing documents with
element metadata.

## Prerequisites

> **This parser requires documents with `_xberg_elements` metadata.**
> These are produced by `XbergReader` configured with element-based
> extraction. Install `llama-index-readers-xberg` (which brings in
> `xberg`) to use the full workflow.

```python
from xberg import ExtractionConfig
from llama_index.readers.xberg import XbergReader

reader = XbergReader(
    extraction_config=ExtractionConfig(result_format="element_based")
)
documents = reader.load_data("report.pdf")
```

## Features

- Element-aware splitting — headings, paragraphs, tables, and code blocks each become a node
- Element type metadata preserved on each node (`element_type`, `page_number`, `element_index`)
- Source document relationships tracked via `NodeRelationship.SOURCE`
- Graceful degradation — documents without elements pass through with a warning
- Composes with other transformations (e.g., `SentenceSplitter` for further chunking)
- Async support via `aget_nodes_from_documents`
- Serialization support (`to_dict` / `from_dict`)

## Usage

### Basic

Full reader-to-nodes flow:

```python
from xberg import ExtractionConfig
from llama_index.readers.xberg import XbergReader
from llama_index.node_parser.xberg import XbergNodeParser

reader = XbergReader(
    extraction_config=ExtractionConfig(result_format="element_based")
)
documents = reader.load_data("report.pdf")

parser = XbergNodeParser()
nodes = parser.get_nodes_from_documents(documents)
```

### IngestionPipeline

Chain with `SentenceSplitter` for further chunking of large elements:

```python
from llama_index.core.ingestion import IngestionPipeline
from llama_index.core.node_parser import SentenceSplitter

pipeline = IngestionPipeline(
    transformations=[
        XbergNodeParser(),
        SentenceSplitter(chunk_size=512),  # Further split large elements
    ]
)
nodes = pipeline.run(documents=documents)
```

### VectorStoreIndex

Using the `transformations` parameter:

```python
from llama_index.core import VectorStoreIndex

index = VectorStoreIndex.from_documents(
    documents,
    transformations=[XbergNodeParser()],
)
```

### Async

```python
nodes = await parser.aget_nodes_from_documents(documents)
```

## Behavior Notes

- Documents without `_xberg_elements` metadata pass through unchanged with
  a warning. This is intentional — silently falling back would prevent users
  from noticing they are not getting element-aware splitting.
- Empty or whitespace-only elements are automatically skipped.
