```python title="Python"
import asyncio
from kreuzberg import extract_file, ExtractionConfig, RedactionConfig

async def main() -> None:
    config = ExtractionConfig(
        redaction=RedactionConfig(
            categories=["email", "phone", "ssn", "credit_card", "iban"],
            strategy="mask",
        ),
    )
    result = await extract_file("contract.pdf", config=config)
    print(result.content)
    print(f"Redacted {result.redaction_report.total_redacted} spans")

asyncio.run(main())
```
