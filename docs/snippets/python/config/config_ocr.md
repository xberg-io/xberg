```python title="Python"
import asyncio
from xberg import ExtractionConfig, OcrConfig, TesseractConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract", language="eng+fra",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract("document.pdf", config=config)
    print(result.content)

asyncio.run(main())
```
