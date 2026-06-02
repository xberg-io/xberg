```python title="Python"
from kreuzberg import (
    ExtractionConfig, RedactionConfig, NerConfig, LlmConfig,
)

config = ExtractionConfig(
    redaction=RedactionConfig(
        categories=["person", "organization", "location", "email"],
        strategy="token_replace",
        ner=NerConfig(
            backend="llm",
            llm=LlmConfig(model="openai/gpt-4o-mini"),
        ),
    ),
)
```
