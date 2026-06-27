```python title="Python"
import asyncio
from xberg import (
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
            ngram_range=(1, 3),
            language="en"
        )
    )
    result = await extract("document.pdf", config=config)
    print(f"Content extracted: {len(result.content)} chars")

asyncio.run(main())
```
