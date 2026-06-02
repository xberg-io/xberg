```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig, NerConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        ner=NerConfig(
            backend="llm",
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract_file("contract.pdf", config=config)
    for entity in result.entities or []:
        print(f"{entity.category}: {entity.text} (confidence={entity.confidence})")

asyncio.run(main())
```
