```python title="Python"
from kreuzberg import batch_extract_bytes_sync, BatchBytesItem, ExtractionConfig

items = [
    BatchBytesItem(content=b"PDF content", mime_type="application/pdf"),
    BatchBytesItem(content=b"<html>...</html>", mime_type="text/html"),
]

results = batch_extract_bytes_sync(items, ExtractionConfig())

for i, result in enumerate(results):
    print(f"Item {i}: {len(result.content)} chars extracted")
```
