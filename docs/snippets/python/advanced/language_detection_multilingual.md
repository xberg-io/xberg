```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, LanguageDetectionConfig, extract


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.8,
            detect_multiple=True,
        )
    )

    result = await extract(ExtractInput.from_uri("multilingual_document.pdf"), config)

    print(f"Detected languages: {result.results[0].detected_languages}")


asyncio.run(main())
```
