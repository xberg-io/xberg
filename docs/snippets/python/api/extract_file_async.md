```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main() -> None:
    result = await extract_file("document.pdf", config=ExtractionConfig())
    print(result.content[:200])
    print(f"Tables: {len(result.tables)}")
    print(f"Format: {result.metadata.format_type}")

asyncio.run(main())
```
