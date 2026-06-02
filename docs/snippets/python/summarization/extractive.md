```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig, SummarizationConfig

async def main() -> None:
    config = ExtractionConfig(
        summarization=SummarizationConfig(
            strategy="extractive",
            max_tokens=200,
        ),
    )
    result = await extract_file("report.pdf", config=config)
    if result.summary:
        print(result.summary.text)

asyncio.run(main())
```
