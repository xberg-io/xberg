```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, TokenReductionConfig

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate", preserve_important_words=True
        )
    )
    result = await extract("verbose_document.pdf", config=config)
    original: int = result.metadata.get("original_token_count", 0)
    reduced: int = result.metadata.get("token_count", 0)
    ratio: float = result.metadata.get("token_reduction_ratio", 0.0)
    print(f"Reduced from {original} to {reduced} tokens")
    print(f"Reduction: {ratio * 100:.1f}%")

asyncio.run(main())
```
