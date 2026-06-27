```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, OcrConfig, TesseractConfig

async def main() -> None:
    config = ExtractionConfig(
        force_ocr=True,
        ocr=OcrConfig(
            backend="tesseract",
            language="eng",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract("scanned.pdf", config=config)
    print(result.content)
    print(f"Detected Languages: {result.detected_languages}")

asyncio.run(main())
```
