```python title="Python"
from xberg import extract_sync, extract_sync, ExtractionConfig
from xberg import (
    XbergError,
    ParsingError,
    OCRError,
    ValidationError,
)

try:
    result = extract_sync("document.pdf")
    print(f"Extracted {len(result.content)} characters")
except FileNotFoundError as e:
    print(f"File not found: {e}")
except ParsingError as e:
    print(f"Failed to parse document: {e}")
except OCRError as e:
    print(f"OCR processing failed: {e}")
except XbergError as e:
    print(f"Extraction error: {e}")

try:
    config: ExtractionConfig = ExtractionConfig()
    pdf_bytes: bytes = b"%PDF-1.4\n"
    result = extract_sync(pdf_bytes, "application/pdf", config)
    print(f"Extracted: {result.content[:100]}")
except ValidationError as e:
    print(f"Invalid configuration: {e}")
except OCRError as e:
    print(f"OCR failed: {e}")
except XbergError as e:
    print(f"Extraction failed: {e}")
```
