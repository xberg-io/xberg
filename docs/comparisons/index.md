# Comparisons

Kreuzberg sits in a crowded space of document extraction tools. Some are general-purpose libraries that handle dozens of formats, others are laser-focused on PDFs. This page maps the landscape so you can find the right tool for your project.

For performance and quality numbers across all of these tools, see the [live benchmarks](https://kreuzberg.dev/benchmarks).

---

## Full-Scope Extraction Libraries

These handle multiple document formats -- not just PDFs.

| Library                                               | Language | Formats        | License      | Focus                                                            | Deep Dive                                 |
| ----------------------------------------------------- | -------- | -------------- | ------------ | ---------------------------------------------------------------- | ----------------------------------------- |
| **Kreuzberg**                                         | Rust     | 90+            | Elastic-2.0  | High-throughput extraction with native bindings for 17 languages | --                                        |
| [Unstructured](https://unstructured.io)               | Python   | ~31            | Apache-2.0   | Element-based output, managed cloud API                          | [Read more](kreuzberg-vs-unstructured.md) |
| [Docling](https://github.com/docling-project/docling) | Python   | ~38            | MIT          | IBM-backed, ML-powered layout analysis                           | [Read more](kreuzberg-vs-docling.md)      |
| [Apache Tika](https://tika.apache.org)                | Java     | 1500+ detected | Apache-2.0   | Enterprise standard, broadest format detection                   | [Read more](kreuzberg-vs-tika.md)         |
| [MarkItDown](https://github.com/microsoft/markitdown) | Python   | ~25            | MIT          | Microsoft-backed, outputs Markdown for LLM prep                  | [Read more](kreuzberg-vs-markitdown.md)   |
| [MinerU](https://github.com/opendatalab/MinerU)       | Python   | PDF + images   | **AGPL-3.0** | Heavy ML models for scientific document layout                   | [Read more](kreuzberg-vs-mineru.md)       |
| [Pandoc](https://pandoc.org)                          | Haskell  | 45+ input      | **GPL-2.0**  | Universal document converter (cannot read PDFs)                  | --                                        |

## PDF-Specific Libraries

These focus on PDF extraction only. They're not direct competitors to Kreuzberg's full format coverage, but you'll often see them in PDF-heavy pipelines.

| Library                                                  | Language           | License      | Focus                                                              |
| -------------------------------------------------------- | ------------------ | ------------ | ------------------------------------------------------------------ |
| [PyMuPDF / PyMuPDF4LLM](https://pymupdf.readthedocs.io)  | Python (C core)    | **AGPL-3.0** | Fast PDF extraction via MuPDF. AGPL license limits commercial use. |
| [pdfplumber](https://github.com/jsvine/pdfplumber)       | Python             | MIT          | Good table extraction, built on pdfminer.six                       |
| [pdfminer.six](https://github.com/pdfminer/pdfminer.six) | Python             | MIT          | Fine-grained text positioning, pure Python                         |
| [pypdf](https://github.com/py-pdf/pypdf)                 | Python             | BSD-3        | Lightweight, pure Python, no C dependencies                        |
| [playa-pdf](https://github.com/dhdaines/playa)           | Python             | MIT          | Modern pure-Python PDF library                                     |
| [pdftotext](https://poppler.freedesktop.org)             | C (Python binding) | **GPL-2.0**  | Thin wrapper around poppler's pdftotext                            |

---

!!! Warning "License matters"

    Libraries marked **AGPL-3.0** (PyMuPDF, MinerU) require that any application using them also be released under AGPL, unless you purchase a commercial license. **GPL-2.0** tools (Pandoc, pdftotext/poppler) have similar copyleft requirements. If you're building a commercial product, check the license before integrating.

!!! Info "Benchmarks"

    Kreuzberg benchmarks against all of the libraries listed above. For extraction speed, quality scores, and format-by-format comparisons, see the [live benchmark dashboard](https://kreuzberg.dev/benchmarks).
