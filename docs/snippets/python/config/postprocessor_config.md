```python title="Python"
import asyncio
from xberg import ExtractionConfig, PostProcessorConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["deduplication"],
        )
    )
    result = await extract("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")

asyncio.run(main())
```
