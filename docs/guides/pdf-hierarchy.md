# PDF Hierarchy Detection

PDF hierarchy detection classifies text blocks in a PDF into heading levels (H1–H6) and body text based on font size analysis. It uses K-means clustering to group font sizes, then assigns heading levels by rank — largest cluster becomes H1, second-largest becomes H2, and so on.

## Quick Start

=== "Python"

    --8<-- "snippets/python/config/pdf_hierarchy_config.md"

=== "TypeScript"

    --8<-- "snippets/typescript/config/pdf_hierarchy_config.md"

=== "Rust"

    --8<-- "snippets/rust/config/pdf_hierarchy_config.md"

=== "Ruby"

    --8<-- "snippets/ruby/config/pdf_hierarchy_config.md"

=== "Go"

    --8<-- "snippets/go/config/pdf_hierarchy_config.md"

=== "Java"

    --8<-- "snippets/java/config/pdf_hierarchy_config.md"

=== "C#"

    --8<-- "snippets/csharp/config/pdf_hierarchy_config.md"

## Output

Hierarchy data is in `result.pages[n].hierarchy`. Each page has a `blocks` list:

```json
{
  "block_count": 4,
  "blocks": [
    {
      "text": "Chapter 1: Introduction",
      "level": "h1",
      "font_size": 24.0,
      "bbox": [50.0, 100.0, 400.0, 125.0]
    },
    {
      "text": "Background",
      "level": "h2",
      "font_size": 18.0,
      "bbox": [50.0, 150.0, 300.0, 168.0]
    },
    {
      "text": "This chapter provides...",
      "level": "body",
      "font_size": 12.0,
      "bbox": [50.0, 200.0, 550.0, 450.0]
    }
  ]
}
```

`bbox` format: `[left, top, right, bottom]` in PDF points. Present only when `include_bbox=True`.

`level` values: `"h1"`, `"h2"`, `"h3"`, `"h4"`, `"h5"`, `"h6"`, `"body"`

## Configuration

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | `bool` | `true` | Enable hierarchy extraction |
| `k_clusters` | `int` | `6` | Number of font size clusters (2–10). Maps to heading levels. |
| `include_bbox` | `bool` | `true` | Include bounding box coordinates in output |
| `ocr_coverage_threshold` | `float \| None` | `None` | Trigger OCR if text coverage is below this fraction of the page area. `None` disables the check. |

### Choosing k_clusters

| `k_clusters` | Heading levels | Use when |
|--------------|----------------|----------|
| `2–3` | H1–H2 | Simple documents with 1–2 heading sizes |
| `4–5` | H1–H4 | Standard documents |
| `6` | H1–H6 | Default — works for most documents |
| `7–8` | H1–H6+ | Books, specs with deep nesting |

If you're unsure, leave it at `6`. Reduce if most blocks classify as `body`.

### ocr_coverage_threshold

| Threshold | Behavior |
|-----------|----------|
| `None` | OCR never triggered by coverage |
| `0.3` | OCR if less than 30% of page has text |
| `0.5` | OCR if less than 50% of page has text |
| `0.8` | OCR for heavily scanned pages |

Requires an OCR backend to be configured separately — this parameter only controls when the condition is met.

## Troubleshooting

**`page.hierarchy` is `None` or empty:**

- Check `hierarchy.enabled` is `True`
- If `result.content` is empty, the PDF may be image-only — enable OCR
- If the document has fewer text blocks than `k_clusters`, reduce `k_clusters`

**Most blocks are classified as `body`:**

- The document may use uniform font sizes — hierarchy detection relies on font size variation
- `k_clusters` may be too high — reduce it (try `3` or `4`)

**Heading levels don't match visual inspection:**

This is expected behavior. Levels are assigned by font size rank, not absolute size. The largest font is always H1, the second-largest is H2, and so on. The algorithm doesn't "know" which size is a heading — it clusters by size and ranks the results.

To use absolute thresholds instead, filter on `block.font_size` directly after extraction.

**Inconsistent results across runs:**

Kreuzberg initializes K-means from actual font sizes (deterministic). If you see inconsistency, check that the same config object isn't being mutated across concurrent calls.

See the [HierarchyConfig reference](../reference/configuration.md#hierarchyconfig) for the full parameter list.
