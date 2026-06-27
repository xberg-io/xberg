```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig

config = ExtractionConfig(enable_quality_processing=True)
result = extract(ExtractInput.from_uri("scanned_document.pdf"), config)

quality_score = result.quality_score or 0.0

if quality_score < 0.5:
    print(f"Warning: Low quality extraction ({quality_score:.2f})")
    print("Consider re-scanning with higher DPI or adjusting OCR settings")
else:
    print(f"Quality score: {quality_score:.2f}")
```
