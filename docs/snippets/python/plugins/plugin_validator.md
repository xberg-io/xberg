```python title="Python"
from xberg import (
    ExtractionResult,
    ValidationError,
    extract_sync,
    register_validator,
)

class MinLengthValidator:
    def name(self) -> str:
        return "min_length"

    def version(self) -> str:
        return "1.0.0"

    def validate(self, result: ExtractionResult) -> None:
        if len(result.content) < 50:
            raise ValidationError(f"Content too short: {len(result.content)}")

    def should_validate(self, result: ExtractionResult) -> bool:
        return True

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

validator: MinLengthValidator = MinLengthValidator()
register_validator(validator)

result = extract_sync("document.pdf")
print(f"Content length: {len(result.content)}")
```
