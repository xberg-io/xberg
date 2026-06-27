```python title="Python"
from xberg import ExtractInput, (
    extract,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    TokenReductionConfig,
    LanguageDetectionConfig,
)

config = ExtractionConfig(
    ocr=OcrConfig(backend="tesseract", language="eng+deu"),
    chunking=ChunkingConfig(max_chars=1000, max_overlap=100),
    token_reduction=TokenReductionConfig(mode="light"),
    language_detection=LanguageDetectionConfig(
        enabled=True, detect_multiple=True
    ),
    use_cache=True,
    enable_quality_processing=True,
)

result = extract(ExtractInput.from_uri("document.pdf"), config)

for chunk in result.results[0].chunks or []:
    print(f"Chunk: {chunk.content[:100]}")

if result.results[0].detected_languages:
    print(f"Languages: {result.results[0].detected_languages}")
```
