```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig, SummarizationConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        summarization=SummarizationConfig(
            strategy="abstractive",
            max_tokens=300,
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract_file("report.pdf", config=config)
    if result.summary:
        print(result.summary.text)

asyncio.run(main())
```
