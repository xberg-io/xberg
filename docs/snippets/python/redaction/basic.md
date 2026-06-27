```python title="Python"
import asyncio
from xberg import ExtractInput, extract, ExtractionConfig, RedactionConfig

async def main() -> None:
    config = ExtractionConfig(
        redaction=RedactionConfig(
            categories=["email", "phone", "ssn", "credit_card", "iban"],
            strategy="mask",
        ),
    )
    result = await extract(ExtractInput.from_uri("contract.pdf"), config)
    print(result.results[0].content)
    print(f"Redacted {result.redaction_report.total_redacted} spans")

asyncio.run(main())
```
