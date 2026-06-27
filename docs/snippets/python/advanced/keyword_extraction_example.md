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
        )
    )

    output = await extract(ExtractInput.from_uri("research_paper.pdf"), config)
    result = output.results[0]

    for keyword in result.results[0].extracted_keywords or []:
        print(f"{keyword.text}: {keyword.score:.3f}")


asyncio.run(main())
```
