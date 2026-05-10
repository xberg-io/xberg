```python title="Python"
from kreuzberg import ExtractionConfig, LanguageDetectionConfig

config: ExtractionConfig = ExtractionConfig(
    language_detection=LanguageDetectionConfig(
        enabled=True,
        min_confidence=0.8,
        detect_multiple=False,
    )
)
```
