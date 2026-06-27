```python title="Python"
from xberg import ExtractInput, (
    extract,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
)

config: ExtractionConfig = ExtractionConfig(
    use_cache=True,
    ocr=OcrConfig(backend="tesseract", language="eng"),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=200),
)

result = extract(ExtractInput.from_uri("document.pdf"), config)
content_length: int = len(result.results[0].content)
print(f"Content length: {content_length}")
```
