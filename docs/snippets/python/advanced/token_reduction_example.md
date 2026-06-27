```python title="Python"
import asyncio
from xberg import (
    ExtractionConfig,
    TokenReductionConfig,
    ReductionLevel,
    extract,
)


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            level=ReductionLevel.MODERATE,
            preserve_markdown=True,
        )
    )

    result = await extract("verbose_document.pdf", config=config)

    print(f"Reduced content length: {len(result.content)} chars")


asyncio.run(main())
```
