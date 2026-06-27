```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=True,
    ),
)

result = extract(ExtractInput.from_uri("multilingual_document.pdf"), config)

if result.results[0].detected_languages:
    print(f"Detected languages: {', '.join(result.results[0].detected_languages)}")
```
