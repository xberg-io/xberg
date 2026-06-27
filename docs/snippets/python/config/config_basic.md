```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig

async def main() -> None:
    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True
    )
    result = await extract("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```
