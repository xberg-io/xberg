```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, SummarizationConfig

async def main() -> None:
    config = ExtractionConfig(
        summarization=SummarizationConfig(
            strategy="extractive",
            max_tokens=200,
        ),
    )
    result = await extract("report.pdf", config=config)
    if result.summary:
        print(result.summary.text)

asyncio.run(main())
```
