```python title="Python"
from kreuzberg import (
    batch_extract_files_sync,
    BatchFileItem,
    ExtractionConfig,
    KreuzbergError,
)

items = [
    BatchFileItem(path="doc1.pdf"),
    BatchFileItem(path="doc2.docx"),
    BatchFileItem(path="missing.html"),
]

config = ExtractionConfig()

try:
    results = batch_extract_files_sync(items, config=config)
    for i, result in enumerate(results):
        if result.metadata.error:
            print(f"Document {i}: ERROR - {result.metadata.error}")
        else:
            print(f"Document {i}: {len(result.content)} chars, {len(result.tables)} tables")
except KreuzbergError as e:
    print(f"Batch extraction failed: {e}")
    raise
```
