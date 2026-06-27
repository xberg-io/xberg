```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, SummarizationConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        summarization=SummarizationConfig(
            strategy="abstractive",
            max_tokens=300,
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract(ExtractInput.from_uri("report.pdf"), config)
    if result.summary:
        print(result.summary.text)

asyncio.run(main())
```
