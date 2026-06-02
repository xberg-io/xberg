```python title="Python"
from kreuzberg import (
    ExtractionConfig, RedactionConfig, RedactionTerm, RedactionPattern,
)

config = ExtractionConfig(
    redaction=RedactionConfig(
        strategy="token_replace",
        custom_terms=[
            RedactionTerm(label="Project", value="Project Polaris"),
            RedactionTerm(label="Employee", value="EMP-7421", case_sensitive=True),
        ],
        custom_patterns=[
            RedactionPattern(label="InternalId", pattern=r"INT-\d{6}"),
        ],
    ),
)
```
