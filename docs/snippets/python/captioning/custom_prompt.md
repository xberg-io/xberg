```python title="Python"
from kreuzberg import ExtractionConfig, CaptioningConfig, LlmConfig

config = ExtractionConfig(
    captioning=CaptioningConfig(
        llm=LlmConfig(model="openai/gpt-4o-mini"),
        prompt="Describe this figure in one sentence suitable for alt-text.",
        min_image_area=4000,
    ),
)
```
