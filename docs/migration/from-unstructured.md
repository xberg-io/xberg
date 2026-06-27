# Migrating from Unstructured to Xberg

This guide helps you migrate from Unstructured.io to Xberg for document intelligence workloads.

## Quick Start

**Unstructured API**:

```bash
curl -X POST "https://api.unstructured.io/general/v0/general" \
  -F 'files=@document.pdf'
```

**Xberg API**:

```bash
curl -X POST "http://localhost:8080/extract" \
  -F 'files=@document.pdf' \
  -F 'output_format=element_based'
```

## Output Format Comparison

### Unified Output (Default)

Xberg's default output provides richer metadata than Unstructured:

**Xberg Unified**:

```json
{
  "content": "Full document text...",
  "mime_type": "application/pdf",
  "metadata": {
    "title": "Document Title",
    "authors": ["Author Name"],
    "created_at": "2024-01-15T10:30:00Z",
    "format": {
      "format_type": "pdf",
      "page_count": 10,
      "version": "1.7"
    }
  },
  "tables": [...],
  "images": [...],
  "pages": [...]
}
```

### Element-Based Output

**Xberg** (when `output_format=element_based`):

```json
{
  "elements": [
    {
      "element_id": "elem-a3f2b1c4",
      "element_type": "title",
      "text": "Introduction",
      "metadata": {
        "page_number": 1,
        "filename": "Document Title",
        "coordinates": {
          "x0": 72.0,
          "y0": 100.0,
          "x1": 540.0,
          "y1": 130.0
        },
        "element_index": 0,
        "additional": {
          "level": "h1",
          "font_size": "24.0"
        }
      }
    },
    {
      "element_type": "narrative_text",
      "text": "This is a paragraph...",
      "metadata": {
        "page_number": 1
      }
    }
  ]
}
```

**Unstructured**:

```json
[
  {
    "type": "Title",
    "text": "Introduction",
    "metadata": {
      "page_number": 1,
      "filename": "document.pdf"
    }
  },
  {
    "type": "NarrativeText",
    "text": "This is a paragraph...",
    "metadata": {
      "page_number": 1
    }
  }
]
```

## API Endpoint Mapping

| Unstructured               | Xberg          | Notes                             |
| -------------------------- | ------------------ | --------------------------------- |
| `POST /general/v0/general` | `POST /extract`    | Single/batch extraction           |
| N/A                        | `POST /embed`      | Built-in embeddings (ONNX models) |
| N/A                        | `GET /health`      | Health check                      |
| N/A                        | `GET /cache/stats` | Cache statistics                  |

## Element Type Mapping

| Unstructured    | Xberg        | Notes                               |
| --------------- | ---------------- | ----------------------------------- |
| `Title`         | `title`          | PDF hierarchy (h1-h6) detection     |
| `NarrativeText` | `narrative_text` | Paragraphs split on double newlines |
| `ListItem`      | `list_item`      | Bullets, numbered, lettered         |
| `Table`         | `table`          | Tab-separated text representation   |
| `Image`         | `image`          | Format, dimensions in metadata      |
| `PageBreak`     | `page_break`     | Between pages in multi-page docs    |
| `Header`        | `header`         | Page header text                    |
| `Footer`        | `footer`         | Page footer text                    |
| N/A             | `heading`        | Section headings (beyond title)     |
| N/A             | `code_block`     | Code snippets                       |
| N/A             | `block_quote`    | Quoted text blocks                  |

## Code Examples

### Python

**Unstructured**:

```python
from unstructured.partition.auto import partition

elements = partition(filename="document.pdf")
for element in elements:
    print(f"{element.category}: {element.text}")
```

**Xberg**:

```python
from xberg import extract

# Option 1: Element-based output
config = {"output_format": "element_based"}
result = extract(pdf_bytes, "application/pdf", config)

for element in result.elements:
    print(f"{element.element_type}: {element.text}")
    if element.metadata.page_number:
        print(f"  Page: {element.metadata.page_number}")

# Option 2: Unified output (default, richer metadata)
result = extract(pdf_bytes, "application/pdf")
print(result.content)  # Full text
print(result.metadata.title)  # Document metadata
for page in result.pages:
    print(f"Page {page.page_number}: {page.content[:100]}")
```

### TypeScript

**Unstructured** (via API):

```typescript
const formData = new FormData();
formData.append("files", fileBlob);

const response = await fetch("https://api.unstructured.io/general/v0/general", {
  method: "POST",
  body: formData,
});
const elements = await response.json();
```

**Xberg**:

```typescript
import { extractBytes } from "xberg";

// Option 1: Element-based output
const result = await extractBytes(pdfBuffer, "application/pdf", {
  output_format: "element_based",
});

for (const element of result.elements) {
  console.log(`${element.element_type}: ${element.text}`);
}

// Option 2: Unified output with pages
const result = await extractBytes(pdfBuffer, "application/pdf", {
  pages: { extract_pages: true },
});

for (const page of result.pages) {
  console.log(`Page ${page.page_number}:`, page.content);
}
```

### CURL

**Unstructured**:

```bash
curl -X POST "https://api.unstructured.io/general/v0/general" \
  -H "unstructured-api-key: $API_KEY" \
  -F 'files=@document.pdf' \
  -F 'strategy=hi_res'
```

**Xberg**:

```bash
# Element-based output
curl -X POST "http://localhost:8080/extract" \
  -F 'files=@document.pdf' \
  -F 'output_format=element_based'

# With configuration JSON
curl -X POST "http://localhost:8080/extract" \
  -F 'files=@document.pdf' \
  -F 'config={"output_format":"element_based","pages":{"extract_pages":true}}'
```

## Feature Comparison

### What Xberg Adds

1. **Richer Metadata**: Format-specific discriminated unions (PDF, Excel, Email, etc.)
2. **Native Per-Page**: `PageContent` with byte offsets, hierarchy, tables, images per page
3. **96 Formats**: vs Unstructured's ~30 formats
4. **Performance**: Rust-based native implementation (vs Python-based)
5. **10 Language Bindings**: Python, TypeScript, Ruby, PHP, Go, Java, C#, Elixir, Rust, WASM
6. **Built-in Embeddings**: ONNX models via `/embed` endpoint (no external API)
7. **Smart Hierarchy**: PDF font-size clustering for h1-h6 detection
8. **Bounding Boxes**: Preserved from PDF source in element coordinates

### What Unstructured Has

1. **Layout Detection Models**: ML-based layout analysis (GPU-accelerated)
2. **Cloud API**: Hosted service (Xberg requires self-hosting)
3. **More Element Types**: More granular element classification
4. **Mature Ecosystem**: Larger community, more integrations

## Configuration Mapping

| Unstructured Parameter                | Xberg Config                     | Notes                              |
| ------------------------------------- | ------------------------------------ | ---------------------------------- |
| `strategy=hi_res`                     | `pdf_options.hierarchy.enabled=true` | PDF hierarchy extraction           |
| `coordinates=true`                    | Always included when available       | Bounding boxes in element metadata |
| `languages=["eng"]`                   | `ocr.language="eng"`                 | OCR language                       |
| `extract_image_block_types=["image"]` | `images.extract_images=true`         | Image extraction                   |
| `chunking_strategy="by_title"`        | `chunking.max_chars=1000`            | Text chunking (basic)              |
| `embedding_model="..."`               | `chunking.embedding.model="..."`     | Embedding generation               |

## Migration Checklist

- [ ] Update API endpoint URLs (Unstructured → Xberg)
- [ ] Add `output_format=element_based` if using element-based workflow
- [ ] Update element type references (`Title` → `title`, camelCase → snake_case)
- [ ] Update metadata field references (Xberg has richer metadata structure)
- [ ] Test with sample documents to verify output equivalence
- [ ] Update error handling (Xberg uses HTTP 422 for validation errors)
- [ ] Configure caching if needed (Xberg has built-in file-based cache)
- [ ] Set up embeddings if using RAG pipeline (Xberg has built-in ONNX support)

## Advanced: Hybrid Approach

You can use **both formats** simultaneously:

```python
from xberg import extract

result = extract(pdf_bytes, "application/pdf", {
    "output_format": "element_based",  # Get elements
    "pages": {"extract_pages": true}   # Also get per-page content
})

# Element-based processing
for element in result.elements:
    if element.element_type == "title":
        index_heading(element.text)

# Page-based processing
for page in result.pages:
    if page.hierarchy:
        for block in page.hierarchy.blocks:
            if block.level == "h1":
                process_section(block.text)
```

## Performance Tips

1. **Enable Caching**: `use_cache: true` (default) for repeated extractions
2. **Disable OCR**: If documents are searchable PDFs, set `force_ocr: false`
3. **Limit Page Extraction**: Only enable `pages` if you need per-page content
4. **Batch Processing**: Send multiple files in single request (up to 10MB total)
5. **Use Embeddings Wisely**: Enable only for chunked content destined for vector DB

## Getting Help

- **Documentation**: <https://github.com/xberg-io/Xberg>
- **Issues**: <https://github.com/xberg-io/xberg/issues>
- **API Reference**: See `docs/api/` for endpoint documentation

## Next Steps

After migration:

1. Review the [Xberg vs Unstructured Comparison](../comparisons/xberg-vs-unstructured.md)
2. Explore Xberg-specific features (hierarchy, per-page metadata, embeddings)
3. Optimize your pipeline with native Rust performance
