```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, KeywordConfig, KeywordAlgorithm

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3
        )
    )
    result = await extract("research_paper.pdf", config=config)

    keywords: list = result.extracted_keywords or []
    for kw in keywords:
        score: float = kw.score or 0.0
        text: str = kw.text or ""
        print(f"{text}: {score:.3f}")

asyncio.run(main())
```
