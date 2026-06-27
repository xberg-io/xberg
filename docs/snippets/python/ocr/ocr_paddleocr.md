```python title="Python"
from xberg import extract_sync, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")  # model_tier="server" for max accuracy
)

result = extract_sync("scanned.pdf", config=config)

content: str = result.content
preview: str = content[:100]
total_length: int = len(content)

print(f"Extracted content (preview): {preview}")
print(f"Total characters: {total_length}")
```
