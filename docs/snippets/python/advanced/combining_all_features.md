```python title="Python"
import asyncio
from xberg import (
    extract,
    ExtractionConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    LanguageDetectionConfig,
    TokenReductionConfig,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        enable_quality_processing=True,
        language_detection=LanguageDetectionConfig(enabled=True),
        token_reduction=TokenReductionConfig(mode="moderate"),
        chunking=ChunkingConfig(
            max_chars=512,
            max_overlap=50,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"), normalize=True
            ),
        ),
    )
    result = await extract("document.pdf", config=config)
    quality = result.quality_score or 0
    print(f"Quality: {quality:.2f}")
    print(f"Languages: {result.detected_languages}")
    if result.chunks:
        print(f"Chunks: {len(result.chunks)}")

asyncio.run(main())
```
