```python title="Python"
import asyncio
from kreuzberg import extract_file

async def main() -> None:
    result = await extract_file("document.pdf")

    content: str = result.content
    tables: int = len(result.tables)
    format_type: str | None = result.metadata.format.format_type if result.metadata.format else None

    print(f"Content length: {len(content)} characters")
    print(f"Tables found: {tables}")
    print(f"Format: {format_type}")

asyncio.run(main())
```
