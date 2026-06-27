```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig

async def main() -> None:
    result = await extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())
    print(result.results[0].content)

asyncio.run(main())
```
