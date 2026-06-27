```python title="Python"
import asyncio
from xberg import ExtractInput, (
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ImagePreprocessingConfig,
    extract,
)

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            tesseract_config=TesseractConfig(
                preprocessing=ImagePreprocessingConfig(
                    target_dpi=300,
                    denoise=True,
                    deskew=True,
                    contrast_enhance=True,
                    binarization_method="otsu",
                )
            )
        )
    )
    result = await extract(ExtractInput.from_uri("scanned.pdf"), config)
    print(f"Content: {result.results[0].content[:100]}")

asyncio.run(main())
```
