```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig

result = extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())

if result.results[0].metadata.pages and result.results[0].metadata.pages.boundaries:
    boundaries = result.results[0].metadata.pages.boundaries
    content_bytes = result.results[0].content.encode("utf-8")

    for boundary in boundaries[:3]:
        page_bytes = content_bytes[boundary.byte_start:boundary.byte_end]
        page_text = page_bytes.decode("utf-8")

        print(f"Page {boundary.page_number}:")
        print(f"  Byte range: {boundary.byte_start}-{boundary.byte_end}")
        print(f"  Preview: {page_text[:100]}...")
```
