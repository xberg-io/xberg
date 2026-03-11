```python title="Python"
import asyncio
from kreuzberg import ExtractionConfig, ChunkingConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            max_chars=1000,
            max_overlap=200,
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Chunks: {len(result.chunks or [])}")
    for chunk in result.chunks or []:
        print(f"Length: {len(chunk.content)}")

asyncio.run(main())
```

```python title="Python - Markdown with Heading Context"
import asyncio
from kreuzberg import ExtractionConfig, ChunkingConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            chunker_type="markdown",
            max_chars=500,
            max_overlap=50,
            sizing_type="tokenizer",
            sizing_model="Xenova/gpt-4o",
        )
    )
    result = await extract_file("document.md", config=config)
    for chunk in result.chunks or []:
        heading_context = chunk.metadata.get("heading_context")
        if heading_context:
            headings = heading_context.get("headings", [])
            for h in headings:
                print(f"Heading L{h['level']}: {h['text']}")
        print(f"Content: {chunk.content[:100]}...")

asyncio.run(main())
```
