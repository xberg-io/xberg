```python title="Python"
from xberg import extract_sync, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")
)

result = extract_sync("scanned.pdf", config=config)

if result.ocr_elements:
    for element in result.ocr_elements:
        print(f"Text: {element.text}")
        print(f"Confidence: {element.confidence.recognition:.2f}")
        print(f"Geometry: {element.geometry}")
        if element.rotation:
            print(f"Rotation: {element.rotation.angle}°")
        print()
```
