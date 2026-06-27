```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, XbergError

config = ExtractionConfig()

try:
    result = extract(ExtractInput.from_uri("missing.pdf"), config)
except XbergError as e:
    print(f"Extraction failed: {e}")
    raise
```
