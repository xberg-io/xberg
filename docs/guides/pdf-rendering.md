# PDF Page Rendering

!!! info "Added in v4.6.2"

Render individual PDF pages as PNG images. Unlike the extraction pipeline (which parses text, tables, metadata), this API produces raw pixel data for thumbnails, vision model input, or custom OCR pipelines.

## Two Approaches

| API | When to use |
|-----|-------------|
| `render_pdf_page` | You know which page you need, or only need a few pages |
| `PdfPageIterator` | Process every page sequentially without loading all images into memory |

### Single Page

=== "Python"

    --8<-- "snippets/python/api/render_pdf_page.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/render_pdf_page.md"

=== "Rust"

    --8<-- "snippets/rust/api/render_pdf_page.md"

=== "Go"

    --8<-- "snippets/go/api/render_pdf_page.md"

=== "Java"

    --8<-- "snippets/java/api/render_pdf_page.md"

=== "C#"

    --8<-- "snippets/csharp/api/render_pdf_page.md"

=== "Ruby"

    --8<-- "snippets/ruby/api/render_pdf_page.md"

=== "PHP"

    --8<-- "snippets/php/api/render_pdf_page.md"

=== "R"

    --8<-- "snippets/r/api/render_pdf_page.md"

=== "Elixir"

    --8<-- "snippets/elixir/api/render_pdf_page.md"

=== "C"

    --8<-- "snippets/c/api/render_pdf_page.md"

### Page Iterator

Renders one page at a time, releasing each page's memory before advancing. Peak memory stays proportional to one page regardless of document length.

=== "Python"

    --8<-- "snippets/python/api/render_pdf_page_iterator.md"

=== "TypeScript"

    --8<-- "snippets/typescript/api/render_pdf_page_iterator.md"

=== "Rust"

    --8<-- "snippets/rust/api/render_pdf_page_iterator.md"

=== "Go"

    --8<-- "snippets/go/api/render_pdf_page_iterator.md"

=== "Java"

    --8<-- "snippets/java/api/render_pdf_page_iterator.md"

=== "C#"

    --8<-- "snippets/csharp/api/render_pdf_page_iterator.md"

=== "C"

    --8<-- "snippets/c/api/render_pdf_page_iterator.md"

!!! note "Iterator availability"
    `PdfPageIterator` is available in Python, TypeScript, Rust, Go, Java, C#, and C. Ruby, PHP, R, and Elixir provide `render_pdf_page` only — iterate pages with a loop over page indices.

## DPI Configuration

| DPI | Pixel size (US Letter) | Use case |
|-----|----------------------|----------|
| 72 | 612 x 792 | Thumbnails, quick previews |
| 150 (default) | 1275 x 1650 | General-purpose, screen display |
| 300 | 2550 x 3300 | OCR input, print quality |

!!! tip "DPI for OCR"
    Use 300 DPI when rendering pages for OCR or vision models. The default 150 DPI may reduce recognition accuracy on small text.

## Examples

### Thumbnails

```python title="Python"
from kreuzberg import render_pdf_page

thumbnail = render_pdf_page("report.pdf", page_index=0, dpi=72)
with open("thumbnail.png", "wb") as f:
    f.write(thumbnail)
```

### Vision Model Input

```python title="Python"
import base64
from kreuzberg import render_pdf_page

png = render_pdf_page("chart.pdf", page_index=2, dpi=300)
b64 = base64.b64encode(png).decode()
```

### Batch-Render All Pages

```python title="Python"
from pathlib import Path
from kreuzberg import render_pdf_page

output_dir = Path("pages")
output_dir.mkdir(exist_ok=True)

for i in range(total_pages):
    png = render_pdf_page("document.pdf", page_index=i, dpi=150)
    (output_dir / f"page_{i}.png").write_bytes(png)
```
