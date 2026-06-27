```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, TokenReductionConfig

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        token_reduction=TokenReductionConfig(
            mode="moderate", preserve_important_words=True
        )
    )
    result = await extract(ExtractInput.from_uri("verbose_document.pdf"), config)
    original: int = result.results[0].metadata.get("original_token_count", 0)
    reduced: int = result.results[0].metadata.get("token_count", 0)
    ratio: float = result.results[0].metadata.get("token_reduction_ratio", 0.0)
    print(f"Reduced from {original} to {reduced} tokens")
    print(f"Reduction: {ratio * 100:.1f}%")

asyncio.run(main())
```
