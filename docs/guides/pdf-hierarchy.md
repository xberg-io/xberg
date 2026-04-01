# PDF Hierarchy Detection

Classifies text blocks in a PDF into heading levels (H1–H6) and body text based on font size analysis. Uses K-means clustering to group font sizes, then assigns heading levels by rank — largest cluster becomes H1, second-largest becomes H2, and so on.

## Quick Start

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

## Output

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

- `bbox`: `[left, top, right, bottom]` in PDF points (present when `include_bbox=True`)
- `level`: `"h1"` – `"h6"` or `"body"`

## Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable hierarchy extraction |
| `k_clusters` | `int` | `6` | Font size clusters (2–10), maps to heading levels |
| `include_bbox` | `bool` | `true` | Include bounding box coordinates |
| `ocr_coverage_threshold` | `float \| None` | `None` | Trigger OCR if text coverage is below this fraction |

### Choosing k_clusters

| `k_clusters` | Heading levels | Use when |
|--------------|----------------|----------|
| 2–3 | H1–H2 | Simple documents with 1–2 heading sizes |
| 4–5 | H1–H4 | Standard documents |
| 6 (default) | H1–H6 | Most documents |
| 7–8 | H1–H6+ | Books, specs with deep nesting |

### ocr_coverage_threshold

| Threshold | Behavior |
|-----------|----------|
| `None` | OCR never triggered by coverage |
| `0.3` | OCR if < 30% of page has text |
| `0.5` | OCR if < 50% of page has text |

Requires an OCR backend to be configured separately.

## Troubleshooting

- **`hierarchy` is `None`** — Check `hierarchy.enabled` is `True`. If the PDF is image-only, enable OCR. If fewer text blocks than `k_clusters`, reduce `k_clusters`.
- **Most blocks classified as `body`** — Document may use uniform font sizes. Reduce `k_clusters` (try 3–4).
- **Heading levels don't match visual inspection** — Levels are assigned by font size rank, not absolute size. Filter on `block.font_size` directly for absolute thresholds.

See the [HierarchyConfig reference](../reference/configuration.md#hierarchyconfig) for the full parameter list.
