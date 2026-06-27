```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, KeywordConfig, KeywordAlgorithm

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        keywords=KeywordConfig(
            algorithm=KeywordAlgorithm.YAKE,
            max_keywords=10,
            min_score=0.3
        )
    )
    output = await extract(ExtractInput.from_uri("research_paper.pdf"), config)
    result = output.results[0]

    keywords: list = result.extracted_keywords or []
    for kw in keywords:
        score: float = kw.score or 0.0
        text: str = kw.text or ""
        print(f"{text}: {score:.3f}")

asyncio.run(main())
```
