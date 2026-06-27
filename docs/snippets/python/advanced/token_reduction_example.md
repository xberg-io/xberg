```python title="Python"
import asyncio
from xberg import ExtractInput, (
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

    result = await extract(ExtractInput.from_uri("verbose_document.pdf"), config)

    print(f"Reduced content length: {len(result.results[0].content)} chars")


asyncio.run(main())
```
