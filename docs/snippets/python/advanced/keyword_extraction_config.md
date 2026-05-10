```python title="Python"
from kreuzberg import ExtractionConfig, KeywordConfig, KeywordAlgorithm

config: ExtractionConfig = ExtractionConfig(
    keywords=KeywordConfig(
        algorithm=KeywordAlgorithm.YAKE,
        max_keywords=10,
        min_score=0.3,
        ngram_range=[1, 3],
        language="en",
    )
)
```
