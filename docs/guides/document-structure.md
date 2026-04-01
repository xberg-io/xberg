# Document Structure

Represents a document as a flat list of nodes with explicit parent-child index references — a traversable tree with heading levels, content layers, inline annotations, and structured table grids.

Use document structure when you need hierarchical relationships between sections. For a flat list of semantic elements, use [element-based output](element-based-output.md). For plain text, use the default unified output.

## Comparison

| Aspect | Unified (default) | Element-based | Document structure |
|--------|------------------|---------------|--------------------|
| Output shape | `content: string` | `elements: array` | `nodes: array` with index refs |
| Hierarchy | None | Inferred from levels | Explicit parent/child indices |
| Inline annotations | No | No | Bold, italic, links per node |
| Tables | `result.tables` | Table elements | `TableGrid` with cell coords |
| Content layers | Not classified | Not classified | body, header, footer, footnote |
| Best for | LLM prompts, full-text | RAG chunking | Knowledge graphs, structured apps |

## Enable

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

## Node Shape

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

## Node Types

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

## Content Layers

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

## Text Annotations

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

## Table Grid

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
