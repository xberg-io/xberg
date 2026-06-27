```python title="Python"
import pytest
from xberg import ExtractedDocument

def test_custom_extractor() -> None:
    extractor = CustomJsonExtractor()
    json_data: bytes = b'{"message": "Hello, world!"}'
    config: dict = {}
    result: ExtractedDocument = extractor.extract(
        json_data, "application/json", config
    )
    assert "Hello, world!" in result.content
    assert result.mime_type == "application/json"
```
