```python title="Python"
from kreuzberg import ExtractionConfig, TokenReductionConfig, ReductionLevel

config: ExtractionConfig = ExtractionConfig(
    token_reduction=TokenReductionConfig(
        level=ReductionLevel.MODERATE,
        preserve_markdown=True,
        preserve_code=True,
        language_hint="eng",
    )
)
```
