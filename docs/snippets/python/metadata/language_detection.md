```python title="Python"
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.9,
        detect_multiple=True,
    ),
)

print(config.language_detection)
```
