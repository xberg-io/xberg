```python title="Python"
import asyncio
from kreuzberg import ExtractionConfig, ImageExtractionConfig, extract_file

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        images=ImageExtractionConfig(
            extract_images=True,
            target_dpi=200,
            max_image_dimension=2048,
            inject_placeholders=True,  # set to False to extract images without markdown references
            auto_adjust_dpi=True,
        )
    )
    result = await extract_file("document.pdf", config=config)
    print(f"Extracted: {result.content[:100]}")

asyncio.run(main())
```
