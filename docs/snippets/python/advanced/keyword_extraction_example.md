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
        )
    )

    result = await extract("research_paper.pdf", config=config)

    for keyword in result.extracted_keywords or []:
        print(f"{keyword.text}: {keyword.score:.3f}")


asyncio.run(main())
```
