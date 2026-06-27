```python title="Python"
from xberg import extract_sync, ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=True,
    ),
)

result = extract_sync("multilingual_document.pdf", config=config)

if result.detected_languages:
    print(f"Detected languages: {', '.join(result.detected_languages)}")
```
