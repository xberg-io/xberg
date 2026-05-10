```python title="Python"
from kreuzberg import batch_extract_files_sync, BatchFileItem, ExtractionConfig

items = [
    BatchFileItem(path="doc1.pdf"),
    BatchFileItem(path="doc2.docx"),
    BatchFileItem(path="doc3.html"),
]

results = batch_extract_files_sync(items, ExtractionConfig())

for i, result in enumerate(results):
    print(f"Document {i}: {len(result.content)} chars, {len(result.tables)} tables")
```
