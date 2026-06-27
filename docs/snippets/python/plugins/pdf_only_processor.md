```python title="Python"
from xberg import ExtractedDocument, register_post_processor

class PdfOnlyProcessor:
    def name(self) -> str:
        return "pdf-only-processor"

    def version(self) -> str:
        return "1.0.0"

    def process(self, result: ExtractedDocument) -> ExtractedDocument:
        return result

    def should_process(self, result: ExtractedDocument) -> bool:
        return result.mime_type == "application/pdf"

processor: PdfOnlyProcessor = PdfOnlyProcessor()
register_post_processor(processor)
```
