# Output Formats <span class="version-badge">v4.1.0</span>

Kreuzberg supports multiple output formats to suit different use cases — from plain text for LLM prompts to structured element arrays for RAG systems to hierarchical document trees for knowledge graphs.

Choose the format that best matches your downstream processing needs:

- **Unified (default)** — Plain text/markdown output, ideal for LLM prompts and full-text search
- **Element-Based** — Flat array of typed elements with metadata, suitable for RAG chunking and semantic search
- **Document Structure** — Hierarchical tree with explicit parent-child references, best for knowledge graphs and structured applications
- **PDF Hierarchy** — Font size classification into heading levels (H1–H6) for PDFs

## Unified Output (Default)

By default, Kreuzberg extracts plain text and markdown-formatted content. No configuration is required. The result contains:

- `content` — Full document text with minimal formatting
- `pages` — Per-page breakdown for PDFs, DOCX, and PPTX
- `tables` — Extracted tables in structured format
- `images` — Image metadata and paths

This format is ideal for:

- Feeding entire documents into LLMs without structured constraints
- Full-text search and indexing
- Simple text processing pipelines

---

## Element-Based Output <span class="version-badge">v4.1.0</span>

Segments a document into a flat array of typed elements — titles, paragraphs, tables, list items, code blocks, images, and more. Each element carries a page number and, for text elements in PDFs when hierarchy extraction is enabled, bounding box coordinates.

Use element-based output for RAG chunking, semantic search, or Unstructured.io-compatible pipelines. For hierarchical tree structure, use [document structure](#document-structure). For plain text, use the default unified output.

### Enable

=== "Python"

    --8<-- "snippets/python/config/element_based_output.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/element_based_output.md"

=== "Rust"

    --8<-- "snippets/rust/config/element_based_output.md"

=== "Go"

    --8<-- "snippets/go/config/element_based_output.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/element_based_output.md"

=== "R"

    --8<-- "snippets/r/config/element_based_output.md"

=== "PHP"

    --8<-- "snippets/php/config/element_based_output.md"

Elements are in `result.elements`. Each element has `element_id`, `element_type`, `text`, and `metadata`.

### Element Types

| `element_type` | Description | Key `additional` fields |
|----------------|-------------|------------------------|
| `title` | Main title or top-level heading | `level` (h1–h6), `font_size`, `font_name` |
| `heading` | Section/subsection heading | `level` (h1–h6) |
| `narrative_text` | Body paragraph | — |
| `list_item` | Bullet, numbered, or indented item | `list_type`, `list_marker`, `indent_level` |
| `table` | Tabular data | `row_count`, `column_count`, `format` |
| `image` | Embedded image | `format`, `width`, `height`, `alt_text` |
| `code_block` | Code snippet | `language`, `line_count` |
| `block_quote` | Quoted text | — |
| `header` | Recurring page header | `position` |
| `footer` | Recurring page footer | `position` |
| `page_break` | Page boundary marker | `next_page` |

### Metadata

Every element's `metadata` contains:

| Field | Type | Description |
|-------|------|-------------|
| `page_number` | `int \| None` | 1-indexed page number (PDF, DOCX, PPTX) |
| `filename` | `str \| None` | Source filename |
| `coordinates` | `BoundingBox \| None` | `x0`, `y0`, `x1`, `y1` in PDF points. Only populated for **text elements** when `pdf_options.hierarchy` is enabled with `include_bbox=True`. Table and image elements do not carry coordinates. |
| `element_index` | `int` | Zero-based position in the elements array |
| `additional` | `dict[str, str]` | Element-type-specific fields (see table above) |

PDF coordinates use bottom-left origin in points (1/72 inch).

### Example Output

```json
{
  "element_id": "elem-a3f2b1c4",
  "element_type": "title",
  "text": "Introduction to Machine Learning",
  "metadata": {
    "page_number": 1,
    "element_index": 0,
    "coordinates": { "x0": 72.0, "y0": 700.0, "x1": 540.0, "y1": 730.0 },
    "additional": { "level": "h1", "font_size": "24" }
  }
}
```

### Filtering Elements

```python
config = ExtractionConfig(result_format="element_based")
result = extract_file_sync("document.pdf", config=config)

titles = [e for e in result.elements if e.element_type == "title"]
tables = [e for e in result.elements if e.element_type == "table"]

for title in titles:
    level = title.metadata.additional.get("level", "h1")
    print(f"[{level}] {title.text}")
```

### Migrating from Unstructured.io

If you're migrating from Unstructured.io, element-based output follows a similar structure with these key differences:

| Aspect | Unstructured.io | Kreuzberg |
|--------|----------------|-----------|
| Type names | PascalCase (`Title`, `NarrativeText`) | snake_case (`title`, `narrative_text`) |
| Element IDs | Not always present | Always present (deterministic hash) |
| Metadata | Basic (`page_number`, `filename`) | Extended (coordinates, `additional` fields) |
| Config key | — | `result_format="element_based"` |

---

## Document Structure

Represents a document as a flat list of nodes with explicit parent-child index references — a traversable tree with heading levels, content layers, inline annotations, and structured table grids.

Use document structure when you need hierarchical relationships between sections. For a flat list of semantic elements, use [element-based output](#element-based-output). For plain text, use the default unified output.

### Comparison

| Aspect | Unified (default) | Element-based | Document structure |
|--------|------------------|---------------|--------------------|
| Output shape | `content: string` | `elements: array` | `nodes: array` with index refs |
| Hierarchy | None | Inferred from levels | Explicit parent/child indices |
| Inline annotations | No | No | Bold, italic, links per node |
| Tables | `result.tables` | Table elements | `TableGrid` with cell coords |
| Content layers | Not classified | Not classified | body, header, footer, footnote |
| Best for | LLM prompts, full-text | RAG chunking | Knowledge graphs, structured apps |

### Enable

=== "Python"

    --8<-- "snippets/python/config/document_structure_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/document_structure_config.md"

=== "Rust"

    --8<-- "snippets/rust/config/document_structure_config.md"

=== "Go"

    --8<-- "snippets/go/config/document_structure_config.md"

=== "Java"

    --8<-- "snippets/java/config/document_structure_config.md"

=== "C#"

    --8<-- "snippets/csharp/config/document_structure_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/document_structure_config.md"

=== "R"

    --8<-- "snippets/r/config/document_structure_config.md"

### Node Shape

Each node in `result.document.nodes`:

```json
{
  "id": "node-a3f2b1c4",
  "content": { "node_type": "heading", "level": 2, "text": "Supervised Learning" },
  "parent": 0,
  "children": [4, 5, 6],
  "content_layer": "body",
  "page": 5,
  "page_end": null,
  "bbox": { "x0": 72.0, "y0": 600.0, "x1": 400.0, "y1": 620.0 },
  "annotations": []
}
```

- `parent` and `children` are integer indices into the `nodes` array (`null` if absent)
- `bbox` is present when bounding box data is available
- `annotations` contains inline formatting spans

### Node Types

| `node_type` | Key fields | Notes |
|-------------|-----------|-------|
| `title` | `text` | Document title |
| `heading` | `level` (1–6), `text` | Section heading |
| `paragraph` | `text` | Body paragraph; may have `annotations` |
| `list` | `ordered` (bool) | Container; children are `list_item` nodes |
| `list_item` | `text` | Child of `list` |
| `table` | `grid` ([TableGrid](#table-grid)) | Grid with cell-level data |
| `image` | `description`, `image_index` | `image_index` references `result.images` |
| `code` | `text`, `language` | Code block |
| `quote` | _(container)_ | Children are typically paragraphs |
| `formula` | `text` | Math formula (plain text, LaTeX, or MathML) |
| `footnote` | `text` | Usually `content_layer: "footnote"` |
| `group` | `label`, `heading_level`, `heading_text` | Section grouping container |
| `page_break` | _(marker)_ | Page boundary |

### Content Layers

| Layer | Description |
|-------|-------------|
| `body` | Main document content |
| `header` | Page header area (repeated chapter titles) |
| `footer` | Page footer area (page numbers, copyright) |
| `footnote` | Footnotes and endnotes |

```python
for node in result.document["nodes"]:
    if node["content_layer"] == "body":
        process_main_content(node)
```

### Text Annotations

Paragraphs carry a list of `annotations` marking character spans:

```json
{ "start": 0, "end": 16, "kind": { "annotation_type": "bold" } }
```

| `annotation_type` | Extra fields |
|-------------------|-------------|
| `bold`, `italic`, `underline`, `strikethrough` | — |
| `code`, `subscript`, `superscript` | — |
| `link` | `url`, `title` (optional) |

```python
for node in result.document["nodes"]:
    for ann in node.get("annotations", []):
        text = node["content"].get("text", "")
        span = text[ann["start"]:ann["end"]]
        kind = ann["kind"]["annotation_type"]
        if kind == "link":
            print(f"Link: {span} -> {ann['kind']['url']}")
        else:
            print(f"{kind}: {span}")
```

### Table Grid

Table nodes contain a `grid` with cell-level data:

```json
{
  "rows": 3, "cols": 3,
  "cells": [
    { "content": "Algorithm", "row": 0, "col": 0, "row_span": 1, "col_span": 1, "is_header": true },
    { "content": "Decision Tree", "row": 1, "col": 0, "row_span": 1, "col_span": 1, "is_header": false }
  ]
}
```

Each cell has `row`, `col`, `row_span`, `col_span`, `is_header`, and optionally `bbox`.

```python
for node in result.document["nodes"]:
    if node["content"]["node_type"] == "table":
        grid = node["content"]["grid"]
        rows, cols = grid["rows"], grid["cols"]
        table = [[None] * cols for _ in range(rows)]
        for cell in grid["cells"]:
            table[cell["row"]][cell["col"]] = cell["content"]
        for row in table:
            print(" | ".join(str(c or "") for c in row))
```

---

## PDF Hierarchy Detection

Classifies text blocks in a PDF into heading levels (H1–H6) and body text based on font size analysis. Uses K-means clustering to group font sizes, then assigns heading levels by rank — largest cluster becomes H1, second-largest becomes H2, and so on.

### Quick Start

=== "Python"

    --8<-- "snippets/python/config/pdf_hierarchy_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/pdf_hierarchy_config.md"

=== "Rust"

    --8<-- "snippets/rust/config/pdf_hierarchy_config.md"

=== "Go"

    --8<-- "snippets/go/config/pdf_hierarchy_config.md"

=== "Java"

    --8<-- "snippets/java/config/pdf_hierarchy_config.md"

=== "C#"

    --8<-- "snippets/csharp/config/pdf_hierarchy_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/pdf_hierarchy_config.md"

### Output

Hierarchy data is in `result.pages[n].hierarchy`. Each page has a `blocks` list:

```json
{
  "block_count": 4,
  "blocks": [
    { "text": "Chapter 1: Introduction", "level": "h1", "font_size": 24.0, "bbox": [50.0, 100.0, 400.0, 125.0] },
    { "text": "Background", "level": "h2", "font_size": 18.0, "bbox": [50.0, 150.0, 300.0, 168.0] },
    { "text": "This chapter provides...", "level": "body", "font_size": 12.0, "bbox": [50.0, 200.0, 550.0, 450.0] }
  ]
}
```

- `bbox`: `[left, top, right, bottom]` in PDF points (present when `include_bbox=True`). This is the only way to obtain bounding box coordinates for text elements — they are not included by default.
- `level`: `"h1"` – `"h6"` or `"body"`

### Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable hierarchy extraction |
| `k_clusters` | `int` | `6` | Font size clusters (2–10), maps to heading levels |
| `include_bbox` | `bool` | `true` | Include bounding box coordinates |
| `ocr_coverage_threshold` | `float \| None` | `None` | Trigger OCR if text coverage is below this fraction |

#### Choosing k_clusters

| `k_clusters` | Heading levels | Use when |
|--------------|----------------|----------|
| 2–3 | H1–H2 | Simple documents with 1–2 heading sizes |
| 4–5 | H1–H4 | Standard documents |
| 6 (default) | H1–H6 | Most documents |
| 7–8 | H1–H6+ | Books, specs with deep nesting |

#### Ocr_coverage_threshold

| Threshold | Behavior |
|-----------|----------|
| `None` | OCR never triggered by coverage |
| `0.3` | OCR if < 30% of page has text |
| `0.5` | OCR if < 50% of page has text |

Requires an OCR backend to be configured separately.

### Troubleshooting

- **`hierarchy` is `None`** — Check `hierarchy.enabled` is `True`. If the PDF is image-only, enable OCR. If fewer text blocks than `k_clusters`, reduce `k_clusters`.
- **Most blocks classified as `body`** — Document may use uniform font sizes. Reduce `k_clusters` (try 3–4).
- **Heading levels don't match visual inspection** — Levels are assigned by font size rank, not absolute size. Filter on `block.font_size` directly for absolute thresholds.

See the [HierarchyConfig reference](../reference/configuration.md#hierarchyconfig) for the full parameter list.
