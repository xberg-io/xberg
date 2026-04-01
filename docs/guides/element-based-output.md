# Element-Based Output <span class="version-badge">v4.1.0</span>

Element-based output segments a document into a flat array of typed elements — titles, paragraphs, tables, list items, code blocks, images, and more. Each element carries its own metadata including page number and bounding box coordinates.

Use element-based output for RAG chunking, semantic search, or Unstructured.io-compatible pipelines. For hierarchical tree structure with parent-child relationships, use [document structure](document-structure.md). For plain text with metadata, use the default unified output.

## Enable

=== "Python"

    --8<-- "snippets/python/config/element_based_output.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/element_based_output.md"

=== "Rust"

    --8<-- "snippets/rust/config/element_based_output.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/element_based_output.md"

=== "Go"

    --8<-- "snippets/go/config/element_based_output.md"

=== "R"

    --8<-- "snippets/r/config/element_based_output.md"

=== "PHP"

    --8<-- "snippets/php/config/element_based_output.md"

Elements are in `result.elements`. Each element has `element_id`, `element_type`, `text`, and `metadata`.

## Element Types

| `element_type` | Description | Key `additional` fields |
|----------------|-------------|------------------------|
| `title` | Main title or top-level heading | `level` (h1–h6), `font_size`, `font_name` |
| `heading` | Section/subsection heading | `level` (h1–h6) |
| `narrative_text` | Body paragraph or prose | — |
| `list_item` | Bullet, numbered, or indented list item | `list_type`, `list_marker`, `indent_level` |
| `table` | Tabular data (Markdown or tab-separated text) | `row_count`, `column_count`, `format` |
| `image` | Embedded image | `format`, `width`, `height`, `alt_text` |
| `code_block` | Code snippet | `language`, `line_count` |
| `block_quote` | Quoted text | — |
| `header` | Recurring page header | `position` |
| `footer` | Recurring page footer | `position` |
| `page_break` | Page boundary marker | `next_page` |

## Metadata Structure

Every element's `metadata` contains:

| Field | Type | Description |
|-------|------|-------------|
| `page_number` | `int \| None` | Page number (1-indexed). Present for PDF, DOCX, PPTX. |
| `filename` | `str \| None` | Source filename |
| `coordinates` | `BoundingBox \| None` | `x0`, `y0`, `x1`, `y1` in PDF points. Present for PDF and OCR results. |
| `element_index` | `int` | Zero-based position in the elements array |
| `additional` | `dict[str, str]` | Element-type-specific fields (see table above) |

PDF coordinates use bottom-left origin in points (1/72 inch). `x0`/`y0` = bottom-left, `x1`/`y1` = top-right.

## Example Output

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

## Filtering Elements

```python
from kreuzberg import extract_file_sync, ExtractionConfig

config = ExtractionConfig(result_format="element_based")
result = extract_file_sync("document.pdf", config=config)

# Filter by type
titles = [e for e in result.elements if e.element_type == "title"]
tables = [e for e in result.elements if e.element_type == "table"]
list_items = [e for e in result.elements if e.element_type == "list_item"]

# Access type-specific metadata
for title in titles:
    level = title.metadata.additional.get("level", "h1")
    print(f"[{level}] {title.text}")

for table in tables:
    rows = table.metadata.additional.get("row_count")
    cols = table.metadata.additional.get("column_count")
    print(f"Table {rows}×{cols}: {table.text[:60]}")
```

## Unstructured.io Compatibility

Element-based output follows Unstructured.io's element array structure. Differences to note when migrating:

| Aspect | Unstructured.io | Kreuzberg |
|--------|----------------|-----------|
| Element type names | PascalCase (`Title`, `NarrativeText`) | snake_case (`title`, `narrative_text`) |
| Element IDs | Not always present | Always present (deterministic hash) |
| Metadata | Basic (`page_number`, `filename`) | Extended (coordinates, `additional` fields) |
| Config key | — | `result_format="element_based"` |
