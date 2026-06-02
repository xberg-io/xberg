```python title="Python"
from kreuzberg import extract_file, ExtractionConfig

config = ExtractionConfig(qr_codes=True)
result = await extract_file("ticket.pdf", config=config)
for image in result.images or []:
    for qr in image.qr_codes or []:
        print(qr.payload)
```
