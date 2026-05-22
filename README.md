# kreuzberg-crewai

<div align="center" style="display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; margin: 20px 0;">
  <a href="https://pypi.org/project/kreuzberg-crewai/"><img src="https://img.shields.io/pypi/v/kreuzberg-crewai?label=kreuzberg-crewai&color=007ec6" alt="PyPI version"></a>
  <a href="https://pypi.org/project/kreuzberg-crewai/"><img src="https://img.shields.io/pypi/pyversions/kreuzberg-crewai?color=007ec6" alt="Python versions"></a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg-crewai/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License"></a>
  <a href="https://docs.kreuzberg.dev"><img src="https://img.shields.io/badge/docs-kreuzberg.dev-blue" alt="Docs"></a>
  <a href="https://github.com/kreuzberg-dev/kreuzberg-crewai/actions/workflows/ci.yaml"><img src="https://github.com/kreuzberg-dev/kreuzberg-crewai/actions/workflows/ci.yaml/badge.svg" alt="CI"></a>
</div>

<img width="3384" height="573" alt="Kreuzberg Banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div align="center" style="margin-top: 20px;">
  <a href="https://discord.gg/xt9WY3GnKR">
    <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
  </a>
</div>

[Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) document extraction tools for [CrewAI](https://www.crewai.com/) agents.

Extract text and metadata from 90+ document formats — PDF, DOCX, XLSX, HTML, images with OCR, and more — directly from your CrewAI agents.

## Installation

```bash
pip install kreuzberg-crewai
```

## Quick Start

```python
from crewai import Agent, Crew, Task

from kreuzberg_crewai import KreuzbergExtractTool

tool = KreuzbergExtractTool()

agent = Agent(
    role="Document Analyst",
    goal="Extract and analyze document content",
    backstory="You are an expert at reading and understanding documents.",
    tools=[tool],
)

task = Task(
    description="Extract the content from report.pdf and summarize the key findings.",
    expected_output="A summary of the key findings in the report.",
    agent=agent,
)

crew = Crew(agents=[agent], tasks=[task])
result = crew.kickoff()
```

## Tools

### KreuzbergExtractTool

Extracts text content from a document file.

**Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `file_path` | `str` | required | Path to the document file |
| `output_format` | `"plain" \| "markdown" \| "html"` | `"markdown"` | Output format |

```python
from kreuzberg_crewai import KreuzbergExtractTool

tool = KreuzbergExtractTool()

# The agent calls this automatically, but you can also call it directly:
content = tool._run(file_path="report.pdf", output_format="markdown")
```

### KreuzbergExtractMetadataTool

Extracts metadata (title, authors, dates, page count, format-specific details) from a document file.

**Parameters:**

| Parameter | Type | Default | Description |
|---|---|---|---|
| `file_path` | `str` | required | Path to the document file |

```python
from kreuzberg_crewai import KreuzbergExtractMetadataTool

tool = KreuzbergExtractMetadataTool()

metadata = tool._run(file_path="report.pdf")
# title: Annual Report 2025
# authors: ['John Doe']
# page_count: 42
# pdf_version: 1.7
```

## Agent Example

Using both tools together:

```python
from crewai import Agent, Crew, Task

from kreuzberg_crewai import KreuzbergExtractMetadataTool, KreuzbergExtractTool

extract_tool = KreuzbergExtractTool()
metadata_tool = KreuzbergExtractMetadataTool()

agent = Agent(
    role="Research Assistant",
    goal="Read documents and extract useful information",
    backstory="You help researchers by reading and analyzing documents.",
    tools=[extract_tool, metadata_tool],
)

task = Task(
    description=(
        "First, check the metadata of research-paper.pdf to find the authors and date. "
        "Then extract the full content in markdown format and list the key conclusions."
    ),
    expected_output="Authors, date, and key conclusions from the paper.",
    agent=agent,
)

crew = Crew(agents=[agent], tasks=[task])
result = crew.kickoff()
```

## Supported Formats

Kreuzberg supports 90+ file formats:

- **Documents:** PDF, DOCX, DOC, XLSX, XLS, PPTX, PPT, ODT, ODS, ODP, RTF, and more
- **Text/Markup:** TXT, MD, HTML, XML, JSON, YAML, LaTeX, Jupyter notebooks
- **Images (OCR):** PNG, JPEG, TIFF, GIF, BMP, WEBP, SVG
- **Email:** EML, MSG (with attachment extraction)
- **eBooks:** EPUB
- **Archives:** ZIP, RAR, 7Z, TAR, GZIP
- **Data:** CSV, DBF

## Development

```bash
# Install dependencies
uv sync

# Run tests
uv run pytest

# Run linting
uv run ruff check src/ tests/
uv run ruff format --check src/ tests/

# Run type checking
uv run mypy src/
```

## License

MIT
