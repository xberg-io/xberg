```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, PageClassificationConfig, LlmConfig

config = ExtractionConfig(
    page_classification=PageClassificationConfig(
        labels=["invoice", "contract", "id_document", "receipt"],
        llm=LlmConfig(model="openai/gpt-4o-mini"),
    ),
)
result = await extract(ExtractInput.from_uri("packet.pdf"), config)
for page in result.page_classifications or []:
    chosen = page.labels[0].label
    print(f"page {page.page_number}: {chosen}")
```
