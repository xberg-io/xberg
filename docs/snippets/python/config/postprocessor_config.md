```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, PostProcessorConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        postprocessor=PostProcessorConfig(
            enabled=True,
            enabled_processors=["deduplication"],
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Content: {result.results[0].content[:100]}")

asyncio.run(main())
```
