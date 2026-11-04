---
title: "CrewAI"
---

The `crewai-xberg` package gives [CrewAI](https://www.crewai.com/) agents document intelligence. It wraps Xberg's extraction pipeline as agent tools that pull text, metadata, keywords, entities, and summaries from 107 file formats, with OCR where needed.

[![PyPI](https://img.shields.io/pypi/v/crewai-xberg)](https://pypi.org/project/crewai-xberg/)
[![Python](https://img.shields.io/pypi/pyversions/crewai-xberg)](https://pypi.org/project/crewai-xberg/)
[![License](https://img.shields.io/pypi/l/crewai-xberg)](https://github.com/xberg-io/xberg/blob/main/integrations/python/crewai/LICENSE)

## Tools

The package exposes three CrewAI `BaseTool`s.

| Tool                       | Input        | Returns                                                     |
| -------------------------- | ------------ | ---------------------------------------------------------- |
| `XbergExtractTool`         | `file_path`  | Extracted text, plus any requested rich results.           |
| `XbergExtractBatchTool`    | `file_paths` | One section per document via `extract_batch`, then errors. |
| `XbergExtractMetadataTool` | `file_path`  | Title, authors, dates, page/table/image counts, format.    |

The batch tool routes every input through Xberg's `extract_batch` in a single native call — faster than looping single extractions, since concurrency happens Rust-side. Per-file failures land in a trailing `Errors` section instead of aborting the batch.

## Installation

```bash
pip install crewai-xberg
```

Requires Python 3.11+.

## Quick start

```python
from crewai import Agent
from crewai_xberg import XbergExtractTool, XbergExtractBatchTool, XbergExtractMetadataTool

agent = Agent(
    role="Document Analyst",
    goal="Extract and analyze document content",
    backstory="You process documents of any format.",
    tools=[XbergExtractTool(), XbergExtractBatchTool(), XbergExtractMetadataTool()],
)
```

## Extraction options

Both extraction tools accept the same options; each toggles an Xberg `ExtractionConfig` capability.

| Option             | Default      | Effect                                              |
| ------------------ | ------------ | --------------------------------------------------- |
| `output_format`    | `"markdown"` | `plain`, `markdown`, or `html`.                     |
| `force_ocr`        | `False`      | Run OCR on every page, even with a text layer.      |
| `chunk`            | `False`      | Split into semantic chunks; report the chunk count. |
| `extract_keywords` | `False`      | Append a keyword list.                              |
| `extract_entities` | `False`      | Append named entities (people, orgs, locations).    |
| `summarize`        | `False`      | Append a short summary.                             |

Detected languages and tables are surfaced automatically when present.

```python
tool = XbergExtractTool()

# Force OCR on a scan and surface keywords, entities, and a summary
enriched = tool.run(
    file_path="scan.pdf",
    force_ocr=True,
    extract_keywords=True,
    extract_entities=True,
    summarize=True,
)

# Many files in one batched extraction
combined = XbergExtractBatchTool().run(file_paths=["report.pdf", "notes.docx"])
```

The tools are async at the core. Inside an event loop, `await tool.arun(...)`; the synchronous `run` bridges to it and must not run inside a running loop.

For the full extraction API and configuration options, see the [Xberg documentation](https://docs.xberg.io) and the [format support list](/reference/formats/).
