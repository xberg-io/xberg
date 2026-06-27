```python title="Python"
from xberg import extract_sync, ExtractionConfig, ChunkingConfig, EmbeddingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(
        max_characters=512,
        overlap=50,
        embedding=EmbeddingConfig(
            normalize=True,
            batch_size=32,
            preset="balanced",
        ),
    ),
)

result = extract_sync("document.pdf", config=config)

records: list[dict] = []
if result.chunks:
    for index, chunk in enumerate(result.chunks):
        if chunk.embedding is None:
            continue
        records.append({
            "id": f"document_chunk_{index}",
            "content": chunk.content,
            "embedding": chunk.embedding,
            "metadata": {
                "document_id": "document.pdf",
                "chunk_index": index,
                "content_length": len(chunk.content),
            },
        })

print(f"Prepared {len(records)} vector records")
```
