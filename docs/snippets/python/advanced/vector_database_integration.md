```python title="Python"
import asyncio
from dataclasses import dataclass, field
from xberg import (
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    extract,
)


@dataclass
class VectorRecord:
    id: str
    content: str
    embedding: list[float]
    metadata: dict[str, str] = field(default_factory=dict)


async def extract_and_vectorize(
    document_path: str,
    document_id: str,
) -> list[VectorRecord]:
    config: ExtractionConfig = ExtractionConfig(
        chunking=ChunkingConfig(
            max_characters=512,
            overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType({"type": "preset", "name": "balanced"}),
                normalize=True,
                batch_size=32,
            ),
        )
    )

    result = await extract(document_path, config=config)

    records: list[VectorRecord] = []
    for index, chunk in enumerate(result.chunks or []):
        if chunk.embedding is None:
            continue
        records.append(
            VectorRecord(
                id=f"{document_id}_chunk_{index}",
                content=chunk.content,
                embedding=chunk.embedding,
                metadata={
                    "document_id": document_id,
                    "chunk_index": str(index),
                    "content_length": str(len(chunk.content)),
                },
            )
        )
    return records


asyncio.run(extract_and_vectorize("document.pdf", "doc_001"))
```
