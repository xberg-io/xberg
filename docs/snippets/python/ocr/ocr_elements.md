```python title="Python"
from xberg import ExtractInput, extract, ExtractionConfig, OcrConfig

config: ExtractionConfig = ExtractionConfig(
    ocr=OcrConfig(backend="paddleocr", language="en")
)

result = extract(ExtractInput.from_uri("scanned.pdf"), config)

if result.results[0].ocr_elements:
    for element in result.results[0].ocr_elements:
        print(f"Text: {element.text}")
        print(f"Confidence: {element.confidence.recognition:.2f}")
        print(f"Geometry: {element.geometry}")
        if element.rotation:
            print(f"Rotation: {element.rotation.angle}°")
        print()
```
