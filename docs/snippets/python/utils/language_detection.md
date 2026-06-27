```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, LanguageDetectionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True, min_confidence=0.9, detect_multiple=True
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Languages: {result.detected_languages}")

asyncio.run(main())
```
