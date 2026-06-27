```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, ChunkingConfig

config = ExtractionConfig(
    chunking=ChunkingConfig(max_characters=500, overlap=50),
)

result = extract(ExtractInput.from_uri("document.pdf"), config)

if result.results[0].chunks:
    for chunk in result.results[0].chunks:
        first = chunk.metadata.first_page
        last = chunk.metadata.last_page
        if first is None:
            continue
        page_range = f"Page {first}" if first == last else f"Pages {first}-{last}"
        print(f"Chunk: {chunk.content[:50]}... ({page_range})")
```
