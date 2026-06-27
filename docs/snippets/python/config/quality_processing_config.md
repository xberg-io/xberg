```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)

    quality_score: float = result.quality_score or 0.0
    print(f"Quality score: {quality_score:.2f}")

asyncio.run(main())
```
