<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-dark.svg">
    <img alt="Xberg" width="420" src="https://cdn.jsdelivr.net/gh/xberg-io/assets@v1/banner/readme-banner-light.svg">
  </picture>
</p>

# crewai-xberg

[CrewAI](https://www.crewai.com/) tools backed by [Xberg](https://github.com/xberg-io/xberg). Give an agent document intelligence: extract text, metadata, keywords, entities, and summaries from 107 file formats — PDF, DOCX, XLSX, HTML, images with OCR, and more. Extraction is async at the core; the batch tool routes many files through Xberg's `extract_batch` in a single native call.

## Install

```bash
pip install crewai-xberg
```

Requires Python 3.11+.

## Tools

| Tool                       | Input        | Returns                                                        |
| -------------------------- | ------------ | ------------------------------------------------------------- |
| `XbergExtractTool`         | `file_path`  | Extracted text, plus any requested rich results.              |
| `XbergExtractBatchTool`    | `file_paths` | One section per document via `extract_batch`, then errors.    |
| `XbergExtractMetadataTool` | `file_path`  | Title, authors, dates, page/table/image counts, format info. |

## Use

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

Call a tool directly to see its output:

```python
tool = XbergExtractTool()

# Plain extraction
text = tool.run(file_path="report.pdf")

# Force OCR and surface keywords, entities, and a summary
enriched = tool.run(
    file_path="scan.pdf",
    output_format="markdown",
    force_ocr=True,
    extract_keywords=True,
    extract_entities=True,
    summarize=True,
)

# Many files in one batched extraction
combined = XbergExtractBatchTool().run(file_paths=["report.pdf", "notes.docx", "sheet.xlsx"])
```

## Options

Both extraction tools accept the same options. Each toggles an Xberg `ExtractionConfig` capability:

| Option             | Default      | Effect                                              |
| ------------------ | ------------ | --------------------------------------------------- |
| `output_format`    | `"markdown"` | `plain`, `markdown`, or `html`.                     |
| `force_ocr`        | `False`      | Run OCR on every page, even with a text layer.      |
| `chunk`            | `False`      | Split into semantic chunks; report the chunk count. |
| `extract_keywords` | `False`      | Append a keyword list.                              |
| `extract_entities` | `False`      | Append named entities (people, orgs, locations).    |
| `summarize`        | `False`      | Append a short summary.                             |

Detected languages and tables are surfaced automatically when present.

## Async

The tools are async at the core. Inside an event loop, `await tool.arun(...)`; the synchronous `run` bridges to it and must not be called from a running loop.

## Errors

A missing file raises from Xberg directly. In batch mode, per-file failures land in `ExtractionResult.errors` and are reported in a trailing `Errors` section instead of aborting the batch.

For the full API, see the [Xberg documentation](https://docs.xberg.io).
