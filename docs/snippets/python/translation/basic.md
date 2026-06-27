```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, TranslationConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        translation=TranslationConfig(
            target_lang="de",
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract(ExtractInput.from_uri("contract.pdf"), config)
    if result.translation:
        print(result.translation.content)

asyncio.run(main())
```
