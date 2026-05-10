```python title="Python"
import asyncio
from kreuzberg import (
    ExtractionConfig,
    TokenReductionConfig,
    ReductionLevel,
    extract_file,
)


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            level=ReductionLevel.MODERATE,
            preserve_markdown=True,
        )
    )

    result = await extract_file("verbose_document.pdf", config=config)

    print(f"Reduced content length: {len(result.content)} chars")


asyncio.run(main())
```
