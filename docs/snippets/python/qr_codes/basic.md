```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig

config = ExtractionConfig(qr_codes=True)
result = await extract(ExtractInput.from_uri("ticket.pdf"), config)
for image in result.images or []:
    for qr in image.qr_codes or []:
        print(qr.payload)
```
