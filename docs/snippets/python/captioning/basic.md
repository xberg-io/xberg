```python title="Python"
from kreuzberg import extract_file, ExtractionConfig, CaptioningConfig, LlmConfig

config = ExtractionConfig(
    captioning=CaptioningConfig(
        llm=LlmConfig(model="openai/gpt-4o-mini"),
    ),
)
result = await extract_file("report.pdf", config=config)
for image in result.images or []:
    if image.caption:
        print(image.caption)
```
