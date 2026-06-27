```python title="Python"
from xberg import extract_sync, ExtractionConfig, XbergError

config = ExtractionConfig()

try:
    result = extract_sync("missing.pdf", config=config)
except XbergError as e:
    print(f"Extraction failed: {e}")
    raise
```
