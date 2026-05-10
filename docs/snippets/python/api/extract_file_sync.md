```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

print(result.content[:200])
print(f"Tables: {len(result.tables)}")
print(f"Format: {result.metadata.format_type}")
```
