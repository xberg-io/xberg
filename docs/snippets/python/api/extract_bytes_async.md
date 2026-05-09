```python title="Python"
import asyncio
from kreuzberg import extract_bytes, ExtractionConfig

async def main() -> None:
    with open("document.pdf", "rb") as f:
        content = f.read()
    
    result = await extract_bytes(content, "application/pdf", config=ExtractionConfig())
    print(result.content[:200])
    print(f"Tables: {len(result.tables)}")

asyncio.run(main())
```
