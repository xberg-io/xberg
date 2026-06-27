```python title="Python"
import asyncio
from xberg import ExtractionConfig, TokenReductionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate", preserve_important_words=True
        )
    )
    result = await extract("document.pdf", config=config)
    print(f"Content length: {len(result.content)}")

asyncio.run(main())
```
