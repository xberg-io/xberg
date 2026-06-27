```python title="Python"
from xberg import extract_sync, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="easyocr", language="en")
)

# EasyOCR-specific options (use_gpu, beam_width, etc.) go in easyocr_kwargs,
# not in OcrConfig — OcrConfig only accepts backend, language, and backend-specific configs.
result = extract_sync("scanned.pdf", config=config, easyocr_kwargs={"use_gpu": True})

content: str = result.content
preview: str = content[:100]
total_length: int = len(content)

print(f"Extracted content (preview): {preview}")
print(f"Total characters: {total_length}")
```
