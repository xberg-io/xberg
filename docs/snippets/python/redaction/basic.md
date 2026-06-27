```python title="Python"
import asyncio
from xberg import extract, ExtractionConfig, RedactionConfig

async def main() -> None:
    config = ExtractionConfig(
        redaction=RedactionConfig(
            categories=["email", "phone", "ssn", "credit_card", "iban"],
            strategy="mask",
        ),
    )
    result = await extract("contract.pdf", config=config)
    print(result.content)
    print(f"Redacted {result.redaction_report.total_redacted} spans")

asyncio.run(main())
```
