```python title="Python"
import asyncio
from xberg import ExtractInput, extract, __version__, ExtractionConfig

async def main() -> None:
    print(f"Xberg version: {__version__}")

    result = await extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())
    print(f"Extraction successful: {len(result.results[0].content) > 0}")

asyncio.run(main())
```
