```python title="Python"
from xberg import extract, ExtractionConfig

config = ExtractionConfig(enable_quality_processing=True)
result = extract("scanned_document.pdf", config=config)

quality_score = result.quality_score or 0.0

if quality_score < 0.5:
    print(f"Warning: Low quality extraction ({quality_score:.2f})")
    print("Consider re-scanning with higher DPI or adjusting OCR settings")
else:
    print(f"Quality score: {quality_score:.2f}")
```
