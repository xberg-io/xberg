```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, OcrConfig, LlmConfig

async def main() -> None:
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="vlm",
            vlm_config=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    )
    result = await extract("scan.pdf", config=config)
    print(result.content)

asyncio.run(main())
```
