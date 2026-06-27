```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, TokenReductionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate", preserve_important_words=True
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Content length: {len(result.results[0].content)}")

asyncio.run(main())
```
