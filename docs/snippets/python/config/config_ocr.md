```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, OcrConfig, TesseractConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            backend="tesseract", language="eng+fra",
            tesseract_config=TesseractConfig(psm=3)
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(result.results[0].content)

asyncio.run(main())
```
