```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, OcrConfig, TesseractConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        ocr=OcrConfig(
            language="eng+fra+deu",
            tesseract_config=TesseractConfig(
                psm=6,
                oem=1,
                min_confidence=0.8,
                enable_table_detection=True,
            ),
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Content: {result.results[0].content[:100]}")

asyncio.run(main())
```
