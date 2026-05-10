```python title="Python"
import asyncio

import httpx


async def main() -> None:
    payload = {
        "text": "Your long text content here...",
        "chunker_type": "text",
        "config": {
            "max_characters": 1000,
            "overlap": 50,
            "trim": True,
        },
    }

    async with httpx.AsyncClient() as client:
        response = await client.post("http://localhost:8000/chunk", json=payload)
    result = response.json()

    print(f"Created {result['chunk_count']} chunks")
    for chunk in result["chunks"]:
        preview = chunk["content"][:50]
        print(f"Chunk {chunk['chunk_index']}: {preview}...")


asyncio.run(main())
```
