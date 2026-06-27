```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, CaptioningConfig, LlmConfig

config = ExtractionConfig(
    captioning=CaptioningConfig(
        llm=LlmConfig(model="openai/gpt-4o-mini"),
    ),
)
result = await extract(ExtractInput.from_uri("report.pdf"), config)
for image in result.images or []:
    if image.caption:
        print(image.caption)
```
