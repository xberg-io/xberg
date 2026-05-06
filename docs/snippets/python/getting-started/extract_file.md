```python title="Python"
import asyncio
from pathlib import Path
from kreuzberg import extract_file

async def main() -> None:
    file_path: Path = Path("document.pdf")

    result = await extract_file(file_path)

    print(f"Content: {result.content}")
    print(f"Format: {result.metadata.format.format_type if result.metadata.format else None}")
    print(f"Tables: {len(result.tables)}")

asyncio.run(main())
```
