```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig, KreuzbergError

config = ExtractionConfig()

try:
    result = extract_file_sync("missing.pdf", config=config)
except KreuzbergError as e:
    print(f"Extraction failed: {e}")
    raise
```
