```python title="Python"
from kreuzberg import ExtractionConfig, NerConfig, LlmConfig

config = ExtractionConfig(
    ner=NerConfig(
        backend="llm",
        llm=LlmConfig(model="openai/gpt-4o-mini"),
        custom_labels=["Treatment", "Vessel", "Product"],
    ),
)
```
