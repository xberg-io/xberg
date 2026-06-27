```python title="Python"
from xberg import (
    extract_sync,
    ExtractionConfig,
    ImagePreprocessingConfig,
    OcrConfig,
    TesseractConfig,
)

preprocessing: ImagePreprocessingConfig = ImagePreprocessingConfig(
    target_dpi=300,
    denoise=True,
    deskew=True,
    contrast_enhance=True,
    binarization_method="otsu",
)

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(
        backend="tesseract",
        language="eng",
        tesseract_config=TesseractConfig(preprocessing=preprocessing),
    )
)

result = extract_sync("document.pdf", config=config)

print(f"Content length: {len(result.content)} characters")
```
