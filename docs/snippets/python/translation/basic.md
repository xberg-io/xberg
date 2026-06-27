```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, TranslationConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        translation=TranslationConfig(
            target_lang="de",
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract("contract.pdf", config=config)
    if result.translation:
        print(result.translation.content)

asyncio.run(main())
```
