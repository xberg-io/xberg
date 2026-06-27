```python title="Python"
import asyncio
from xberg import ExtractionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True
    )
    result = await extract("document.pdf", config=config)

    quality_score: float = result.quality_score or 0.0
    print(f"Quality score: {quality_score:.2f}")

asyncio.run(main())
```
