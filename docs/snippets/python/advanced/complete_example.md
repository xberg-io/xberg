```python title="Python"
import asyncio
from xberg import ExtractInput, (
    extract,
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    PdfConfig,
    ChunkingConfig,
    EmbeddingConfig,
    EmbeddingModelType,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng+fra",
            tesseract_config=TesseractConfig(psm=3),
        ),
        pdf_options=PdfConfig(extract_images=True),
        chunking=ChunkingConfig(
            max_chars=1000,
            max_overlap=200,
            embedding=EmbeddingConfig(
                model=EmbeddingModelType.preset("all-MiniLM-L6-v2")
            ),
        ),
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Content: {result.results[0].content[:100]}")

asyncio.run(main())
```
