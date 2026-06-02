```python title="Python"
from kreuzberg import ExtractionConfig, PageClassificationConfig, LlmConfig

config = ExtractionConfig(
    page_classification=PageClassificationConfig(
        labels=["invoice", "purchase_order", "delivery_note"],
        multi_label=True,
        llm=LlmConfig(model="openai/gpt-4o-mini"),
    ),
)
```
