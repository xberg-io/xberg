```python title="Python"
import asyncio
from xberg import ExtractInput, ExtractionConfig, extract

async def main() -> None:
    config = ExtractionConfig(
        output_format="html",
        html_output={
            "theme": "github",
            "embed_css": True,
        },
    )
    result = await extract(ExtractInput.from_uri("document.pdf"), config)
    print(result.results[0].content)  # HTML with kb-* classes and GitHub theme

asyncio.run(main())
```
