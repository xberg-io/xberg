```python title="Python"
import asyncio
from xberg import extract

async def main() -> None:
    result = await extract("document.pdf")
    print(result.content)

asyncio.run(main())
```
