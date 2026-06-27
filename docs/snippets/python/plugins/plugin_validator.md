```python title="Python"
from xberg import ExtractInput, (, ExtractionConfig
    ExtractedDocument,
    ValidationError,
    extract,
    register_validator,
)

class MinLengthValidator:
    def name(self) -> str:
        return "min_length"

    def version(self) -> str:
        return "1.0.0"

    def validate(self, result: ExtractedDocument) -> None:
        if len(result.results[0].content) < 50:
            raise ValidationError(f"Content too short: {len(result.results[0].content)}")

    def should_validate(self, result: ExtractedDocument) -> bool:
        return True

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

validator: MinLengthValidator = MinLengthValidator()
register_validator(validator)

result = extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())
print(f"Content length: {len(result.results[0].content)}")
```
