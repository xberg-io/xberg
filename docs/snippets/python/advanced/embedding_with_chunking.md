```python title="Python"
from kreuzberg import (
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)

config: ExtractionConfig = ExtractionConfig(
    chunking=ChunkingConfig(
        max_characters=1024,
        overlap=100,
        embedding=EmbeddingConfig(
            model=EmbeddingModelType({"type": "preset", "name": "balanced"}),
            normalize=True,
            batch_size=32,
            show_download_progress=False,
        ),
    )
)
```
