```python title="Python"
import asyncio
from xberg import extract, __version__

async def main() -> None:
    print(f"Xberg version: {__version__}")

    result = await extract("document.pdf")
    print(f"Extraction successful: {len(result.content) > 0}")

asyncio.run(main())
```
