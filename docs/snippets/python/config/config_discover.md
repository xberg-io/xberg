```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig()
    result = await extract(ExtractInput.from_uri("document.pdf"), config)

    content: str = result.results[0].content
    content_preview: str = content[:100]

    print(f"Content preview: {content_preview}")
    print(f"Total length: {len(content)}")

asyncio.run(main())
```
