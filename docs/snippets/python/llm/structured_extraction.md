```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, StructuredExtractionConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        structured_extraction=StructuredExtractionConfig(
            schema={
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "authors": {"type": "array", "items": {"type": "string"}},
                    "date": {"type": "string"},
                },
                "required": ["title", "authors", "date"],
                "additionalProperties": False,
            },
            llm=LlmConfig(model="openai/gpt-4o-mini"),
            strict=True,
        ),
    )
    result = await extract("paper.pdf", config=config)
    print(result.structured_output)
    # {"title": "...", "authors": ["..."], "date": "..."}

asyncio.run(main())
```
