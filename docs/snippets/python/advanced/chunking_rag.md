```python title="Python"
import asyncio
from xberg import (
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    extract,
)


async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            max_characters=500,
            overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType({"type": "preset", "name": "balanced"}),
                normalize=True,
            ),
        )
    )

    result = await extract("research_paper.pdf", config=config)

    for chunk in result.chunks or []:
        print(
            f"Chunk {chunk.metadata.chunk_index + 1}/{chunk.metadata.total_chunks}"
        )
        print(
            f"Position: {chunk.metadata.byte_start}-{chunk.metadata.byte_end}"
        )
        print(f"Content: {chunk.content[:100]}...")
        if chunk.embedding is not None:
            print(f"Embedding: {len(chunk.embedding)} dimensions")


asyncio.run(main())
```
