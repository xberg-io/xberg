```python title="Python"
from xberg import extract, ExtractionConfig

config = ExtractionConfig(qr_codes=True)
result = await extract("ticket.pdf", config=config)
for image in result.images or []:
    for qr in image.qr_codes or []:
        print(qr.payload)
```
