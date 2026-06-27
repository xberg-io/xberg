```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, extract


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True,
    )

    result = await extract(ExtractInput.from_uri("scanned_document.pdf"), config)

    if result.quality_score is not None:
        if result.quality_score < 0.5:
            print(f"Warning: Low quality extraction ({result.quality_score:.2f})")
        else:
            print(f"Quality score: {result.quality_score:.2f}")


asyncio.run(main())
```
