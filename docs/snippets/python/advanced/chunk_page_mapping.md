```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(max_characters=500, overlap=50),
)

result = extract_file_sync("document.pdf", config=config)

if result.chunks:
    for chunk in result.chunks:
        first = chunk.metadata.first_page
        last = chunk.metadata.last_page
        if first is None:
            continue
        page_range = f"Page {first}" if first == last else f"Pages {first}-{last}"
        print(f"Chunk: {chunk.content[:50]}... ({page_range})")
```
