```python title="Python"
from xberg import (
    extract_sync,
    ExtractionConfig,
    OcrConfig,
    TesseractConfig,
    ImagePreprocessingConfig,
)

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        tesseract_config=TesseractConfig(
            preprocessing=ImagePreprocessingConfig(target_dpi=300),
        ),
    ),
)

result = extract_sync("scanned.pdf", config=config)

content_length: int = len(result.content)
table_count: int = len(result.tables)

print(f"Content length: {content_length} characters")
print(f"Tables detected: {table_count}")
```
