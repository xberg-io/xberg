```python title="Python"
import asyncio
from xberg import ExtractionConfig, extract

async def main() -> None:
    config = ExtractionConfig(
        output_format="html",
        html_output={
            "theme": "github",
            "embed_css": True,
        },
    )
    result = await extract("document.pdf", config=config)
    print(result.content)  # HTML with kb-* classes and GitHub theme

asyncio.run(main())
```
