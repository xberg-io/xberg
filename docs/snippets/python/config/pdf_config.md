```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, PdfConfig, HierarchyConfig, extract

async def main() -> None:
    config: ExtractionConfig = ExtractionConfig(
        pdf_options=PdfConfig(
            extract_images=True,
            extract_metadata=True,
            passwords=["password1", "password2"],
            hierarchy=HierarchyConfig(enabled=True, k_clusters=6)
        )
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(f"Content: {result.results[0].content[:100]}")

asyncio.run(main())
```
