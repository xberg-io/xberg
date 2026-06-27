```python title="Python"
import asyncio
from xberg import ExtractionConfig, extract


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True,
    )

    result = await extract("scanned_document.pdf", config=config)

    if result.quality_score is not None:
        if result.quality_score < 0.5:
            print(f"Warning: Low quality extraction ({result.quality_score:.2f})")
        else:
            print(f"Quality score: {result.quality_score:.2f}")


asyncio.run(main())
```
