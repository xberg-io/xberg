```python title="Python"
from kreuzberg import ExtractionConfig, TranslationConfig, LlmConfig

config = ExtractionConfig(
    translation=TranslationConfig(
        target_lang="de",
        source_lang="en",
        preserve_markup=True,
        llm=LlmConfig(model="openai/gpt-4o"),
    ),
)
```
