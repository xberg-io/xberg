```python title="Python"
import asyncio
from xberg import ExtractionConfig, LanguageDetectionConfig, extract


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.8,
            detect_multiple=True,
        )
    )

    result = await extract("multilingual_document.pdf", config=config)

    print(f"Detected languages: {result.detected_languages}")


asyncio.run(main())
```
