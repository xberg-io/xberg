```python title="Python"
import asyncio
from xberg import (
    extract,
    ExtractionConfig,
    OcrConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
    LanguageDetectionConfig,
    TokenReductionOptions,
    PostProcessorConfig,
    KeywordConfig,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
        ),
        chunking=ChunkingConfig(
            max_characters=1000,
            overlap=200,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("balanced"),
                batch_size=32,
                normalize=True,
            ),
        ),
        language_detection=LanguageDetectionConfig(
            enabled=True,
            min_confidence=0.8,
            detect_multiple=False,
        ),
        keywords=KeywordConfig(
            algorithm="yake",
            max_keywords=10,
            min_score=0.1,
            ngram_range=[1, 3],
            language="en",
        ),
        token_reduction=TokenReductionOptions(
            mode="moderate",
            preserve_important_words=True,
        ),
        postprocessor=PostProcessorConfig(enabled=True),
    )

    result = await extract("document.pdf", config=config)
    print(f"Content: {result.content[:100]}")
    if result.detected_languages:
        print(f"Languages: {result.detected_languages}")
    if result.chunks:
        print(f"Chunks: {len(result.chunks)}")

asyncio.run(main())
```
