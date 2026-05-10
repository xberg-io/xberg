```python title="Python"
import asyncio
import json

import httpx


async def main() -> None:
    async with httpx.AsyncClient() as client, open("document.pdf", "rb") as f:
        response = await client.post(
            "http://localhost:8000/extract",
            files={"files": f},
        )
    data = response.json()
    print(json.dumps(data, indent=2))


asyncio.run(main())
```
