from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Generator

TEST_SOURCE_FILES = Path(__file__).parent.parent.parent / "tests" / "test_source_files"
BENCHMARK_DIR = Path(__file__).parent / "results"
BENCHMARK_DIR.mkdir(parents=True, exist_ok=True)

SMALL_DOC_SIZE = 100 * 1024
MEDIUM_DOC_SIZE = 1024 * 1024
LARGE_DOC_SIZE = 10 * 1024 * 1024


@pytest.fixture
def disable_gc() -> Generator[None]:
    import gc

    was_enabled = gc.isenabled()
    gc.disable()
    try:
        yield
    finally:
        if was_enabled:
            gc.enable()


@pytest.fixture
def sample_markdown_small() -> bytes:
    content = """---
title: Small Test Document
author: Benchmark Author
date: 2025-09-27
---

# Introduction

This is a small document for benchmarking purposes.

"""
    while len(content) < SMALL_DOC_SIZE:
        content += """
## Section {i}

This is paragraph text that contains some content for extraction.
It includes multiple sentences to make it more realistic.

- List item 1
- List item 2
- List item 3

```python
def example_function():
    return "example"
```

""".replace("{i}", str(len(content)))

    return content[:SMALL_DOC_SIZE].encode()


@pytest.fixture
def sample_markdown_medium() -> bytes:
    content = """---
title: Medium Test Document
author: Benchmark Author
date: 2025-09-27
abstract: This is a medium-sized document for performance testing
keywords: [benchmark, performance, testing]
---

# Introduction

This is a medium document for benchmarking purposes.

"""
    section_num = 0
    while len(content) < MEDIUM_DOC_SIZE:
        section_num += 1
        content += f"""
## Section {section_num}

This section contains substantial content for realistic benchmarking.
The text includes various formatting elements and structures.

### Subsection {section_num}.1

Here we have a detailed explanation of various concepts that might appear
in a real document. This helps ensure our benchmarks are representative
of actual usage patterns.

| Column A | Column B | Column C |
|----------|----------|----------|
| Data 1   | Data 2   | Data 3   |
| Data 4   | Data 5   | Data 6   |

### Subsection {section_num}.2

Additional content with code examples:

```python
class BenchmarkExample:
    def __init__(self):
        self.data = []

    def process(self, item):
        self.data.append(item)
        return len(self.data)
```

And some mathematical expressions: $E = mc^2$

"""

    return content[:MEDIUM_DOC_SIZE].encode()


@pytest.fixture
def sample_markdown_large() -> bytes:
    content = """---
title: Large Test Document
author: Benchmark Author
date: 2025-09-27
abstract: |
  This is a large document designed for stress testing and performance benchmarking.
  It contains extensive content to simulate real-world document processing scenarios.
keywords: [benchmark, performance, stress-test, large-document]
---

# Introduction

This is a large document for comprehensive benchmarking.

"""
    chapter = 0
    while len(content) < LARGE_DOC_SIZE:
        chapter += 1
        content += f"""
# Chapter {chapter}

This chapter contains extensive documentation and analysis.

"""
        for section in range(10):
            content += f"""
## Section {chapter}.{section}

This section includes comprehensive information about topic {chapter}.{section}.
The content is designed to be representative of real documentation.

### Technical Details

Here we provide detailed technical information that might be found in
API documentation, technical specifications, or research papers.

```python
# Example code block
def process_data(input_data):
    # Process input data with complex transformations
    result = []
    for item in input_data:
        processed = transform(item)
        validated = validate(processed)
        result.append(validated)
    return result
```

### Data Analysis

| Metric       | Value    | Unit | Status  |
|--------------|----------|------|---------|
| Performance  | 95.5     | %    | Good    |
| Memory Usage | 1024     | MB   | Normal  |
| CPU Usage    | 45.2     | %    | Optimal |
| Disk I/O     | 120.5    | MB/s | Fast    |

### Mathematical Formulations

The system performance can be modeled using the following equation:

$$P(t) = P_0 e^{{-\\lambda t}} + \\sum_{{i=1}}^n A_i \\sin(\\omega_i t + \\phi_i)$$

Where:
- $P(t)$ represents performance at time $t$
- $P_0$ is the initial performance
- $\\lambda$ is the decay constant
- $A_i$, $\\omega_i$, and $\\phi_i$ are amplitude, frequency, and phase parameters

"""

        if len(content) >= LARGE_DOC_SIZE:
            break

    return content[:LARGE_DOC_SIZE].encode()


@pytest.fixture
def benchmark_results_dir() -> Path:
    return BENCHMARK_DIR


@pytest.fixture
def test_source_files_dir() -> Path:
    return TEST_SOURCE_FILES


@pytest.fixture
def test_pdfs_dir() -> Path:
    return TEST_SOURCE_FILES / "pdfs"


@pytest.fixture
def test_documents_dir() -> Path:
    return TEST_SOURCE_FILES / "documents"


@pytest.fixture
def test_images_dir() -> Path:
    return TEST_SOURCE_FILES / "images"


@pytest.fixture
def sample_pdf_path(test_pdfs_dir: Path) -> Path:
    pdf_files = list(test_pdfs_dir.glob("*.pdf"))
    if not pdf_files:
        raise FileNotFoundError(f"No PDF files found in {test_pdfs_dir}")
    return pdf_files[0]


@pytest.fixture
def sample_docx_path(test_documents_dir: Path) -> Path:
    docx_files = list(test_documents_dir.glob("*.docx"))
    if not docx_files:
        raise FileNotFoundError(f"No DOCX files found in {test_documents_dir}")
    return docx_files[0]


def pytest_configure(config: Any) -> None:
    config.addinivalue_line("markers", "benchmark_pandoc: mark test as pandoc benchmark")
    config.addinivalue_line("markers", "memory_profile: mark test for memory profiling")
