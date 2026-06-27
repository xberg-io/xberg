```python title="Python"
import asyncio
from xberg import ExtractInput, (
    ExtractionConfig,
    KeywordConfig,
    KeywordAlgorithm,
    extract,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3,
            language="en"
        )
    )
    output = await extract(ExtractInput.from_uri("document.pdf"), config)
    result = output.results[0]
    print(f"Content extracted: {len(result.results[0].content)} chars")

asyncio.run(main())
```
