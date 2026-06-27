```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract"),
    force_ocr=True,
)

result = extract(ExtractInput.from_uri("document.pdf"), config)

content: str = result.results[0].content
preview: str = content[:100]
total_length: int = len(content)

print(f"Extracted content (preview): {preview}")
print(f"Total characters: {total_length}")
```
