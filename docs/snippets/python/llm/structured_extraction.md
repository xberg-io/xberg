```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, StructuredExtractionConfig, LlmConfig

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
    result = await extract(ExtractInput.from_uri("paper.pdf"), config)
    print(result.structured_output)
    # {"title": "...", "authors": ["..."], "date": "..."}

asyncio.run(main())
```
